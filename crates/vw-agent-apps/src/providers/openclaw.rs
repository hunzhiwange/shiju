//! Incremental OpenClaw JSON/JSONC edits; shared transactions own publication.
use super::live::{
    patch_json::{JsonPatch, object_value},
    write_guard::WriteError,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, ffi::OsStr, path::Path};
use vw_api_types::{agent_apps::AppId, app_providers::openclaw::*};
#[path = "openclaw_form.rs"]
pub mod form;

fn invalid<T>() -> Result<T, WriteError> {
    Err(WriteError::InvalidResource)
}
fn model_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 1024 && !value.chars().any(char::is_control)
}
pub fn validate(profile: &OpenClawProfile) -> Result<(), WriteError> {
    let v = &profile.declaration;
    if profile.name.trim().is_empty()
        || profile.name.len() > 128
        || profile.name.chars().any(char::is_control)
        || !v.is_object()
        || serde_json::to_vec(v).map_err(|_| WriteError::InvalidResource)?.len() > 1024 * 1024
        || ["agents", "providers", "env", "tools", "memory"].iter().any(|k| v.get(k).is_some())
    {
        return invalid();
    }
    for key in ["baseUrl", "apiKey", "api"] {
        if v.get(key).is_some_and(|v| !v.is_string()) {
            return invalid();
        }
    }
    if v.get("headers")
        .is_some_and(|v| !v.as_object().is_some_and(|m| m.values().all(Value::is_string)))
    {
        return invalid();
    }
    if let Some(models) = v.get("models") {
        let mut ids = std::collections::HashSet::new();
        for model in models.as_array().ok_or(WriteError::InvalidResource)? {
            let id = model.get("id").and_then(Value::as_str).ok_or(WriteError::InvalidResource)?;
            if !model_id(id) || !ids.insert(id) {
                return invalid();
            }
        }
    }
    Ok(())
}
fn value(pre: Option<&[u8]>) -> Result<Value, WriteError> {
    let v = object_value(pre.unwrap_or(b"{}")).map_err(|_| WriteError::InvalidResource)?;
    for path in
        ["/models", "/models/providers", "/agents", "/agents/defaults", "/agents/defaults/model"]
    {
        if v.pointer(path).is_some_and(|v| !v.is_object()) {
            return invalid();
        }
    }
    if v.pointer("/agents/defaults/model/primary")
        .is_some_and(|v| !v.as_str().is_some_and(model_id))
        || v.pointer("/agents/defaults/model/fallbacks").is_some_and(|v| {
            !v.as_array().is_some_and(|v| v.iter().all(|v| v.as_str().is_some_and(model_id)))
        })
    {
        return invalid();
    }
    Ok(v)
}
pub fn declarations(pre: Option<&[u8]>) -> Result<BTreeMap<String, OpenClawProfile>, WriteError> {
    let v = value(pre)?;
    let mut rows = BTreeMap::new();
    if let Some(map) = v.pointer("/models/providers").and_then(Value::as_object) {
        for (id, declaration) in map {
            super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
            let profile = OpenClawProfile { name: id.clone(), declaration: declaration.clone() };
            validate(&profile)?;
            rows.insert(id.clone(), profile);
        }
    }
    Ok(rows)
}
pub fn state(pre: Option<&[u8]>) -> Result<OpenClawState, WriteError> {
    let v = value(pre)?;
    Ok(OpenClawState {
        members: declarations(pre)?.into_keys().collect(),
        model: v
            .pointer("/agents/defaults/model/primary")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}
// Keep comments within an edited declaration by applying only differing leaves.
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
/// Only one membership node or primary/fallback leaves can change.
pub fn patch(
    pre: Option<&[u8]>,
    id: &str,
    declaration: Option<&Value>,
    model: Option<&str>,
) -> Result<Vec<u8>, WriteError> {
    super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
    let v = value(pre)?;
    let rows = declarations(pre)?;
    let mut patch = JsonPatch::default();
    if let Some(model) = model {
        let Some((provider, model_key)) = model.split_once('/') else {
            return invalid();
        };
        if provider != id
            || !model_id(model)
            || model_key.is_empty()
            || !rows
                .get(id)
                .and_then(|p| p.declaration.get("models"))
                .and_then(Value::as_array)
                .is_some_and(|models| {
                    models.iter().any(|m| m.get("id").and_then(Value::as_str) == Some(model_key))
                })
        {
            return invalid();
        }
        patch.set.push((
            vec!["agents".into(), "defaults".into(), "model".into(), "primary".into()],
            json!(model),
        ));
        if let Some(fallbacks) =
            v.pointer("/agents/defaults/model/fallbacks").and_then(Value::as_array)
        {
            patch.set.push((
                vec!["agents".into(), "defaults".into(), "model".into(), "fallbacks".into()],
                Value::Array(
                    fallbacks.iter().filter(|v| v.as_str() != Some(model)).cloned().collect(),
                ),
            ));
        }
    } else if let Some(declaration) = declaration {
        validate(&OpenClawProfile { name: id.into(), declaration: declaration.clone() })?;
        declaration_patch(
            &mut patch,
            vec!["models".into(), "providers".into(), id.into()],
            v.pointer("/models/providers").and_then(|m| m.get(id)),
            declaration,
        );
        if v.get("models").is_none() {
            patch.seed.push((vec!["models".into(), "mode".into()], json!("merge")));
        }
    } else {
        patch.remove.push(vec!["models".into(), "providers".into(), id.into()]);
    }
    patch.apply(pre).map_err(|_| WriteError::InvalidResource)
}
/// Read-only, fixed authorized HOME; JSON5 syntax is explicitly rejected by the parser.
pub fn read(home: &Path) -> Result<Option<Vec<u8>>, WriteError> {
    super::catalog::read_file(home, &[".openclaw", "openclaw.json"])
        .map_err(|_| WriteError::InvalidResource)
}
pub struct OpenClawFiles {
    pub guard: super::live::write_guard::AppWriteGuard,
}
impl OpenClawFiles {
    pub fn open(home: &Path) -> Result<Self, WriteError> {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        };
        if !home.is_absolute() {
            return invalid();
        }
        let mut dir = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(home)
            .map_err(|_| WriteError::InvalidResource)?;
        {
            let name = c".openclaw";
            // SAFETY: fixed single child of the retained authorized directory.
            if unsafe { libc::mkdirat(dir.as_raw_fd(), name.as_ptr(), 0o700) } != 0
                && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
            {
                return Err(WriteError::Io);
            }
            // SAFETY: fixed child; no symlinks; returned fd consumed once.
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
        Ok(Self { guard: super::live::write_guard::lock_directory(AppId::OpenClaw, dir)? })
    }
    pub fn snapshot(&self) -> Result<(&'static str, Option<Vec<u8>>), WriteError> {
        Ok(("openclaw.json", self.guard.read(OsStr::new("openclaw.json"))?))
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
        if name != "openclaw.json" {
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
                if unchanged() && tx.state(AppId::OpenClaw)? == *expected {
                    Ok(())
                } else {
                    Err(WriteError::Conflict)
                }
            },
        )
    }
}
