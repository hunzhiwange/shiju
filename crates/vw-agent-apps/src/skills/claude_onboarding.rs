//! Explicit first-launch confirmation control, separate from plugin installation
//! and account authorization. Only hasCompletedOnboarding is edited.
use super::claude_status::ClaudeStatusError;
use crate::providers::{
    live::{
        commit::{PendingTarget, ProviderTransactions},
        patch_json::JsonPatch,
        stage::{FileChange, WholeFile},
        write_guard::{FilePrivacy, WriteError, lock_directory},
    },
    paths::open_beneath,
};
use serde_json::Value;
use std::{
    ffi::OsStr,
    fs::File,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::MetadataExt,
    },
    path::Path,
};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

/// HOME and host identify the authorized executor, never client-supplied paths.
/// Default layout uses HOME/.claude.json; a saved override uses its own
/// .claude.json. Returns whether the file changed, without exposing its contents.
pub fn apply_claude_onboarding_skip(
    home: &Path,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    target_host: &str,
) -> Result<bool, ClaudeStatusError> {
    update(home, tx, tenant, target_host, true)
}

/// Removes the field regardless of its previous value; missing is a no-op.
pub fn clear_claude_onboarding_skip(
    home: &Path,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    target_host: &str,
) -> Result<bool, ClaudeStatusError> {
    update(home, tx, tenant, target_host, false)
}

fn update(
    home: &Path,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    target_host: &str,
    enabled: bool,
) -> Result<bool, ClaudeStatusError> {
    if !home.is_absolute() || !tx.is_for_host(target_host) {
        return Err(WriteError::InvalidResource.into());
    }
    let _directory_lock = tx.lock_claude_directory()?;
    let state = tx.state(AppId::Claude)?;
    if state.pointer.as_ref().is_some_and(|p| &p.tenant != tenant)
        || state.mode_tenant.as_ref().is_some_and(|owner| owner != tenant)
        || state.stack_tenant.as_ref().is_some_and(|owner| owner != tenant)
        || state.project_scope.as_ref().is_some_and(|scope| scope.tenant_scope() != tenant)
        || tx.pending(AppId::Claude)?.is_some()
    {
        return Err(WriteError::Conflict.into());
    }
    let selected = tx.claude_directory(tenant)?;
    let directory = match &selected {
        Some(selected) => selected.open(home)?,
        None => open_beneath(home, home, true).map_err(|_| WriteError::InvalidResource)?,
    };
    check_target(&directory)?;
    let mut guard = lock_directory(AppId::Claude, directory)?;
    let name = OsStr::new(".claude.json");
    let before = guard.read(name)?;
    let document = match before.as_deref() {
        Some(bytes) => match serde_json::from_slice::<Value>(bytes) {
            Ok(Value::Object(document)) => document,
            _ => return Err(ClaudeStatusError::InvalidConfig),
        },
        None => serde_json::Map::new(),
    };
    let current = document.get("hasCompletedOnboarding");
    if enabled && current == Some(&Value::Bool(true)) || !enabled && current.is_none() {
        return Ok(false);
    }
    let path = vec!["hasCompletedOnboarding".to_owned()];
    let patch = if enabled {
        JsonPatch { set: vec![(path, Value::Bool(true))], ..Default::default() }
    } else {
        JsonPatch { remove: vec![path], ..Default::default() }
    };
    // Strict JSON validation above; preserve untouched field bytes with the
    // existing native editor and reject concurrent changes with WholeFile.
    let after = patch.apply(before.as_deref()).map_err(|_| ClaudeStatusError::InvalidConfig)?;
    let replacement = WholeFile::new(before.as_deref(), Some(after));
    let directory = guard.retained_directory().try_clone().map_err(|_| WriteError::Io)?;
    tx.commit_checked(
        tenant,
        &mut guard,
        &[FileChange { name, privacy: FilePrivacy::Private, patch: &replacement }],
        PendingTarget { project_scope: state.project_scope.clone(), ..Default::default() },
        || {
            if tx.state(AppId::Claude)? != state {
                return Err(WriteError::Conflict);
            }
            if let Some(selected) = &selected {
                selected.open(home)?;
            }
            check_target(&directory)
        },
    )?;
    Ok(true)
}

fn check_target(directory: &File) -> Result<(), WriteError> {
    let metadata = directory.metadata().map_err(|_| WriteError::Io)?;
    // SAFETY: geteuid only reads the executor identity.
    let uid = unsafe { libc::geteuid() };
    if metadata.uid() != uid || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o300 != 0o300
    {
        return Err(WriteError::InvalidResource);
    }
    // SAFETY: fixed direct child of a retained directory; refuse links and avoid
    // blocking on special files. The descriptor is consumed exactly once below.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(), c".claude.json".as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
            Ok(())
        } else {
            Err(WriteError::InvalidResource)
        };
    }
    // SAFETY: openat returned a fresh owned descriptor.
    let file = unsafe { File::from_raw_fd(fd) };
    let metadata = file.metadata().map_err(|_| WriteError::Io)?;
    if !metadata.is_file() || metadata.uid() != uid
        || metadata.mode() & 0o022 != 0 || metadata.mode() & 0o200 == 0
    {
        return Err(WriteError::InvalidResource);
    }
    Ok(())
}
