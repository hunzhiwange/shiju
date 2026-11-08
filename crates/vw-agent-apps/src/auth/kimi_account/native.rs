//! Kimi CLI's file-backed credential slot and proper-lockfile directory.

use super::write_error;
use crate::{
    auth::credential_file::{PrivateDirectory, check_private, same_file},
    providers::{live::write_guard::prepare_private_child, paths::open_beneath},
};
use serde_json::Value;
use std::{
    ffi::{CString, OsStr},
    fs::File,
    io::{self, Read},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use tokio::task::JoinHandle;
use vw_api_types::agent_apps_auth::AuthErrorCode;

const MAX_BYTES: u64 = 100_000;

pub(super) struct NativeSlot {
    home: PathBuf,
    credentials: PrivateDirectory,
    filename: CString,
    lock: NativeLock,
}

impl NativeSlot {
    pub(super) fn open(home: &Path, slot: &str) -> Result<Self, AuthErrorCode> {
        let credentials = open_directory(home, ".kimi-code/credentials")?;
        let oauth = open_directory(home, ".kimi-code/oauth")?;
        let filename = CString::new(format!("{slot}.json")).expect("validated slot");
        let name = CString::new(format!("{slot}.lock")).expect("validated slot");
        Ok(Self {
            home: home.to_owned(),
            credentials,
            filename,
            lock: NativeLock::acquire(oauth, name)?,
        })
    }

    pub(super) fn read(&self) -> Result<Value, AuthErrorCode> {
        self.lock.check()?;
        let credentials = open_directory(&self.home, ".kimi-code/credentials")?;
        let oauth = open_directory(&self.home, ".kimi-code/oauth")?;
        if !is_same_file(&credentials.0, &self.credentials.0)?
            || !is_same_file(&oauth.0, &self.lock.parent.0)?
        {
            return Err(AuthErrorCode::Conflict);
        }
        let file = self
            .credentials
            .open_child(&self.filename, libc::O_RDONLY | libc::O_NONBLOCK, 0)
            .map_err(|error| {
                if error.kind() == io::ErrorKind::NotFound {
                    AuthErrorCode::ReauthRequired
                } else {
                    AuthErrorCode::StorageUnavailable
                }
            })?;
        check_private(&file, 0o600).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(AuthErrorCode::StorageUnavailable);
        }
        serde_json::from_slice(&bytes).map_err(|_| AuthErrorCode::StorageUnavailable)
    }

    pub(super) fn device_id(&self) -> Result<String, AuthErrorCode> {
        let path = self.home.join(".kimi-code/device_id");
        let file = open_beneath(&self.home, &path, false)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let mut id = String::new();
        file.take(128).read_to_string(&mut id).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let id = id.trim();
        uuid::Uuid::parse_str(id).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        Ok(id.to_owned())
    }

    pub(super) fn replace(&self, before: &Value, after: &Value) -> Result<(), AuthErrorCode> {
        let bytes = serde_json::to_vec(after).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(AuthErrorCode::StorageUnavailable);
        }
        let name = OsStr::from_bytes(self.filename.as_bytes());
        let staged =
            prepare_private_child(&self.credentials.0, name, &bytes).map_err(write_error)?;
        if self.read()? != *before {
            return Err(AuthErrorCode::Conflict);
        }
        staged.commit().map_err(write_error)?;
        self.credentials.0.sync_all().map_err(|_| AuthErrorCode::StorageUnavailable)
    }
}

fn open_directory(home: &Path, relative: &str) -> Result<PrivateDirectory, AuthErrorCode> {
    let dir = open_beneath(home, &home.join(relative), true)
        .map_err(|_| AuthErrorCode::StorageUnavailable)?;
    let meta = dir.metadata().map_err(|_| AuthErrorCode::StorageUnavailable)?;
    // SAFETY: geteuid has no arguments.
    if meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o022 != 0 {
        return Err(AuthErrorCode::StorageUnavailable);
    }
    Ok(PrivateDirectory(dir))
}

fn is_same_file(a: &File, b: &File) -> Result<bool, AuthErrorCode> {
    same_file(a, b).map_err(|_| AuthErrorCode::StorageUnavailable)
}

struct NativeLock {
    parent: PrivateDirectory,
    name: CString,
    dir: File,
    heartbeat: JoinHandle<()>,
}

impl NativeLock {
    fn acquire(parent: PrivateDirectory, name: CString) -> Result<Self, AuthErrorCode> {
        // SAFETY: name is one ASCII child component of the retained directory.
        if unsafe { libc::mkdirat(parent.0.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
            return if io::Error::last_os_error().kind() == io::ErrorKind::AlreadyExists {
                Err(AuthErrorCode::AccountInUse)
            } else {
                Err(AuthErrorCode::StorageUnavailable)
            };
        }

        let opened = (|| {
            let dir = Self::open_lock(&parent, &name)?;
            let clock = dir.try_clone().map_err(|_| AuthErrorCode::StorageUnavailable)?;
            let heartbeat = tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    if clock.set_modified(SystemTime::now()).is_err() {
                        break;
                    }
                }
            });
            Ok((dir, heartbeat))
        })();
        match opened {
            Ok((dir, heartbeat)) => Ok(Self { parent, name, dir, heartbeat }),
            Err(error) => {
                Self::remove(&parent, &name);
                Err(error)
            }
        }
    }

    fn open_lock(parent: &PrivateDirectory, name: &CString) -> Result<File, AuthErrorCode> {
        // SAFETY: parent is a live directory descriptor; name is a single C string component.
        let fd = unsafe {
            libc::openat(
                parent.0.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(AuthErrorCode::Conflict);
        }
        // SAFETY: successful openat returned an owned descriptor.
        Ok(unsafe { File::from_raw_fd(fd) })
    }

    fn check(&self) -> Result<(), AuthErrorCode> {
        if !self.heartbeat.is_finished()
            && is_same_file(&self.dir, &Self::open_lock(&self.parent, &self.name)?)?
        {
            Ok(())
        } else {
            Err(AuthErrorCode::Conflict)
        }
    }

    fn remove(parent: &PrivateDirectory, name: &CString) {
        // SAFETY: removes one directory entry under the retained parent.
        unsafe { libc::unlinkat(parent.0.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) };
    }
}

impl Drop for NativeLock {
    fn drop(&mut self) {
        self.heartbeat.abort();
        if Self::open_lock(&self.parent, &self.name)
            .is_ok_and(|entry| is_same_file(&self.dir, &entry) == Ok(true))
        {
            Self::remove(&self.parent, &self.name);
        }
    }
}
