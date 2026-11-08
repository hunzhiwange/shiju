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
        self.remove_oauth_file()
    }

    pub(super) fn remove_oauth_file(&self) -> Result<(), String> {
        if !matches!(
            self.kind,
            CredentialKind::Copilot
                | CredentialKind::Xai
                | CredentialKind::Kimi
                | CredentialKind::App(AppId::Codex)
        ) {
            return Err("official credential service mismatch".into());
        }
        let _write = self.write.lock().map_err(|_| "official account lock unavailable")?;
        self.check()?;
        remove_credential_file(&self.directory, &self.location.path)
    }

    #[cfg(target_os = "macos")]
    pub(super) fn remove_claude(
        &self,
        keychain: &dyn super::claude_keychain::ClaudeKeychain,
        identity_sha256: [u8; 32],
    ) -> Result<(), vw_api_types::agent_apps_auth::AuthErrorCode> {
        use super::credential_file::{managed_config_path, remove_account_directory};
        use crate::providers::{
            claude_login::{LoginMaterial, lock_account},
            live::write_guard::WriteError,
        };
        use sha2::{Digest, Sha256};
        use std::os::unix::fs::MetadataExt;
        use vw_api_types::agent_apps_auth::AuthErrorCode;
        let storage = |_| AuthErrorCode::StorageUnavailable;
        if !matches!(self.kind, CredentialKind::App(AppId::Claude)) {
            return Err(AuthErrorCode::Unsupported);
        }
        let _write = self.write.lock().map_err(|_| AuthErrorCode::StorageUnavailable)?;
        self.check().map_err(storage)?;
        let path = managed_config_path(&self.location.path);
        let name = child_name(&path).map_err(storage)?;
        let directory =
            match self.directory.open_child(&name, libc::O_RDONLY | libc::O_DIRECTORY, 0) {
                Ok(directory) => {
                    check_private(&directory, 0o700).map_err(storage)?;
                    if directory.metadata().map_err(|_| AuthErrorCode::StorageUnavailable)?.dev()
                        != self
                            .directory
                            .0
                            .metadata()
                            .map_err(|_| AuthErrorCode::StorageUnavailable)?
                            .dev()
                    {
                        return Err(AuthErrorCode::Conflict);
                    }
                    let (locked, locks) = lock_account(&path).map_err(|error| match error {
                        WriteError::Conflict => AuthErrorCode::AccountInUse,
                        _ => AuthErrorCode::StorageUnavailable,
                    })?;
                    if !same_file(&directory, &locked.0).map_err(storage)? {
                        return Err(AuthErrorCode::Conflict);
                    }
                    Some((directory, locks))
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(_) => return Err(AuthErrorCode::StorageUnavailable),
            };
        self.check().map_err(storage)?;
        let read_keychain =
            || keychain.read(Some(&path)).map_err(|_| AuthErrorCode::KeychainUnavailable);
        if let Some(previous) = read_keychain()? {
            if directory.is_none() {
                return Err(AuthErrorCode::Conflict);
            }
            let identity = LoginMaterial::from_account(&path, keychain)
                .and_then(|material| material.identity())
                .map_err(|_| AuthErrorCode::IdentityMismatch)?
                .ok_or(AuthErrorCode::IdentityMismatch)?;
            let actual: [u8; 32] = Sha256::digest(identity.as_bytes()).into();
            if actual != identity_sha256 {
                return Err(AuthErrorCode::IdentityMismatch);
            }
            if read_keychain()?.as_ref() != Some(&previous) {
                return Err(AuthErrorCode::Conflict);
            }
            keychain.write(Some(&path), None).map_err(|_| AuthErrorCode::KeychainUnavailable)?;
            if read_keychain()?.is_some() {
                return Err(AuthErrorCode::KeychainUnavailable);
            }
        }
        if let Some((directory, _locks)) = directory {
            remove_account_directory(&self.directory, &directory, &path).map_err(storage)?;
        }
        remove_credential_file(&self.directory, &self.location.path).map_err(storage)
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
        self.remove_oauth_file()
    }

    pub(super) fn remove_oauth_file(&self) -> Result<(), String> {
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
