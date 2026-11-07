//! Claude adapter: one field projection, guarded native file and shared pending transaction.
use super::{
    live::{patch_json::JsonPatch, write_guard::WriteError},
    projection::fields,
};
use serde_json::{Value, json};
use vw_api_types::app_providers::claude::*;

#[path = "claude_form.rs"]
pub mod form;

fn valid(value: &str, max: usize) -> Result<(), WriteError> {
    let s = value.trim();
    if s.is_empty()
        || value.len() > max
        || value.chars().any(char::is_control)
        || (s.eq_ignore_ascii_case("[redacted]") || s.eq_ignore_ascii_case("<redacted>"))
        || s.chars().all(|c| c == '*' || c == '•')
    {
        return Err(WriteError::InvalidResource);
    }
    Ok(())
}
/// Explicit config variants do not pass through legacy required-key validation.
pub fn settings(profile: &ClaudeProfile) -> Result<Value, WriteError> {
    valid(&profile.name, 128)?;
    let mut env = serde_json::Map::new();
    let mut put = |key: &str, value: &str| -> Result<(), WriteError> {
        valid(value, 8192)?;
        env.insert(key.into(), json!(value));
        Ok(())
    };
    match &profile.config {
        ClaudeConfig::Official { account } => {
            if let Some(binding) = account {
                super::validate_identity(&binding.account_id)
                    .map_err(|_| WriteError::InvalidResource)?;
            }
        }
        ClaudeConfig::ThirdParty { base_url, api_key, api_key_header } => {
            super::validation::endpoint(base_url).map_err(|_| WriteError::InvalidResource)?;
            put("ANTHROPIC_BASE_URL", base_url)?;
            put(
                if *api_key_header { "ANTHROPIC_API_KEY" } else { "ANTHROPIC_AUTH_TOKEN" },
                api_key,
            )?;
        }
        ClaudeConfig::Bedrock { region, auth } => {
            put("CLAUDE_CODE_USE_BEDROCK", "1")?;
            put("AWS_REGION", region)?;
            match auth {
                BedrockAuth::DefaultChain {} => {}
                BedrockAuth::Bearer { token } => put("AWS_BEARER_TOKEN_BEDROCK", token)?,
                BedrockAuth::Profile { name } => put("AWS_PROFILE", name)?,
                BedrockAuth::Keys { access_key_id, secret_access_key, session_token } => {
                    put("AWS_ACCESS_KEY_ID", access_key_id)?;
                    put("AWS_SECRET_ACCESS_KEY", secret_access_key)?;
                    if let Some(token) = session_token {
                        put("AWS_SESSION_TOKEN", token)?;
                    }
                }
            }
        }
        ClaudeConfig::Vertex { project, region, credentials_file } => {
            put("CLAUDE_CODE_USE_VERTEX", "1")?;
            put("ANTHROPIC_VERTEX_PROJECT_ID", project)?;
            put("CLOUD_ML_REGION", region)?;
            if let Some(path) = credentials_file {
                if !std::path::Path::new(path).is_absolute() {
                    return Err(WriteError::InvalidResource);
                }
                put("GOOGLE_APPLICATION_CREDENTIALS", path)?;
            }
        }
    }
    if let Some(model) = &profile.model {
        put("ANTHROPIC_MODEL", model)?;
    }
    Ok(json!({"env":env}))
}

