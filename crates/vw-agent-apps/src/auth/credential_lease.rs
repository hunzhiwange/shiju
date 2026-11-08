//! Cross-process credential leases. The persistent empty lock inode must not be unlinked.
use super::credential_file::AccountCredential;
#[cfg(unix)]
use super::credential_file::{
    PrivateDirectory, ScopedCredentialPath, check_private, child_name, read_from,
    register_claude_directory, same_file, write_to,
};
#[cfg(unix)]
use std::{fs::File, os::fd::AsRawFd, sync::Mutex};
#[cfg(unix)]
use vw_api_types::agent_apps::AppId;

#[cfg(unix)]
pub struct CredentialLease {
    location: ScopedCredentialPath,
    directory: PrivateDirectory,
    app: AppId,
    lock: File,
    write: Mutex<()>,
}

#[cfg(unix)]
impl CredentialLease {
    pub(super) fn new(
        location: ScopedCredentialPath,
        app: AppId,
        create: bool,
    ) -> Result<Self, String> {
        let directory = location.directory(create)?;
        let name = child_name(&location.path.with_extension("session-lock"))?;
        let lock = directory
            .open_child(&name, libc::O_RDWR | libc::O_CREAT, 0o600)
            .map_err(|_| "official account lock unavailable")?;
        check_private(&lock, 0o600)?;
        // SAFETY: lock owns the live descriptor. flock does not retain pointers.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("official account is in use".into());
        }
        let lease = Self { location, directory, app, lock, write: Mutex::new(()) };
        lease.check()?;
        Ok(lease)
    }

    fn check(&self) -> Result<(), String> {
        let directory = self.location.directory(false)?;
        if !same_file(&directory.0, &self.directory.0)? {
            return Err("official credential directory changed during lease".into());
        }
        let name = child_name(&self.location.path.with_extension("session-lock"))?;
        let lock = directory
            .open_child(&name, libc::O_RDONLY, 0)
            .map_err(|_| "official account lock unavailable")?;
        check_private(&lock, 0o600)?;
        if !same_file(&lock, &self.lock)? {
            return Err("official credential lock changed during lease".into());
        }
        Ok(())
    }

    pub fn read(&self) -> Result<AccountCredential, String> {
        self.check()?;
        read_from(&self.directory, &self.location.path, self.app)
    }

    pub fn save(&self, credential: &AccountCredential) -> Result<(), String> {
        let _write = self.write.lock().map_err(|_| "official account lock unavailable")?;
        self.check()?;
        write_to(&self.directory, &self.location.path, self.app, credential, || self.check())
    }

    pub(super) fn register_claude_directory(&self) -> Result<(), String> {
        register_claude_directory(&self.directory, &self.location.path, || self.check())
    }
}

#[cfg(not(unix))]
pub struct CredentialLease {
    _unsupported: (),
}

#[cfg(not(unix))]
impl CredentialLease {
    pub fn read(&self) -> Result<AccountCredential, String> {
        Err("official account locking requires a reviewed host adapter".into())
    }

    pub fn save(&self, _credential: &AccountCredential) -> Result<(), String> {
        Err("official account locking requires a reviewed host adapter".into())
    }

    pub(super) fn register_claude_directory(&self) -> Result<(), String> {
        Err("official account locking requires a reviewed host adapter".into())
    }
}

#[cfg(unix)]
impl Drop for CredentialLease {
    fn drop(&mut self) {
        // A concurrent fork can retain a CLOEXEC descriptor until exec. Unlock at the
        // owner's lifetime boundary rather than extending the lease until that copy closes.
        loop {
            // SAFETY: the lock descriptor remains live through Drop.
            if unsafe { libc::flock(self.lock.as_raw_fd(), libc::LOCK_UN) } == 0
                || std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted
            {
                break;
            }
        }
    }
}
