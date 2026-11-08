//! Durable intent recovery. Resource registrations originate only from an
//! executor-authorized guard, never from pending paths or HTTP input.
use super::{
    commit::{ProviderTransactions, apply_target, filename, validate_target},
    stage::digest,
    write_guard::{AppWriteGuard, WriteError, lock_directories, read_from, read_staged},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    ffi::{CString, OsStr},
    fs::File,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Component, Path},
};
use vw_api_types::agent_apps::AppId;

#[derive(Debug, PartialEq, Eq)]
pub enum RecoveryOutcome {
    Discarded,
    RolledForward,
    /// Relative filenames only; callers must authorize before exposing these.
    RolledForwardExcept {
        names: Vec<Vec<u8>>,
    },
}

#[derive(Serialize, Deserialize)]
struct RegisteredDirectory {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    additional: Vec<RegisteredDirectory>,
    host: super::commit::ExecutionHostId,
    app: AppId,
    path: Vec<u8>,
    device: u64,
    inode: u64,
    uid: u32,
}

impl ProviderTransactions {
    /// Called by executor initialization. Each app failure retains its intent;
    /// other registered applications can still recover independently.
    pub fn recover_on_startup(&self) -> Vec<(AppId, Result<RecoveryOutcome, WriteError>)> {
        let mut outcomes = Vec::new();
        for entry in crate::apps::catalog::catalog() {
            let app = entry.metadata.id;
            // Lock order must match commit: physical guard, then host-state flock.
            let result = (|| {
                if self.pending(app)?.is_none() {
                    return Ok(None);
                }
                let registered = self.registered(app)?;
                let directory = open_directory(Path::new(OsStr::from_bytes(&registered.path)))?;
                verify(&registered, &directory)?;
                let mut additional = Vec::new();
                for resource in &registered.additional {
                    let dir = open_directory(Path::new(OsStr::from_bytes(&resource.path)))?;
                    verify(resource, &dir)?;
                    additional.push(dir);
                }
                let mut guard = lock_directories(app, directory, additional, Vec::new())?;
                self.recover(&mut guard)
            })();
            match result {
                Ok(Some(outcome)) => outcomes.push((app, Ok(outcome))),
                Ok(None) => {}
                Err(error) => outcomes.push((app, Err(error))),
            }
        }
        outcomes
    }

    pub fn recover(
        &self,
        guard: &mut AppWriteGuard,
    ) -> Result<Option<RecoveryOutcome>, WriteError> {
        let _lock = self.lock(guard.app())?;
        self.recover_locked(guard)
    }

    pub(super) fn register(&self, guard: &AppWriteGuard) -> Result<(), WriteError> {
        let mut records = Vec::new();
        for directory in guard.directories() {
            let mut path = [0u8; libc::PATH_MAX as usize];
            // SAFETY: retained directory descriptor and writable PATH_MAX buffer.
            if unsafe { libc::fcntl(directory.as_raw_fd(), libc::F_GETPATH, path.as_mut_ptr()) } < 0
            {
                return Err(WriteError::InvalidResource);
            }
            let end = path.iter().position(|b| *b == 0).ok_or(WriteError::InvalidResource)?;
            let metadata = directory.metadata().map_err(|_| WriteError::Io)?;
            let resource = RegisteredDirectory {
                additional: Vec::new(),
                host: self.host.clone(),
                app: guard.app(),
                path: path[..end].to_vec(),
                device: metadata.dev(),
                inode: metadata.ino(),
                uid: metadata.uid(),
            };
            verify(&resource, &open_directory(Path::new(OsStr::from_bytes(&resource.path)))?)?;
            records.push(resource);
        }
        let mut primary = records.remove(0);
        primary.additional = records;
        self.save(guard.app(), "resource", &primary)
    }

    fn registered(&self, app: AppId) -> Result<RegisteredDirectory, WriteError> {
        let bytes = read_from(&self.directory, OsStr::new(&filename(app, "resource")))?
            .ok_or(WriteError::InvalidResource)?;
        let registered: RegisteredDirectory =
            serde_json::from_slice(&bytes).map_err(|_| WriteError::InvalidResource)?;
        if registered.host != self.host
            || registered.app != app
            || registered
                .additional
                .iter()
                .any(|d| d.host != self.host || d.app != app || !d.additional.is_empty())
        {
            return Err(WriteError::InvalidResource);
        }
        Ok(registered)
    }

