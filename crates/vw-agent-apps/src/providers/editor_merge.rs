//! Claude editor changes. Baselines are executor-only; shared projection owns field classification.
use super::live::patch_json::JsonPatch;
use serde_json::Value;
use vw_api_types::app_providers::editor_merge::*;
#[derive(Debug, PartialEq, Eq)]
pub enum MergeError {
    Invalid,
    Conflict(Vec<Vec<String>>),
}
pub fn merge(
    base: &Value,
    current: &[u8],
    edits: &[EditorEdit],
    choice: EditorChoice,
    global: bool,
) -> Result<Vec<u8>, MergeError> {
    let now: Value = serde_json::from_slice(current).map_err(|_| MergeError::Invalid)?;
    if !base.is_object() || !now.is_object() || edits.len() > 128 {
        return Err(MergeError::Invalid);
    }
    let mut patch = JsonPatch::default();
    let mut conflicts = vec![];
    for (index, edit) in edits.iter().enumerate() {
        let path = &edit.path;
        if path.is_empty()
            || path.len() > 64
            || path.iter().any(|k| k.is_empty() || k.len() > 256 || k.chars().any(char::is_control))
            || edits[..index].iter().any(|e| e.path.starts_with(path) || path.starts_with(&e.path))
        {
            return Err(MergeError::Invalid);
        }
        if global {
            use super::projection::fields;
            let root = path[0].as_str();
            if fields::claude_floor_top(root)
                || ["api_format", "apiFormat", "openrouter_compat_mode", "openrouterCompatMode"]
                    .contains(&root)
                || root == "env"
                    && (path.len() < 2
                        || fields::claude_floor_env(&path[1])
                        || fields::claude_exclusive_env(&path[1]))
            {
                return Err(MergeError::Invalid);
            }
        }
        let after = match &edit.value {
            EditorValue::Set { value } => {
                if placeholder(value) {
                    return Err(MergeError::Invalid);
                }
                Some(value)
            }
            EditorValue::Remove => None,
        };
        let before = at(base, path);
        // Sending an unchanged display value cannot revert an external edit.
        if before == after {
            continue;
        }
        let current_value = at(&now, path);
        if current_value != before && current_value != after {
            conflicts.push(path.clone());
            if choice == EditorChoice::KeepTheirs {
                continue;
            }
        }
        match after {
            Some(value) => patch.set.push((path.clone(), value.clone())),
            None => patch.remove.push(path.clone()),
        }
    }
    if !conflicts.is_empty() && choice == EditorChoice::Refuse {
        return Err(MergeError::Conflict(conflicts));
    }
    let result = patch.apply(Some(current)).map_err(|_| MergeError::Invalid)?;
    let value: Value = serde_json::from_slice(&result).map_err(|_| MergeError::Invalid)?;
    if value.get("env").is_some_and(|env| !env.is_object()) {
        return Err(MergeError::Invalid);
    }
    Ok(result)
}

#[cfg(target_os = "macos")]
pub struct ClaudeFile {
    guard: super::live::write_guard::AppWriteGuard,
    name: &'static str,
    pub bytes: Vec<u8>,
    pub identity: (u64, u64),
}
#[cfg(target_os = "macos")]
impl ClaudeFile {
    pub fn open(home: &std::path::Path) -> Result<Self, super::live::write_guard::WriteError> {
        Self::open_selected(home, None)
    }

    pub fn open_selected(
        home: &std::path::Path,
        selected: Option<&super::live::claude_directory::ClaudeDirectory>,
    ) -> Result<Self, super::live::write_guard::WriteError> {
        use super::live::write_guard::WriteError;
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::{MetadataExt, OpenOptionsExt},
        };
        let directory = if let Some(selected) = selected {
            selected.open(home)?
        } else {
            let home = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(home)
                .map_err(|_| WriteError::InvalidResource)?;
            // SAFETY: fixed child and retained trusted HOME descriptor; fresh descriptor consumed once.
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
            unsafe { std::fs::File::from_raw_fd(fd) }
        };
        let meta = directory.metadata().map_err(|_| WriteError::Io)?;
        let guard = super::live::write_guard::lock_directory(
            vw_api_types::agent_apps::AppId::Claude,
            directory,
        )?;
        let (name, bytes) = match guard.read(std::ffi::OsStr::new("settings.json"))? {
            Some(bytes) => ("settings.json", bytes),
            None if selected.is_some() => (
                "claude.json",
                guard
                    .read(std::ffi::OsStr::new("claude.json"))?
                    .ok_or(WriteError::InvalidResource)?,
            ),
            None => return Err(WriteError::InvalidResource),
        };
        Ok(Self { guard, name, bytes, identity: (meta.dev(), meta.ino()) })
    }
    pub fn commit(
        &mut self,
        transactions: &super::live::commit::ProviderTransactions,
        tenant: &vw_api_types::id::TenantScope,
        state: &super::live::commit::AppState,
        base: &Value,
        edits: &[EditorEdit],
        choice: EditorChoice,
    ) -> Result<(), super::live::write_guard::WriteError> {
        use super::live::{
            stage::FileChange,
            write_guard::{FilePrivacy, WriteError},
        };
        let expected = &self.bytes;
        let patch = |pre: Option<&[u8]>| {
            if transactions.state(vw_api_types::agent_apps::AppId::Claude)? != *state
                || pre != Some(expected.as_slice())
            {
                return Err(WriteError::Conflict);
            }
            merge(base, pre.ok_or(WriteError::InvalidResource)?, edits, choice, true)
                .map_err(|_| WriteError::Conflict)
        };
        transactions.commit(
            tenant,
            &mut self.guard,
            &[FileChange {
                name: std::ffi::OsStr::new(self.name),
                privacy: FilePrivacy::Private,
                patch: &patch,
            }],
            Default::default(),
        )
    }
}
/// Same projection as preview; unsupported ownership shapes remain explicit failures.
pub fn project(live: &[u8], row: &Value) -> Result<Value, MergeError> {
    let live = serde_json::from_slice(live).map_err(|_| MergeError::Invalid)?;
    super::projection::claude(live, row, Some(row), None)
        .map(|p| p.settings)
        .map_err(|_| MergeError::Invalid)
}
pub fn display(value: Value) -> Value {
    super::preview::redact(value)
}

fn at<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.as_object()?.get(key)?;
    }
    Some(current)
}
fn placeholder(value: &Value) -> bool {
    match value {
        Value::String(s) => matches!(s.trim(), "[redacted]" | "<redacted>"),
        Value::Array(a) => a.iter().any(placeholder),
        Value::Object(m) => m.values().any(placeholder),
        _ => false,
    }
}
