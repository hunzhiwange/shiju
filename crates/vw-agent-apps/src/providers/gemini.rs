//! Gemini adapter composes existing field projection and two-file pending transaction.
use super::live::{patch_dotenv::DotenvPatch, patch_json::JsonPatch, write_guard::WriteError};
use serde_json::{Value, json};
use vw_api_types::app_providers::gemini::*;
#[path = "gemini_form.rs"]
pub mod form;

fn valid(value: &str, max: usize) -> Result<(), WriteError> {
    let s = value.trim();
    if s.is_empty()
        || value.len() > max
        || value.chars().any(char::is_control)
        || matches!(s, "[redacted]" | "<redacted>")
        || s.chars().all(|c| matches!(c, '*' | '•'))
    {
        return Err(WriteError::InvalidResource);
    }
    Ok(())
}
// Dotenv quotes are not JSON escapes. Single/backtick quotes preserve literal
// backslashes and #; double quotes additionally expand the two newline escapes.
fn dotenv_value(value: &str) -> Result<String, WriteError> {
    for quote in ['\'', '`', '"'] {
        if !value.contains(quote)
            && (quote != '"' || !(value.contains("\\n") || value.contains("\\r")))
        {
            return Ok(format!("{quote}{value}{quote}"));
        }
    }
    Err(WriteError::InvalidResource)
}
pub fn settings(profile: &GeminiProfile) -> Result<Value, WriteError> {
    valid(&profile.name, 128)?;
    let env = match &profile.config {
        GeminiConfig::Official {} => json!({}),
        GeminiConfig::ThirdParty { base_url, api_key } => {
            super::validation::endpoint(base_url).map_err(|_| WriteError::InvalidResource)?;
            valid(api_key, 8192)?;
            json!({"GOOGLE_GEMINI_BASE_URL":base_url,"GEMINI_API_KEY":api_key})
        }
    };
    if let Some(model) = &profile.model {
        valid(model, 256)?;
    }
    Ok(json!({"env":env,"config":{"model":{"name":profile.model}}}))
}
/// Keep the preview projection as the sole owner of the managed-field semantics.
pub fn patches(
    env: Option<&[u8]>,
    config: Option<&[u8]>,
    profile: &GeminiProfile,
) -> Result<(Vec<u8>, Vec<u8>), WriteError> {
    let row = settings(profile)?;
    let text =
        std::str::from_utf8(env.unwrap_or_default()).map_err(|_| WriteError::InvalidResource)?;
    let live = super::live::patch_json::object_value(config.unwrap_or(b"{}"))
        .map_err(|_| WriteError::InvalidResource)?;
    let entries = super::live::patch_dotenv::entries(text)
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect();
    let projected = super::projection::gemini(
        entries,
        live,
        &row,
        matches!(profile.config, GeminiConfig::Official {}),
    )
    .map_err(|_| WriteError::InvalidResource)?
    .settings;
    let dotenv = DotenvPatch {
        clear: Some(super::projection::fields::gemini_floor_env),
        set: projected["env"]
            .as_object()
            .ok_or(WriteError::InvalidResource)?
            .iter()
            .filter(|(k, _)| super::projection::fields::gemini_floor_env(k))
            // Encode semantic strings; dotenv patch accepts raw values.
            .map(|(k, v)| {
                Ok((k.clone(), dotenv_value(v.as_str().ok_or(WriteError::InvalidResource)?)?))
            })
            .collect::<Result<_, WriteError>>()?,
        ..Default::default()
    };
    let mut patch = JsonPatch::default();
    for path in super::projection::fields::GEMINI_FLOOR_SETTINGS {
        let key = path.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let value = path.iter().try_fold(&projected["config"], |v, k| v.get(k));
        match value {
            Some(v) => patch.set.push((key, v.clone())),
            None => patch.remove.push(key),
        }
    }
    Ok((
        dotenv.apply(env).map_err(|_| WriteError::InvalidResource)?,
        patch.apply(config).map_err(|_| WriteError::InvalidResource)?,
    ))
}
#[cfg(target_os = "macos")]
pub struct GeminiFiles {
    pub guard: super::live::write_guard::AppWriteGuard,
}
#[cfg(target_os = "macos")]
impl GeminiFiles {
    pub fn open(home: &std::path::Path) -> Result<Self, WriteError> {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        };
        let home = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(home)
            .map_err(|_| WriteError::InvalidResource)?;
        // SAFETY: fixed child of the administrator-authorized retained HOME descriptor.
        if unsafe { libc::mkdirat(home.as_raw_fd(), c".gemini".as_ptr(), 0o700) } != 0
            && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(WriteError::Io);
        }
        // SAFETY: fixed no-follow child, fresh fd consumed exactly once below.
        let fd = unsafe {
            libc::openat(
                home.as_raw_fd(),
                c".gemini".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(WriteError::InvalidResource);
        }
        let dir = unsafe { std::fs::File::from_raw_fd(fd) };
        Ok(Self {
            guard: super::live::write_guard::lock_directory(
                vw_api_types::agent_apps::AppId::Gemini,
                dir,
            )?,
        })
    }

    pub fn preflight(&self, profile: &GeminiProfile) -> Result<(), WriteError> {
        let env = self.guard.read(std::ffi::OsStr::new(".env"))?;
        let config = self.guard.read(std::ffi::OsStr::new("settings.json"))?;
        patches(env.as_deref(), config.as_deref(), profile)?;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &mut self,
        tx: &super::live::commit::ProviderTransactions,
        tenant: &vw_api_types::id::TenantScope,
        state: &super::live::commit::AppState,
        id: &str,
        profile: &GeminiProfile,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), WriteError> {
        use super::live::{
            commit::*,
            stage::{FileChange, WholeFile},
            write_guard::*,
        };
        use std::ffi::OsStr;
        use vw_api_types::agent_apps::AppId;
        if tx.recover(&mut self.guard)?.is_some() {
            return Err(WriteError::Recovered);
        }
        if state.pointer.as_ref().is_some_and(|p| p.tenant != *tenant)
            || state.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(WriteError::Conflict);
        }
        let env = self.guard.read(OsStr::new(".env"))?;
        let config = self.guard.read(OsStr::new("settings.json"))?;
        let (next_env, next_config) = patches(env.as_deref(), config.as_deref(), profile)?;
        let env_patch = WholeFile::new(env.as_deref(), Some(next_env));
        let config_patch = WholeFile::new(config.as_deref(), Some(next_config));
        tx.commit_checked(
            tenant,
            &mut self.guard,
            &[
                FileChange {
                    name: OsStr::new(".env"),
                    privacy: FilePrivacy::Private,
                    patch: &env_patch,
                },
                FileChange {
                    name: OsStr::new("settings.json"),
                    privacy: FilePrivacy::Private,
                    patch: &config_patch,
                },
            ],
            PendingTarget {
                pointer: Some(id.into()),
                mode: Some(ModeState {
                    mode: Mode::Direct,
                    attached: false,
                    proxy_route: state.mode.as_ref().and_then(|m| m.proxy_route.clone()),
                    contract: None,
                }),
                ..Default::default()
            },
            || {
                if unchanged() && tx.state(AppId::Gemini)? == *state {
                    Ok(())
                } else {
                    Err(WriteError::Conflict)
                }
            },
        )
    }
}