    pub(super) fn recover_locked(
        &self,
        guard: &mut AppWriteGuard,
    ) -> Result<Option<RecoveryOutcome>, WriteError> {
        let app = guard.app();
        let Some(pending) = self.pending(app)? else {
            return Ok(None);
        };
        let registered = self.registered(app)?;
        verify(&registered, &guard.directory)?;
        if pending.directory_device != registered.device
            || pending.directory_inode != registered.inode
        {
            return Err(WriteError::InvalidResource);
        }
        if registered.additional.len() + 1 != guard.directories().count()
            || (pending.version < 3 && !registered.additional.is_empty())
        {
            return Err(WriteError::InvalidResource);
        }
        for (index, resource) in
            std::iter::once(&registered).chain(&registered.additional).enumerate()
        {
            let directory = guard.directory_at(index)?;
            verify(resource, directory)?;
            if pending.version >= 3 {
                let metadata = directory.metadata().map_err(|_| WriteError::Io)?;
                if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o022 != 0 {
                    return Err(WriteError::InvalidResource);
                }
            }
            verify(resource, &open_directory(Path::new(OsStr::from_bytes(&resource.path)))?)?;
        }
        validate_target(app, &pending.target)?;
        // Validate all records before modifying any file. Targets and temporary
        // names occupy disjoint namespaces; duplicates could corrupt cleanup.
        let mut targets = HashSet::new();
        let mut staged = HashSet::new();
        for file in &pending.files {
            guard.directory_at(file.directory)?;
            child(&file.name)?;
            child(&file.staged)?;
            if file.name.starts_with(b".shiju-live-")
                || !file.staged.starts_with(b".shiju-live-")
                || !targets.insert((file.directory, &file.name))
                || !staged.insert((file.directory, &file.staged))
            {
                return Err(WriteError::InvalidResource);
            }
        }
        let mut skipped = Vec::new();
        if pending.published {
            for file in &pending.files {
                let current =
                    read_from(guard.directory_at(file.directory)?, OsStr::from_bytes(&file.name));
                let current = match current {
                    Ok(bytes) => bytes.as_deref().map(digest),
                    Err(_) => {
                        skipped.push(file.name.clone());
                        continue;
                    }
                };
                if current == file.planned {
                    continue;
                }
                if current != file.pre {
                    skipped.push(file.name.clone());
                    continue;
                }
                let bytes = read_staged(
                    guard.directory_at(file.directory)?,
                    OsStr::from_bytes(&file.staged),
                );
                if !matches!(bytes, Ok(Some(ref bytes)) if file.planned.as_ref().map_or(bytes.is_empty(), |planned| digest(bytes) == *planned))
                {
                    skipped.push(file.name.clone());
                    continue;
                }
                // Preserve the staged file's exact mode and use the original
                // owner of descriptor-relative atomic file publication.
                let write = super::write_guard::PreparedWrite {
                    directory: guard.directory_at(file.directory)?,
                    name: child(&file.name)?,
                    temp: child(&file.staged)?,
                    retain: true,
                    delete: file.planned.is_none(),
                };
                write.publish()?;
                guard.directory_at(file.directory)?.sync_all().map_err(|_| WriteError::Io)?;
            }
            let mut state = self.state(app)?;
            // A Kimi binding is contingent on publishing its native selection.
            if pending.target.kimi_binding.is_none() || skipped.is_empty() {
                apply_target(&mut state, &pending.tenant, &pending.target);
                self.save(app, "state", &state)?;
            }
        }
        for file in &pending.files {
            unlink(guard.directory_at(file.directory)?, &file.staged)?;
        }
        for directory in guard.directories() {
            directory.sync_all().map_err(|_| WriteError::Io)?;
        }
        unlink(&self.directory, filename(app, "pending").as_bytes())?;
        self.directory.sync_all().map_err(|_| WriteError::Io)?;
        Ok(Some(if !pending.published {
            RecoveryOutcome::Discarded
        } else if skipped.is_empty() {
            RecoveryOutcome::RolledForward
        } else {
            RecoveryOutcome::RolledForwardExcept { names: skipped }
        }))
    }
}

fn verify(resource: &RegisteredDirectory, directory: &File) -> Result<(), WriteError> {
    let metadata = directory.metadata().map_err(|_| WriteError::Io)?;
    if !metadata.is_dir()
        || metadata.dev() != resource.device
        || metadata.ino() != resource.inode
        || metadata.uid() != resource.uid
    {
        return Err(WriteError::InvalidResource);
    }
    Ok(())
}
fn child(bytes: &[u8]) -> Result<CString, WriteError> {
    if bytes.is_empty() || bytes == b"." || bytes == b".." || bytes.contains(&b'/') {
        return Err(WriteError::InvalidResource);
    }
    CString::new(bytes).map_err(|_| WriteError::InvalidResource)
}
fn unlink(directory: &File, bytes: &[u8]) -> Result<(), WriteError> {
    let name = child(bytes)?;
    // SAFETY: validated direct child and retained descriptor. Never follows a link.
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0
        && std::io::Error::last_os_error().kind() != std::io::ErrorKind::NotFound
    {
        return Err(WriteError::Io);
    }
    Ok(())
}
/// Absolute physical paths only. Every component is opened without following
/// symlinks; inode identity must additionally match the trusted registration.
fn open_directory(path: &Path) -> Result<File, WriteError> {
    if !path.is_absolute() {
        return Err(WriteError::InvalidResource);
    }
    let mut directory = File::open("/").map_err(|_| WriteError::Io)?;
    for component in path.components() {
        let name = match component {
            Component::RootDir => continue,
            Component::Normal(name) => child(name.as_bytes())?,
            _ => return Err(WriteError::InvalidResource),
        };
        // SAFETY: owned parent descriptor and validated component; owns returned fd.
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(WriteError::InvalidResource);
        }
        // SAFETY: newly owned descriptor from openat.
        directory = unsafe { File::from_raw_fd(fd) };
    }
    Ok(directory)
}
