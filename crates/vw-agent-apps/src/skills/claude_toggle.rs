//! Fixed Claude config.json marker edits on the authorized execution host.
//! Unlike the upstream fallback, malformed JSON is never reset and clearing a
//! user-owned primaryApiKey conflicts instead of deleting the credential.
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
use vw_api_types::{agent_apps::AppId, app_providers::paths::ConfigPathState, id::TenantScope};

/// Returns whether config.json changed. HOME and host must come from trusted
/// executor dispatch. This marker does not install or authorize any plugin.
pub fn apply_claude_plugin_config(
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
    let directory = match tx.claude_directory(tenant)? {
        Some(selected) => selected.open(home)?,
        None => match open_beneath(home, &home.join(".claude"), true) {
            Ok(directory) => directory,
            Err(ConfigPathState::Absent) if !enabled => return Ok(false),
            Err(ConfigPathState::Absent) => create_default_directory(home)?,
            Err(_) => return Err(WriteError::InvalidResource.into()),
        },
    };
    check_directory(&directory)?;
    let mut guard = lock_directory(AppId::Claude, directory)?;
    check_config_permissions(guard.retained_directory())?;
    let name = OsStr::new("config.json");
    let before = guard.read(name)?;
    let document = match before.as_deref() {
        Some(bytes) => match serde_json::from_slice::<Value>(bytes) {
            Ok(Value::Object(document)) => document,
            _ => return Err(ClaudeStatusError::InvalidConfig),
        },
        None => serde_json::Map::new(),
    };
    let current = document.get("primaryApiKey");
    let managed = current.and_then(Value::as_str) == Some("any");
    if (enabled && managed) || (!enabled && current.is_none()) {
        return Ok(false);
    }
    if !enabled && !managed {
        return Err(WriteError::Conflict.into());
    }
    let path = vec!["primaryApiKey".to_owned()];
    let patch = if enabled {
        JsonPatch { set: vec![(path, Value::String("any".to_owned()))], ..Default::default() }
    } else {
        JsonPatch { remove: vec![path], ..Default::default() }
    };
    // Strict JSON above; the shared editor preserves untouched field bytes.
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
            check_directory(&directory)?;
            check_config_permissions(&directory)
        },
    )?;
    Ok(true)
}

fn check_directory(directory: &File) -> Result<(), WriteError> {
    let metadata = directory.metadata().map_err(|_| WriteError::Io)?;
    // SAFETY: geteuid only reads the executor identity.
    if metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o300 != 0o300
    {
        return Err(WriteError::InvalidResource);
    }
    Ok(())
}

fn create_default_directory(home: &Path) -> Result<File, WriteError> {
    let parent = open_beneath(home, home, true).map_err(|_| WriteError::InvalidResource)?;
    check_directory(&parent)?;
    let name = c".claude";
    // SAFETY: retained HOME descriptor and fixed direct child; never traverse links.
    if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0
        && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
    {
        return Err(WriteError::Io);
    }
    let directory = open_child_directory(&parent)?;
    check_directory(&directory)?;
    parent.sync_all().map_err(|_| WriteError::Io)?;
    Ok(directory)
}

fn open_child_directory(parent: &File) -> Result<File, WriteError> {
    // SAFETY: fixed direct child and owned parent descriptor; consume fresh fd once.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            c".claude".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(WriteError::InvalidResource);
    }
    // SAFETY: openat returned a fresh owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn check_config_permissions(directory: &File) -> Result<(), WriteError> {
    // SAFETY: fixed child of a retained directory. NONBLOCK refuses hanging on FIFOs.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            c"config.json".as_ptr(),
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
    // SAFETY: geteuid only reads the executor identity.
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o200 == 0
    {
        return Err(WriteError::InvalidResource);
    }
    Ok(())
}
