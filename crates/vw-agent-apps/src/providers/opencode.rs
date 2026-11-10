//! Incremental OpenCode JSONC adapter. All writes use the shared transaction owner.
#[path = "opencode_form.rs"]
pub mod form;

use super::live::{
    patch_json::{JsonPatch, object_value},
    write_guard::WriteError,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, ffi::OsStr, path::Path};
use vw_api_types::{agent_apps::AppId, app_providers::opencode::*};

fn invalid<T>() -> Result<T, WriteError> {
    Err(WriteError::InvalidResource)
}
pub fn validate(profile: &OpenCodeProfile) -> Result<(), WriteError> {
    if profile.name.trim().is_empty()
        || profile.name.len() > 128
        || profile.name.chars().any(char::is_control)
    {
        return invalid();
    }
    let v = &profile.declaration;
    if !v.is_object()
        || serde_json::to_vec(v).map_err(|_| WriteError::InvalidResource)?.len() > 1024 * 1024
    {
        return invalid();
    }
    match profile.format {
        OpenCodeFormat::V2 => {
            if super::opencode_schema::native_provider_problem(v).is_some() {
                return invalid();
            }
        }
        OpenCodeFormat::V1 => {
            for key in ["npm", "name", "api"] {
                if v.get(key).is_some_and(|v| !v.is_string()) {
                    return invalid();
                }
            }
            if v.get("options").is_some_and(|v| !v.is_object()) {
                return invalid();
            }
            if v.get("models")
                .is_some_and(|v| !v.as_object().is_some_and(|m| m.values().all(Value::is_object)))
            {
                return invalid();
            }
        }
    }
    // A declaration is a fragment, never a pasted full config.
    if ["provider", "providers", "$schema"].iter().any(|k| v.get(k).is_some()) {
        return invalid();
    }
    Ok(())
}
fn value(pre: Option<&[u8]>) -> Result<Value, WriteError> {
    let v = object_value(pre.unwrap_or(b"{}")).map_err(|_| WriteError::InvalidResource)?;
    for key in ["provider", "providers"] {
        if v.get(key).is_some_and(|v| !v.is_object()) {
            return invalid();
        }
    }
    Ok(v)
}
/// Read-only declarations, with valid native declarations taking precedence.
pub fn declarations(pre: Option<&[u8]>) -> Result<BTreeMap<String, OpenCodeProfile>, WriteError> {
    let v = value(pre)?;
    let mut rows = BTreeMap::new();
    for format in [OpenCodeFormat::V1, OpenCodeFormat::V2] {
        if let Some(map) = v.get(format.key()).and_then(Value::as_object) {
            for (id, declaration) in map {
                super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
                let profile = OpenCodeProfile {
                    name: declaration
                        .get("name")
                        .and_then(Value::as_str)
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or(id)
                        .into(),
                    format,
                    declaration: declaration.clone(),
                };
                if validate(&profile).is_ok() {
                    rows.insert(id.clone(), profile);
                } else if format == OpenCodeFormat::V1 {
                    return invalid();
                }
            }
        }
    }
    Ok(rows)
}
pub fn state(pre: Option<&[u8]>) -> Result<OpenCodeState, WriteError> {
    let v = value(pre)?;
    let model = v
        .get("model")
        .map(|v| v.as_str().map(str::to_string).ok_or(WriteError::InvalidResource))
        .transpose()?;
    Ok(OpenCodeState { members: declarations(pre)?.into_keys().collect(), model })
}
// Replacing a declaration still edits its individual keys, retaining JSONC
// trivia inside existing objects as well as outside the selected provider.
fn declaration_patch(
    patch: &mut JsonPatch,
    path: Vec<String>,
    previous: Option<&Value>,
    desired: &Value,
) {
    if previous == Some(desired) {
        return;
    }
    if let (Some(previous), Some(desired)) =
        (previous.and_then(Value::as_object), desired.as_object())
    {
        for key in previous.keys().filter(|key| !desired.contains_key(*key)) {
            let mut child = path.clone();
            child.push(key.clone());
            patch.remove.push(child);
        }
        for (key, value) in desired {
            let mut child = path.clone();
            child.push(key.clone());
            declaration_patch(patch, child, previous.get(key), value);
        }
    } else {
        patch.set.push((path, desired.clone()));
    }
}
/// Only the named provider node or top-level model changes; never small_model.
pub fn patch(
    pre: Option<&[u8]>,
    id: &str,
    format: OpenCodeFormat,
    declaration: Option<&Value>,
    model: Option<&str>,
) -> Result<Vec<u8>, WriteError> {
    super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
    let v = value(pre)?;
    let rows = declarations(pre)?;
    let mut patch = JsonPatch::default();
    if let Some(model) = model {
        let Some((provider, model_id)) = model.split_once('/') else { return invalid() };
        if provider != id
            || model_id.is_empty()
            || model.len() > 1024
            || model.chars().any(char::is_control)
            || !rows
                .get(provider)
                .and_then(|r| r.declaration.get("models"))
                .and_then(|m| m.get(model_id))
                .is_some_and(|m| m.is_object() && m.get("disabled") != Some(&Value::Bool(true)))
        {
            return invalid();
        }
        patch.set.push((vec!["model".into()], json!(model)));
    } else if let Some(declaration) = declaration {
        validate(&OpenCodeProfile { name: id.into(), format, declaration: declaration.clone() })?;
        if format == OpenCodeFormat::V1 {
            if rows.get(id).is_some_and(|p| p.format == OpenCodeFormat::V2) {
                return invalid();
            }
            let complete = declaration
                .get("npm")
                .and_then(Value::as_str)
                .is_some_and(|v| !v.trim().is_empty())
                && declaration
                    .get("models")
                    .and_then(Value::as_object)
                    .is_some_and(|m| !m.is_empty());
            if !complete && !rows.contains_key(id) {
                return invalid();
            }
        }
        declaration_patch(
            &mut patch,
            vec![format.key().into(), id.into()],
            v.get(format.key()).and_then(|providers| providers.get(id)),
            declaration,
        );
    } else {
        let legacy = v.get("provider").and_then(|m| m.get(id)).is_some();
        patch.remove.push(vec!["provider".into(), id.into()]);
        if !legacy
            || v.get("providers")
                .and_then(|m| m.get(id))
                .is_some_and(|v| super::opencode_schema::native_provider_problem(v).is_none())
        {
            patch.remove.push(vec!["providers".into(), id.into()]);
        }
    }
    patch.apply(pre).map_err(|_| WriteError::InvalidResource)
}
/// Read without creating directories, locks, files or transaction state.
pub fn read(home: &Path) -> Result<Option<Vec<u8>>, WriteError> {
    for name in ["opencode.jsonc", "opencode.json"] {
        let bytes = super::catalog::read_file(home, &[".config", "opencode", name])
            .map_err(|_| WriteError::InvalidResource)?;
        if bytes.is_some() {
            return Ok(bytes);
        }
    }
    Ok(None)
}
pub struct OpenCodeFiles {
    pub guard: super::live::write_guard::AppWriteGuard,
    directory: std::fs::File,
}
/// Open or provision an already-authorized OpenCode directory beneath the host home.
pub(crate) fn open_config_directory(
    home: &Path,
    directory: &Path,
) -> Result<std::fs::File, WriteError> {
    use std::{
        ffi::CString,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::{ffi::OsStrExt, fs::OpenOptionsExt},
        },
        path::Component,
    };
    if !home.is_absolute() {
        return invalid();
    }
    let relative = directory.strip_prefix(home).map_err(|_| WriteError::InvalidResource)?;
    let names = relative
        .components()
        .map(|component| {
            let Component::Normal(name) = component else {
                return Err(WriteError::InvalidResource);
            };
            CString::new(name.as_bytes()).map_err(|_| WriteError::InvalidResource)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut dir = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(home)
        .map_err(|_| WriteError::InvalidResource)?;
    for name in names {
        // SAFETY: validated single child of the retained authorized directory.
        if unsafe { libc::mkdirat(dir.as_raw_fd(), name.as_ptr(), 0o700) } != 0
            && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(WriteError::Io);
        }
        // SAFETY: validated child, no symlinks, and a newly owned descriptor.
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return invalid();
        }
        dir = unsafe { std::fs::File::from_raw_fd(fd) };
    }
    Ok(dir)
}

