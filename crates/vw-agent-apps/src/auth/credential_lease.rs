//! Cross-process credential leases. The persistent empty lock inode must not be unlinked.
use super::credential_file::{AccountCredential, check_private, read_credential, write_credential};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};
use vw_api_types::agent_apps::AppId;

pub struct CredentialLease {
    path: PathBuf,
    app: AppId,
    _lock: File,
}
impl CredentialLease {
    pub(super) fn new(path: PathBuf, app: AppId) -> Result<Self, String> {
        let lock_path = path.with_extension("session-lock");
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::{fd::AsRawFd, unix::fs::OpenOptionsExt};
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
            let lock = options.open(&lock_path).map_err(|_| "official account lock unavailable")?;
            check_private(&lock_path)?;
            // SAFETY: fd is owned by lock, which outlives the advisory lock. flock does not retain pointers.
            if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                return Err("official account is in use".into());
            }
            Ok(Self { path, app, _lock: lock })
        }
        #[cfg(not(unix))]
        {
            Err("official account locking requires a reviewed host adapter".into())
        }
    }
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
    pub fn read(&self) -> Result<AccountCredential, String> {
        read_credential(&self.path, self.app)
    }
    pub fn save(&self, credential: &AccountCredential) -> Result<(), String> {
        write_credential(&self.path, self.app, credential)
    }
}
