//! Codex owns rollout compression. Only its native feature flag is managed here.
//! Call from the authorized host executor, under its lifecycle reservation.
use crate::providers::{
    live::{
        commit::{PendingTarget, ProviderTransactions},
        patch_toml::TomlPatch,
        stage::FileChange,
        write_guard::{AppWriteGuard, FilePrivacy, WriteError, lock_directory},
    },
    paths::open_beneath,
};
use std::{
    ffi::{CStr, OsStr},
    fs::File,
    os::fd::{AsRawFd, FromRawFd},
    path::Path,
};
use vw_api_types::{ProjectScope, agent_apps::AppId};

const FEATURE: &str = "local_thread_store_compression";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiskUsage {
    /// Stored file bytes (compressed files retain their compressed size).
    pub active_bytes: u64,
    pub archived_bytes: u64,
}

/// Holds the existing physical configuration lock across read/plan/commit.
/// Inputs are resolved by the trusted application instance owner, never clients.
pub struct CodexCompression {
    directory: File,
    guard: AppWriteGuard,
}
impl CodexCompression {
    pub fn open(execution_home: &Path, config_directory: &Path) -> Result<Self, WriteError> {
        let directory = open_beneath(execution_home, config_directory, true)
            .map_err(|_| WriteError::InvalidResource)?;
        let guard =
            lock_directory(AppId::Codex, directory.try_clone().map_err(|_| WriteError::Io)?)?;
        Ok(Self { directory, guard })
    }

    pub fn is_enabled(&self) -> Result<bool, WriteError> {
        let bytes = self.guard.read(OsStr::new("config.toml"))?;
        let doc = parse(bytes.as_deref())?;
        enabled(&doc)
    }

    /// Disabled means the key is absent, including a prior explicit false/table.
    /// No other native setting or managed provider state is projected here.
    pub fn set_enabled(
        &mut self,
        tx: &ProviderTransactions,
        scope: &ProjectScope,
        requested: bool,
        validate: impl Fn() -> Result<(), WriteError>,
    ) -> Result<bool, WriteError> {
        if tx.recover_project(scope, &mut self.guard)?.is_some() {
            return Err(WriteError::Recovered);
        }
        let state = tx.state(AppId::Codex)?;
        state.require_project(scope)?;
        let patch = |before: Option<&[u8]>| -> Result<Vec<u8>, WriteError> {
            let doc = parse(before)?;
            let current = enabled(&doc)?;
            let present = doc.get("features").and_then(|i| i.get(FEATURE)).is_some();
            // Preserve even table-form true and exact original bytes on no-op.
            if requested && current || !requested && !present {
                return Ok(before.unwrap_or_default().to_vec());
            }
            let path = vec!["features".into(), FEATURE.into()];
            let edit = if requested {
                TomlPatch { set: vec![(path, toml_edit::value(true))], ..Default::default() }
            } else {
                TomlPatch { remove: vec![path], ..Default::default() }
            };
            edit.apply(before).map_err(|_| WriteError::InvalidResource)
        };
        // Avoid creating an empty config or touching the transaction on a no-op.
        let before = self.guard.read(OsStr::new("config.toml"))?;
        let after = patch(before.as_deref())?;
        if before.as_deref().unwrap_or_default() == after {
            validate()?;
            return self.is_enabled();
        }
        tx.commit_project_checked(
            scope,
            &mut self.guard,
            &state,
            false,
            &[FileChange {
                name: OsStr::new("config.toml"),
                privacy: FilePrivacy::Private,
                patch: &patch,
            }],
            PendingTarget::default(),
            validate,
        )?;
        self.is_enabled()
    }

    /// Missing session roots count as zero; unreadable or unsafe trees fail.
    /// All traversal is descriptor-relative and refuses links and special files.
    pub fn disk_usage(&self) -> Result<DiskUsage, WriteError> {
        Ok(DiskUsage {
            active_bytes: root_size(&self.directory, c"sessions")?,
            archived_bytes: root_size(&self.directory, c"archived_sessions")?,
        })
    }
}

fn parse(bytes: Option<&[u8]>) -> Result<toml_edit::DocumentMut, WriteError> {
    let bytes = bytes.unwrap_or_default();
    if bytes.len() > 1024 * 1024 {
        return Err(WriteError::InvalidResource);
    }
    std::str::from_utf8(bytes)
        .map_err(|_| WriteError::InvalidResource)?
        .parse()
        .map_err(|_| WriteError::InvalidResource)
}
fn enabled(doc: &toml_edit::DocumentMut) -> Result<bool, WriteError> {
    let Some(features) = doc.get("features") else { return Ok(false) };
    let features = features.as_table_like().ok_or(WriteError::InvalidResource)?;
    let Some(value) = features.get(FEATURE) else { return Ok(false) };
    if let Some(value) = value.as_bool() {
        return Ok(value);
    }
    value
        .as_table_like()
        .and_then(|table| table.get("enabled"))
        .and_then(|value| value.as_bool())
        .ok_or(WriteError::InvalidResource)
}

fn root_size(parent: &File, name: &CStr) -> Result<u64, WriteError> {
    // SAFETY: parent is a live directory; name is a fixed direct child.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
            Ok(0)
        } else {
            Err(WriteError::Io)
        };
    }
    // SAFETY: openat transferred a fresh descriptor to this owner.
    tree_size(unsafe { File::from_raw_fd(fd) }, 0)
}

fn tree_size(directory: File, depth: usize) -> Result<u64, WriteError> {
    if depth > 64 {
        return Err(WriteError::InvalidResource);
    }
    use std::os::fd::IntoRawFd;
    let fd = directory.into_raw_fd();
    // SAFETY: fd is owned here; fdopendir consumes it only on success.
    let stream = unsafe { libc::fdopendir(fd) };
    if stream.is_null() {
        unsafe {
            libc::close(fd);
        }
        return Err(WriteError::Io);
    }
    struct Directory(*mut libc::DIR);
    impl Drop for Directory {
        fn drop(&mut self) {
            unsafe {
                libc::closedir(self.0);
            }
        }
    }
    let stream = Directory(stream);
    let mut total = 0u64;
    loop {
        // SAFETY: macOS errno is thread-local. readdir's borrowed name is used
        // before the next call; the RAII owner closes the stream on every exit.
        unsafe {
            *libc::__error() = 0;
        }
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            if unsafe { *libc::__error() } != 0 {
                return Err(WriteError::Io);
            }
            return Ok(total);
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if matches!(name.to_bytes(), b"." | b"..") {
            continue;
        }
        let child = unsafe {
            libc::openat(
                fd,
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if child < 0 {
            return Err(WriteError::Io);
        }
        let child = unsafe { File::from_raw_fd(child) };
        let meta = child.metadata().map_err(|_| WriteError::Io)?;
        let size = if meta.is_dir() {
            tree_size(child, depth + 1)?
        } else if meta.is_file() {
            meta.len()
        } else {
            return Err(WriteError::InvalidResource);
        };
        total = total.checked_add(size).ok_or(WriteError::InvalidResource)?;
    }
}