/// Only owned fields change; global values and their spelling remain intact.
pub fn patch(
    pre: Option<&[u8]>,
    previous: Option<&Value>,
    target: &Value,
) -> Result<Vec<u8>, WriteError> {
    patch_with_residue(pre, previous, target, true)
}
pub(crate) fn patch_with_residue(
    pre: Option<&[u8]>,
    previous: Option<&Value>,
    target: &Value,
    clean_legacy: bool,
) -> Result<Vec<u8>, WriteError> {
    let live = super::live::patch_json::object_value(pre.unwrap_or(b"{}"))
        .map_err(|_| WriteError::InvalidResource)?;
    let projected =
        super::projection::claude_with_residue(live.clone(), target, previous, None, clean_legacy)
            .map_err(|_| WriteError::InvalidResource)?
            .settings;
    let mut patch = JsonPatch::default();
    let before = live.as_object().ok_or(WriteError::InvalidResource)?;
    let after = projected.as_object().ok_or(WriteError::InvalidResource)?;
    for key in before.keys().chain(after.keys()) {
        if key == "env" {
            continue;
        }
        if before.get(key) != after.get(key) {
            if let Some(value) = after.get(key) {
                patch.set.push((vec![key.clone()], value.clone()));
            } else {
                patch.remove.push(vec![key.clone()]);
            }
        }
    }
    let empty = serde_json::Map::new();
    let old = before.get("env").and_then(Value::as_object).unwrap_or(&empty);
    let new = after.get("env").and_then(Value::as_object).unwrap_or(&empty);
    for key in old.keys().chain(new.keys()) {
        if old.get(key) != new.get(key) {
            let path = vec!["env".into(), key.clone()];
            if let Some(value) = new.get(key) {
                patch.set.push((path, value.clone()));
            } else {
                patch.remove.push(path);
            }
        }
    }
    patch.apply(pre).map_err(|_| WriteError::InvalidResource)
}

/// Import owned fields from the authorized native file, never return secrets on wire.
/// Unsupported auth/helper/selector shapes fail rather than silently losing fields.
pub fn import(name: &str, bytes: &[u8]) -> Result<ClaudeProfile, WriteError> {
    let value =
        super::live::patch_json::object_value(bytes).map_err(|_| WriteError::InvalidResource)?;
    let empty = serde_json::Map::new();
    let env = value.get("env").and_then(Value::as_object).unwrap_or(&empty);
    let string = |k: &str| env.get(k).and_then(Value::as_str).map(str::to_owned);
    let required = |k: &str| string(k).ok_or(WriteError::InvalidResource);
    let truthy =
        |k: &str| env.get(k).is_some_and(|v| v == "1" || v == "true" || v == true || v == 1);
    let config = if truthy("CLAUDE_CODE_USE_BEDROCK") {
        let auth = if let Some(token) = string("AWS_BEARER_TOKEN_BEDROCK") {
            BedrockAuth::Bearer { token }
        } else if let Some(access_key_id) = string("AWS_ACCESS_KEY_ID") {
            BedrockAuth::Keys {
                access_key_id,
                secret_access_key: required("AWS_SECRET_ACCESS_KEY")?,
                session_token: string("AWS_SESSION_TOKEN"),
            }
        } else if let Some(name) = string("AWS_PROFILE") {
            BedrockAuth::Profile { name }
        } else {
            BedrockAuth::DefaultChain {}
        };
        ClaudeConfig::Bedrock { region: required("AWS_REGION")?, auth }
    } else if truthy("CLAUDE_CODE_USE_VERTEX") {
        ClaudeConfig::Vertex {
            project: required("ANTHROPIC_VERTEX_PROJECT_ID")?,
            region: required("CLOUD_ML_REGION")?,
            credentials_file: string("GOOGLE_APPLICATION_CREDENTIALS"),
        }
    } else if let Some(base_url) = string("ANTHROPIC_BASE_URL") {
        let api_key_header = env.contains_key("ANTHROPIC_API_KEY");
        ClaudeConfig::ThirdParty {
            base_url,
            api_key: required(if api_key_header {
                "ANTHROPIC_API_KEY"
            } else {
                "ANTHROPIC_AUTH_TOKEN"
            })?,
            api_key_header,
        }
    } else {
        ClaudeConfig::Official { account: None }
    };
    let profile = ClaudeProfile { name: name.into(), config, model: string("ANTHROPIC_MODEL") };
    let owned = settings(&profile)?;
    // Secrets not represented by the type must not be silently discarded or reused.
    if !value.is_object()
        || value.get("env").is_some_and(|v| !v.is_object())
        || value.as_object().is_some_and(|m| m.keys().any(|k| fields::claude_floor_top(k)))
        || env.iter().any(|(k, v)| fields::claude_floor_env(k) && owned["env"].get(k) != Some(v))
    {
        return Err(WriteError::InvalidResource);
    }
    Ok(profile)
}

