//! Host-owned AGY native account selection.

use super::flow::AuthAccountScope;
use std::path::Path;
use vw_api_types::agent_apps_auth::AuthErrorCode;

/// The execution host's trusted policy for the one physical AGY login slot.
pub struct AgyNativeTarget<'a> {
    pub home: &'a Path,
    pub exclusive_owner: Option<AuthAccountScope<'a>>,
}

/// Activate the managed account in the authorized execution user's native AGY slot.
/// The host adapter must resolve the account ID and slot owner from scoped policy.
/// Run this blocking operation off the async runtime.
pub fn switch_default_account(
    scope: AuthAccountScope<'_>,
    account_id: &str,
    native: AgyNativeTarget<'_>,
) -> Result<(), AuthErrorCode> {
    #[cfg(target_os = "macos")]
    {
        let entry = KeychainEntry::new("gemini", "antigravity").map_err(keychain_error)?;
        switch_account_in(scope, account_id, native, &entry)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (scope, account_id, native);
        Err(AuthErrorCode::Unsupported)
    }
}

/// Whether AGY is running and may write its old credentials back after a switch.
pub fn agy_account_is_running() -> bool {
    #[cfg(unix)]
    {
        let Ok(output) = std::process::Command::new("/bin/ps")
            .args(["-axww", "-o", "comm="])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
        else {
            return false;
        };
        output.status.success()
            && String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| Path::new(line.trim()).file_name().is_some_and(|name| name == "agy"))
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[cfg(target_os = "macos")]
use super::keychain::KeychainEntry;
#[cfg(target_os = "macos")]
pub(super) use macos::keychain_error;
#[cfg(target_os = "macos")]
pub(super) use macos::{
    credential_account_id, decode, execution_home, lock_keychain, locked_keychain, managed,
    native_home, rotation_is_newer, save_managed, switch_account_in, validate_account_id,
    validate_scope,
};

#[cfg(target_os = "macos")]
mod macos {
    use super::{AgyNativeTarget, AuthAccountScope, AuthErrorCode, KeychainEntry, Path};
    use crate::{
        auth::{
            agy_login::{KEYCHAIN_ITEM_NOT_FOUND, MANAGED_SERVICE, account_id_for, token_identity},
            copilot::valid_token,
            keychain::EntryError,
        },
        providers::paths::open_beneath,
    };
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use chrono::{DateTime, FixedOffset};
    use security_framework::os::macos::{
        keychain::{SecKeychain, SecPreferencesDomain},
        keychain_item::SecKeychainItem,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use sha2::{Digest, Sha256};
    use std::{
        fs::File,
        os::{fd::AsRawFd, unix::fs::MetadataExt},
    };
    use vw_api_types::agent_apps_auth::{AuthAccount, AuthCredentialState, AuthService};

    const KEYRING_PREFIX: &str = "go-keyring-base64:";

    #[derive(Deserialize, Serialize)]
    pub(in crate::auth) struct ManagedRecord {
        pub(in crate::auth) account: AuthAccount,
        pub(in crate::auth) credential: Value,
    }

    pub(in crate::auth) fn switch_account_in(
        scope: AuthAccountScope<'_>,
        account_id: &str,
        native: AgyNativeTarget<'_>,
        entry: &KeychainEntry<'_>,
    ) -> Result<(), AuthErrorCode> {
        let home = native_home(scope, native)?;
        validate_account_id(account_id)?;
        let (_lock, keychain) = locked_keychain(&home)?;
        let (target, _, _) = managed(&keychain, scope, account_id)?;
        let before = entry.read().map_err(keychain_error)?;
        if let Some(raw) = &before {
            let live = decode(raw).ok_or(AuthErrorCode::Conflict)?;
            let live_id = credential_account_id(scope, &live).ok_or(AuthErrorCode::Conflict)?;
            if live_id == account_id {
                return if same_grant(&live, &target.credential) {
                    Ok(())
                } else {
                    Err(AuthErrorCode::Conflict)
                };
            }
            // An external subject can be shared by different scopes. Preserve a
            // rotation only when the live grant belongs to this managed account.
            let (mut previous, item, _) =
                managed(&keychain, scope, &live_id).map_err(|error| match error {
                    AuthErrorCode::NotFound | AuthErrorCode::ReauthRequired => {
                        AuthErrorCode::Conflict
                    }
                    error => error,
                })?;
            if rotation_is_newer(&live, &previous.credential)? {
                if entry.read().map_err(keychain_error)? != before {
                    return Err(AuthErrorCode::Conflict);
                }
                previous.credential = live;
                save_managed(&keychain, previous, item)?;
            }
        }
        let serialized = serde_json::to_vec(&target.credential)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let encoded = format!("{KEYRING_PREFIX}{}", STANDARD.encode(serialized));
        entry.replace_base64(before.as_deref(), &encoded).map_err(keychain_error)
    }

    pub(in crate::auth) fn validate_scope(
        scope: AuthAccountScope<'_>,
    ) -> Result<(), AuthErrorCode> {
        scope.validate()?;
        if scope.service != AuthService::AgyOauth {
            return Err(AuthErrorCode::Unsupported);
        }
        Ok(())
    }

    pub(in crate::auth) fn validate_account_id(account_id: &str) -> Result<(), AuthErrorCode> {
        if account_id.len() != 43
            || !account_id.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
        {
            return Err(AuthErrorCode::InvalidInput);
        }
        Ok(())
    }

    pub(in crate::auth) fn execution_home() -> Result<std::path::PathBuf, AuthErrorCode> {
        std::env::var_os("HOME")
            .and_then(|path| std::fs::canonicalize(path).ok())
            .ok_or(AuthErrorCode::KeychainUnavailable)
    }

    pub(in crate::auth) fn native_home(
        scope: AuthAccountScope<'_>,
        native: AgyNativeTarget<'_>,
    ) -> Result<std::path::PathBuf, AuthErrorCode> {
        validate_scope(scope)?;
        if native.exclusive_owner != Some(scope) {
            return Err(AuthErrorCode::Forbidden);
        }
        let home = execution_home()?;
        if std::fs::canonicalize(native.home).ok().as_deref() != Some(home.as_path()) {
            return Err(AuthErrorCode::Forbidden);
        }
        Ok(home)
    }

    pub(in crate::auth) fn locked_keychain(
        home: &Path,
    ) -> Result<(File, SecKeychain), AuthErrorCode> {
        let lock = lock_keychain(home)?;
        let keychain = SecKeychain::default_for_domain(SecPreferencesDomain::User)
            .map_err(|_| AuthErrorCode::KeychainUnavailable)?;
        Ok((lock, keychain))
    }

    // Shared by sign-in and switching. All logical scopes using the same local
    // keychain user contend on the same physical directory, even across processes.
    pub(in crate::auth) fn lock_keychain(home: &Path) -> Result<File, AuthErrorCode> {
        let directory = open_beneath(home, &home.join("Library/Keychains"), true)
            .map_err(|_| AuthErrorCode::KeychainUnavailable)?;
        let metadata = directory.metadata().map_err(|_| AuthErrorCode::KeychainUnavailable)?;
        // SAFETY: geteuid has no preconditions. The descriptor remains owned by directory.
        if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o022 != 0 {
            return Err(AuthErrorCode::KeychainUnavailable);
        }
        // SAFETY: flock borrows the open descriptor. Dropping directory releases the lock.
        if unsafe { libc::flock(directory.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(AuthErrorCode::KeychainUnavailable);
        }
        Ok(directory)
    }

    pub(in crate::auth) fn managed(
        keychain: &SecKeychain,
        scope: AuthAccountScope<'_>,
        id: &str,
    ) -> Result<(ManagedRecord, SecKeychainItem, [u8; 32]), AuthErrorCode> {
        let (bytes, item) =
            keychain.find_generic_password(MANAGED_SERVICE, id).map_err(|error| {
                if error.code() == KEYCHAIN_ITEM_NOT_FOUND {
                    AuthErrorCode::NotFound
                } else {
                    AuthErrorCode::KeychainUnavailable
                }
            })?;
        let record: ManagedRecord =
            serde_json::from_slice(&bytes).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        if record.account.account_id != id
            || record.account.service != scope.service
            || credential_account_id(scope, &record.credential).as_deref() != Some(id)
        {
            return Err(AuthErrorCode::NotFound);
        }
        if record.account.credential_state != AuthCredentialState::Ready
            || expiry(&record.credential).is_none()
        {
            return Err(AuthErrorCode::ReauthRequired);
        }
        Ok((record, item, Sha256::digest(&*bytes).into()))
    }

    pub(in crate::auth) fn decode(raw: &[u8]) -> Option<Value> {
        let raw = std::str::from_utf8(raw).ok()?.trim();
        match raw.strip_prefix(KEYRING_PREFIX) {
            Some(encoded) => serde_json::from_slice(&STANDARD.decode(encoded).ok()?).ok(),
            None => serde_json::from_str(raw).ok(),
        }
    }

    pub(in crate::auth) fn credential_account_id(
        scope: AuthAccountScope<'_>,
        value: &Value,
    ) -> Option<String> {
        for field in ["/token/access_token", "/token/refresh_token"] {
            value.pointer(field)?.as_str().filter(|token| valid_token(token))?;
        }
        let identity = token_identity(value["id_token"].as_str()?)?;
        account_id_for(scope, &identity.sub).ok()
    }

    pub(in crate::auth) fn save_managed(
        keychain: &SecKeychain,
        record: ManagedRecord,
        mut item: SecKeychainItem,
    ) -> Result<(), AuthErrorCode> {
        let saved = serde_json::to_vec(&record).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        item.set_password(&saved).map_err(|_| AuthErrorCode::KeychainUnavailable)?;
        let (readback, _) = keychain
            .find_generic_password(MANAGED_SERVICE, &record.account.account_id)
            .map_err(|_| AuthErrorCode::KeychainUnavailable)?;
        if *readback != saved {
            return Err(AuthErrorCode::KeychainUnavailable);
        }
        Ok(())
    }

    pub(in crate::auth) fn rotation_is_newer(
        live: &Value,
        managed: &Value,
    ) -> Result<bool, AuthErrorCode> {
        let live_expiry = expiry(live).ok_or(AuthErrorCode::Conflict)?;
        let managed_expiry = expiry(managed).ok_or(AuthErrorCode::Conflict)?;
        if !same_grant(live, managed)
            || (live_expiry == managed_expiry
                && live.pointer("/token/access_token") != managed.pointer("/token/access_token"))
        {
            return Err(AuthErrorCode::Conflict);
        }
        Ok(live_expiry > managed_expiry)
    }

    fn same_grant(left: &Value, right: &Value) -> bool {
        left.pointer("/token/refresh_token") == right.pointer("/token/refresh_token")
    }

    fn expiry(value: &Value) -> Option<DateTime<FixedOffset>> {
        DateTime::parse_from_rfc3339(value.pointer("/token/expiry")?.as_str()?).ok()
    }

    pub(in crate::auth) fn keychain_error(error: EntryError) -> AuthErrorCode {
        match error {
            EntryError::Conflict => AuthErrorCode::Conflict,
            _ => AuthErrorCode::KeychainUnavailable,
        }
    }
}
