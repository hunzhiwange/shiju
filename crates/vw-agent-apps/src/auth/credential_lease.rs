//! Cross-process credential leases. The persistent empty lock inode must not be unlinked.
use super::credential_file::AccountCredential;
#[cfg(unix)]
use super::credential_file::{
    CredentialKind, PrivateDirectory, ScopedCredentialPath, check_private, child_name, read_from,
    register_claude_directory, remove_credential_file, same_file, write_to,
};
#[cfg(unix)]
use std::{fs::File, os::fd::AsRawFd, sync::Mutex};
#[cfg(unix)]
use vw_api_types::agent_apps::AppId;

pub(super) const CREDENTIAL_GENERATION_CHANGED: &str =
    "official credential changed before publication";

#[cfg(unix)]
pub struct CredentialLease {
    location: ScopedCredentialPath,
    directory: PrivateDirectory,
    kind: CredentialKind,
    lock: File,
    write: Mutex<()>,
}

#[cfg(unix)]
impl CredentialLease {
    pub(super) fn new(
        location: ScopedCredentialPath,
        kind: CredentialKind,
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
        let lease = Self { location, directory, kind, lock, write: Mutex::new(()) };
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
        read_from(&self.directory, &self.location.path, self.kind)
    }

    pub fn save(&self, credential: &AccountCredential) -> Result<(), String> {
        let _write = self.write.lock().map_err(|_| "official account lock unavailable")?;
        self.check()?;
        write_to(&self.directory, &self.location.path, self.kind, credential, || self.check())
    }

    /// Compare the generation used by the caller again at publication, under this lease.
    pub(crate) fn save_codex_generation(
        &self,
        expected: &serde_json::Value,
        next: &AccountCredential,
    ) -> Result<(), String> {
        if !matches!(self.kind, CredentialKind::App(AppId::Codex)) {
            return Err("official credential service mismatch".into());
        }
        let _write = self.write.lock().map_err(|_| "official account lock unavailable")?;
        self.check()?;
        write_to(&self.directory, &self.location.path, self.kind, next, || match self.read()? {
            AccountCredential::CodexAuth(current) if &current == expected => Ok(()),
            _ => Err(CREDENTIAL_GENERATION_CHANGED.into()),
        })
    }

    /// Roll back an unpublished OAuth credential under this account's lease.
    pub fn remove_unpublished_oauth(&self) -> Result<(), String> {
        if !matches!(
            self.kind,
            CredentialKind::Copilot | CredentialKind::Xai | CredentialKind::App(AppId::Codex)
        ) {
            return Err("official credential service mismatch".into());
        }
        let _write = self.write.lock().map_err(|_| "official account lock unavailable")?;
        self.check()?;
        remove_credential_file(&self.directory, &self.location.path)
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

    pub(crate) fn save_codex_generation(
        &self,
        _expected: &serde_json::Value,
        _next: &AccountCredential,
    ) -> Result<(), String> {
        Err("official account locking requires a reviewed host adapter".into())
    }

    pub fn remove_unpublished_oauth(&self) -> Result<(), String> {
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