#[cfg(target_os = "macos")]
pub struct ClaudeFile {
    pub(crate) guard: super::live::write_guard::AppWriteGuard,
    pub(crate) name: &'static str,
    pub bytes: Option<Vec<u8>>,
}
#[cfg(target_os = "macos")]
impl ClaudeFile {
    pub fn open(home: &std::path::Path, create: bool) -> Result<Self, WriteError> {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        };
        let home = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(home)
            .map_err(|_| WriteError::InvalidResource)?;
        if create {
            // SAFETY: trusted HOME descriptor, fixed direct child; no client path.
            if unsafe { libc::mkdirat(home.as_raw_fd(), c".claude".as_ptr(), 0o700) } != 0
                && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
            {
                return Err(WriteError::Io);
            }
        }
        // SAFETY: retained HOME descriptor and fixed direct child, result uniquely owned.
        let fd = unsafe {
            libc::openat(
                home.as_raw_fd(),
                c".claude".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(WriteError::InvalidResource);
        }
        let directory = unsafe { std::fs::File::from_raw_fd(fd) };
        let guard = super::live::write_guard::lock_directory(
            vw_api_types::agent_apps::AppId::Claude,
            directory,
        )?;
        let settings = guard.read(std::ffi::OsStr::new("settings.json"))?;
        let (name, bytes) = if settings.is_some() {
            ("settings.json", settings)
        } else if let Some(old) = guard.read(std::ffi::OsStr::new("claude.json"))? {
            ("claude.json", Some(old))
        } else {
            ("settings.json", None)
        };
        Ok(Self { guard, name, bytes })
    }
    #[allow(clippy::too_many_arguments)] // One guarded file operation with its immutable transaction inputs.
    pub fn apply(
        &mut self,
        tx: &super::live::commit::ProviderTransactions,
        tenant: &vw_api_types::id::TenantScope,
        state: &super::live::commit::AppState,
        previous: Option<&Value>,
        target: &Value,
        pointer: Option<String>,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), WriteError> {
        self.apply_with_login(tx, tenant, state, previous, target, pointer, unchanged, None)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply_with_login(
        &mut self,
        tx: &super::live::commit::ProviderTransactions,
        tenant: &vw_api_types::id::TenantScope,
        state: &super::live::commit::AppState,
        previous: Option<&Value>,
        target: &Value,
        pointer: Option<String>,
        unchanged: impl Fn() -> bool,
        login_operation: Option<String>,
    ) -> Result<(), WriteError> {
        use super::live::{
            commit::{Contract, Mode, ModeState, PendingTarget},
            stage::FileChange,
            write_guard::FilePrivacy,
        };
        let previous_contract = state
            .mode
            .as_ref()
            .and_then(|m| m.contract.as_ref())
            .filter(|c| {
                c.version == 1
                    && state.mode_tenant.as_ref() == Some(tenant)
                    && state.pointer.as_ref().is_some_and(|p| p.id == c.key)
            })
            .map(|c| json!({"env":c.exclusive}));
        let previous = previous_contract.as_ref().or(previous);
        let id = pointer
            .as_deref()
            .or_else(|| state.pointer.as_ref().map(|p| p.id.as_str()))
            .ok_or(WriteError::InvalidResource)?;
        let exclusive = target
            .get("env")
            .and_then(Value::as_object)
            .map(|m| {
                m.iter()
                    .filter(|(k, _)| fields::claude_exclusive_env(k))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let mode = ModeState {
            mode: Mode::Direct,
            attached: false,
            proxy_route: state.mode.as_ref().and_then(|m| m.proxy_route.clone()),
            contract: Some(Contract {
                version: 1,
                key: id.into(),
                exclusive,
                claude_login_operation: login_operation,
            }),
        };
        let change = |pre: Option<&[u8]>| {
            if !unchanged() || tx.state(vw_api_types::agent_apps::AppId::Claude)? != *state {
                return Err(WriteError::Conflict);
            }
            patch(pre, previous, target)
        };
        tx.commit(
            tenant,
            &mut self.guard,
            &[FileChange {
                name: std::ffi::OsStr::new(self.name),
                privacy: FilePrivacy::Private,
                patch: &change,
            }],
            PendingTarget { pointer, mode: Some(mode), ..Default::default() },
        )
    }
}
