//! Cross-process credential leases. The persistent empty lock inode must not be unlinked.
use super::credential_file::AccountCredential;
#[cfg(unix)]
use super::credential_file::{
    CredentialKind, PrivateDirectory, ScopedCredentialPath, check_private, child_name,
    credential_bytes, decode_credential, read_from, register_claude_directory,
    remove_credential_file, same_file, write_to,
};
#[cfg(unix)]
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::{ffi::CStr, fs::File, os::fd::AsRawFd, sync::Mutex};
#[cfg(unix)]
use vw_api_types::agent_apps::AppId;
#[cfg(not(target_arch = "wasm32"))]
use vw_api_types::agent_apps_auth::AuthErrorCode;

const CREDENTIAL_GENERATION_CHANGED: &str = "official credential changed before publication";

#[cfg(unix)]
pub struct CredentialLease {
    location: ScopedCredentialPath,
    directory: PrivateDirectory,
    kind: CredentialKind,
    lock: CredentialLock,
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
        let lock = CredentialLock::acquire(&directory, &name)?;
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
        self.lock.check(&directory, &name)
    }

    /// Serialize upstream exchanges and Codex CLI rotation without holding
    /// the credential publication lock across network I/O.
    pub(crate) fn lock_refresh(&self) -> Result<CredentialLock, String> {
        self.check()?;
        let name = child_name(&self.location.path.with_extension("refresh-lock"))?;
        CredentialLock::acquire(&self.directory, &name)
    }

    pub(super) fn read_for_refresh(
        &self,
    ) -> Result<(AccountCredential, RefreshGeneration), String> {
        let lock = self.lock_refresh()?;
        let file = self
            .directory
            .file(&child_name(&self.location.path)?)?
            .ok_or("official account not found")?;
        let bytes = credential_bytes(&file)?;
        let digest = Sha256::digest(&bytes).into();
        let credential = decode_credential(&self.directory, &self.location.path, self.kind, bytes)?;
        Ok((credential, RefreshGeneration { file, digest, lock }))
    }

    pub(super) fn check_refresh_generation(
        &self,
        expected: &RefreshGeneration,
    ) -> Result<(), String> {
        self.check()?;
        let name = child_name(&self.location.path.with_extension("refresh-lock"))?;
        expected.lock.check(&self.directory, &name)?;
        let file = self
            .directory
            .file(&child_name(&self.location.path)?)?
            .ok_or(CREDENTIAL_GENERATION_CHANGED)?;
        // Keep the old descriptor alive across OAuth so unlink and recreation
        // cannot recycle its inode, even if the replacement has identical bytes.
        let digest: [u8; 32] = Sha256::digest(credential_bytes(&file)?).into();
        if !same_file(&file, &expected.file)? || digest != expected.digest {
            return Err(CREDENTIAL_GENERATION_CHANGED.into());
        }
        Ok(())
    }

    pub(super) fn save_refreshed(
        &self,
        expected: &RefreshGeneration,
        next: &AccountCredential,
    ) -> Result<(), String> {
        let _write = self.write.lock().map_err(|_| "official account lock unavailable")?;
        self.check_refresh_generation(expected)?;
        write_to(&self.directory, &self.location.path, self.kind, next, || {
            self.check_refresh_generation(expected)
        })
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

#[cfg(unix)]
pub(crate) struct CredentialLock(File);

#[cfg(unix)]
impl CredentialLock {
    fn acquire(directory: &PrivateDirectory, name: &CStr) -> Result<Self, String> {
        let file = directory
            .open_child(name, libc::O_RDWR | libc::O_CREAT, 0o600)
            .map_err(|_| "official account lock unavailable")?;
        check_private(&file, 0o600)?;
        // SAFETY: file owns the live descriptor. flock does not retain pointers.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("official account is in use".into());
        }
        Ok(Self(file))
    }

    fn check(&self, directory: &PrivateDirectory, name: &CStr) -> Result<(), String> {
        let lock = directory
            .open_child(name, libc::O_RDONLY, 0)
            .map_err(|_| "official account lock unavailable")?;
        check_private(&lock, 0o600)?;
        if !same_file(&lock, &self.0)? {
            return Err("official credential lock changed during lease".into());
        }
        Ok(())
    }
}

#[cfg(unix)]
pub(super) struct RefreshGeneration {
    file: File,
    digest: [u8; 32],
    lock: CredentialLock,
}

#[cfg(not(unix))]
pub(crate) struct CredentialLock;

#[cfg(all(not(unix), not(target_arch = "wasm32")))]
pub(super) struct RefreshGeneration;

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn refresh_publication_lease(
    open: impl Fn() -> Result<CredentialLease, String>,
) -> Result<CredentialLease, AuthErrorCode> {
    use std::time::Duration;
    use tokio::time::{Instant, sleep};

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match open() {
            Err(error) if error == "official account is in use" && Instant::now() < deadline => {
                // Let a short metadata operation finish before deciding whether its
                // publication superseded the generation already exchanged upstream.
                sleep(Duration::from_millis(10)).await;
            }
            result => return result.map_err(refresh_publication_error),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn refresh_publication_error(error: String) -> AuthErrorCode {
    match error.as_str() {
        CREDENTIAL_GENERATION_CHANGED
        | "official account not found"
        | "official account is in use" => AuthErrorCode::Conflict,
        _ => AuthErrorCode::StorageUnavailable,
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

    pub(crate) fn lock_refresh(&self) -> Result<CredentialLock, String> {
        Err("official account locking requires a reviewed host adapter".into())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn read_for_refresh(
        &self,
    ) -> Result<(AccountCredential, RefreshGeneration), String> {
        Err("official account locking requires a reviewed host adapter".into())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn check_refresh_generation(
        &self,
        _expected: &RefreshGeneration,
    ) -> Result<(), String> {
        Err("official account locking requires a reviewed host adapter".into())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn save_refreshed(
        &self,
        _expected: &RefreshGeneration,
        _next: &AccountCredential,
    ) -> Result<(), String> {
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
impl Drop for CredentialLock {
    fn drop(&mut self) {
        // A concurrent fork can retain a CLOEXEC descriptor until exec. Unlock at the
        // owner's lifetime boundary rather than extending the lease until that copy closes.
        loop {
            // SAFETY: the lock descriptor remains live through Drop.
            if unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) } == 0
                || std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted
            {
                break;
            }
        }
    }
}