impl OpenCodeFiles {
    pub fn open(home: &Path) -> Result<Self, WriteError> {
        let dir = open_config_directory(home, &home.join(".config/opencode"))?;
        let directory = dir.try_clone().map_err(|_| WriteError::Io)?;
        Ok(Self {
            guard: super::live::write_guard::lock_directory(AppId::OpenCode, dir)?,
            directory,
        })
    }
    pub fn snapshot(&self) -> Result<(&'static str, Option<Vec<u8>>), WriteError> {
        if let Some(bytes) = self.guard.read(OsStr::new("opencode.jsonc"))? {
            Ok(("opencode.jsonc", Some(bytes)))
        } else {
            Ok(("opencode.json", self.guard.read(OsStr::new("opencode.json"))?))
        }
    }
    /// Detect a higher-precedence file appearing before a JSON fallback is published.
    pub(crate) fn selected_file_preflight(
        &self,
        name: &str,
    ) -> Result<impl Fn() -> Result<(), WriteError> + use<>, WriteError> {
        let directory = self.directory.try_clone().map_err(|_| WriteError::Io)?;
        let json_fallback = name == "opencode.json";
        Ok(move || {
            if json_fallback {
                use std::os::fd::AsRawFd;
                let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
                // SAFETY: retained directory, fixed child and valid output buffer;
                // existence (including a symlink) changes JSONC path precedence.
                let exists = unsafe {
                    libc::fstatat(
                        directory.as_raw_fd(),
                        c"opencode.jsonc".as_ptr(),
                        metadata.as_mut_ptr(),
                        libc::AT_SYMLINK_NOFOLLOW,
                    )
                };
                if exists == 0
                    || std::io::Error::last_os_error().kind() != std::io::ErrorKind::NotFound
                {
                    return Err(WriteError::Conflict);
                }
            }
            Ok(())
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &mut self,
        tx: &super::live::commit::ProviderTransactions,
        tenant: &vw_api_types::id::TenantScope,
        expected: &super::live::commit::AppState,
        name: &str,
        pre: Option<&[u8]>,
        next: Vec<u8>,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), WriteError> {
        use super::live::{
            commit::*,
            stage::{FileChange, WholeFile},
            write_guard::FilePrivacy,
        };
        if !matches!(name, "opencode.json" | "opencode.jsonc") {
            return invalid();
        }
        if expected.pointer.is_some()
            || expected.mode_tenant.as_ref().is_some_and(|t| t != tenant)
            || expected.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(WriteError::Conflict);
        }
        let after =
            if pre.is_none() && object_value(&next) == Ok(json!({})) { None } else { Some(next) };
        let patch = WholeFile::new(pre, after);
        let check_selection = self.selected_file_preflight(name)?;
        tx.commit_checked(
            tenant,
            &mut self.guard,
            &[FileChange { name: OsStr::new(name), privacy: FilePrivacy::Private, patch: &patch }],
            PendingTarget {
                pointer: None,
                mode: Some(ModeState {
                    mode: Mode::Direct,
                    attached: false,
                    proxy_route: expected.mode.as_ref().and_then(|m| m.proxy_route.clone()),
                    contract: None,
                }),
                ..Default::default()
            },
            || {
                check_selection()?;
                if unchanged() && tx.state(AppId::OpenCode)? == *expected {
                    Ok(())
                } else {
                    Err(WriteError::Conflict)
                }
            },
        )
    }
}