/// Decode existing dotenv/settings without consulting process credentials or login files.
pub fn import(env: &[u8], config: Option<&[u8]>) -> Result<GeminiProfile, WriteError> {
    super::import::reject_placeholders(env)?;
    let text = std::str::from_utf8(env).map_err(|_| WriteError::InvalidResource)?;
    let mut values = std::collections::BTreeMap::new();
    for (key, raw) in super::live::patch_dotenv::entries(text) {
        if !matches!(key.as_str(), "GEMINI_API_KEY" | "GOOGLE_GEMINI_BASE_URL" | "GEMINI_MODEL") {
            continue;
        }
        let value =
            if let Some(quote) = raw.chars().next().filter(|c| matches!(c, '\'' | '"' | '`')) {
                let tail = &raw[quote.len_utf8()..];
                let end = tail.find(quote).ok_or(WriteError::InvalidResource)?;
                let rest = tail[end + quote.len_utf8()..].trim();
                if !rest.is_empty() && !rest.starts_with('#') {
                    return Err(WriteError::InvalidResource);
                }
                let inner = &tail[..end];
                if quote == '"' && inner.contains(['\\', '$']) {
                    return Err(WriteError::InvalidResource);
                }
                inner.to_string()
            } else {
                if raw.contains('$') {
                    return Err(WriteError::InvalidResource);
                }
                raw.split('#').next().unwrap_or_default().trim().to_string()
            };
        values.insert(key, value);
    }
    let doc = super::live::patch_json::object_value(config.unwrap_or(b"{}"))
        .map_err(|_| WriteError::InvalidResource)?;
    let key = values.remove("GEMINI_API_KEY").filter(|s| !s.is_empty());
    let endpoint = values.remove("GOOGLE_GEMINI_BASE_URL").filter(|s| !s.is_empty());
    let config = match (key, endpoint) {
        (None, None) => GeminiConfig::Official {},
        (Some(api_key), base_url) => GeminiConfig::ThirdParty {
            api_key,
            base_url: base_url
                .unwrap_or_else(|| "https://generativelanguage.googleapis.com".into()),
        },
        _ => return Err(WriteError::InvalidResource),
    };
    let native_model = match doc.pointer("/model/name") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        _ => return Err(WriteError::InvalidResource),
    };
    let model = values.remove("GEMINI_MODEL").or(native_model);
    let p = GeminiProfile { name: "default".into(), config, model };
    settings(&p)?;
    Ok(p)
}
