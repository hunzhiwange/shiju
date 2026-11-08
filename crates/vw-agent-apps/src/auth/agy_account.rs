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
use macos::keychain_error;
#[cfg(target_os = "macos")]
pub(super) use macos::{lock_keychain, switch_account_in};

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
    use std::{
        fs::File,
        os::{fd::AsRawFd, unix::fs::MetadataExt},
    };
    use vw_api_types::agent_apps_auth::{AuthAccount, AuthCredentialState, AuthService};

    const KEYRING_PREFIX: &str = "go-keyring-base64:";

    #[derive(Deserialize, Serialize)]
    struct ManagedRecord {
        account: AuthAccount,
        credential: Value,
    }

    pub(in crate::auth) fn switch_account_in(
        scope: AuthAccountScope<'_>,
        account_id: &str,
        native: AgyNativeTarget<'_>,
        entry: &KeychainEntry<'_>,
    ) -> Result<(), AuthErrorCode> {
        scope.validate()?;
        if scope.service != AuthService::AgyOauth {
            return Err(AuthErrorCode::Unsupported);
        }
        if native.exclusive_owner != Some(scope) {
            return Err(AuthErrorCode::Forbidden);
        }
        let home = std::env::var_os("HOME")
            .and_then(|path| std::fs::canonicalize(path).ok())
            .ok_or(AuthErrorCode::KeychainUnavailable)?;
        if std::fs::canonicalize(native.home).ok().as_deref() != Some(home.as_path()) {
            return Err(AuthErrorCode::Forbidden);
        }
        if account_id.len() != 43
            || !account_id.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
        {
            return Err(AuthErrorCode::InvalidInput);
        }

        let _lock = lock_keychain(&home)?;
        let keychain = SecKeychain::default_for_domain(SecPreferencesDomain::User)
            .map_err(|_| AuthErrorCode::KeychainUnavailable)?;
        let (target, _) = managed(&keychain, scope, account_id)?;
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
            let (mut previous, mut item) =
                managed(&keychain, scope, &live_id).map_err(|error| match error {
                    AuthErrorCode::NotFound | AuthErrorCode::ReauthRequired => {
                        AuthErrorCode::Conflict
                    }
                    error => error,
                })?;
            let live_expiry = expiry(&live).ok_or(AuthErrorCode::Conflict)?;
            let previous_expiry = expiry(&previous.credential).ok_or(AuthErrorCode::Conflict)?;
            if !same_grant(&live, &previous.credential)
                || (live_expiry == previous_expiry
                    && live.pointer("/token/access_token")
                        != previous.credential.pointer("/token/access_token"))
            {
                return Err(AuthErrorCode::Conflict);
            }
            if live_expiry > previous_expiry {
                if entry.read().map_err(keychain_error)? != before {
                    return Err(AuthErrorCode::Conflict);
                }
                previous.credential = live;
                let saved =
                    serde_json::to_vec(&previous).map_err(|_| AuthErrorCode::StorageUnavailable)?;
                item.set_password(&saved).map_err(|_| AuthErrorCode::KeychainUnavailable)?;
                let (readback, _) = keychain
                    .find_generic_password(MANAGED_SERVICE, &live_id)
                    .map_err(|_| AuthErrorCode::KeychainUnavailable)?;
                if *readback != saved {
                    return Err(AuthErrorCode::KeychainUnavailable);
                }
            }
        }
        let serialized = serde_json::to_vec(&target.credential)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let encoded = format!("{KEYRING_PREFIX}{}", STANDARD.encode(serialized));
        entry.replace_base64(before.as_deref(), &encoded).map_err(keychain_error)
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

    fn managed(
        keychain: &SecKeychain,
        scope: AuthAccountScope<'_>,
        id: &str,
    ) -> Result<(ManagedRecord, SecKeychainItem), AuthErrorCode> {
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
        Ok((record, item))
    }

    fn decode(raw: &[u8]) -> Option<Value> {
        let raw = std::str::from_utf8(raw).ok()?.trim();
        match raw.strip_prefix(KEYRING_PREFIX) {
            Some(encoded) => serde_json::from_slice(&STANDARD.decode(encoded).ok()?).ok(),
            None => serde_json::from_str(raw).ok(),
        }
    }

    fn credential_account_id(scope: AuthAccountScope<'_>, value: &Value) -> Option<String> {
        for field in ["/token/access_token", "/token/refresh_token"] {
            value.pointer(field)?.as_str().filter(|token| valid_token(token))?;
        }
        let identity = token_identity(value["id_token"].as_str()?)?;
        account_id_for(scope, &identity.sub).ok()
    }

    fn same_grant(left: &Value, right: &Value) -> bool {
        left.pointer("/token/refresh_token") == right.pointer("/token/refresh_token")
    }

    fn expiry(value: &Value) -> Option<DateTime<FixedOffset>> {
        DateTime::parse_from_rfc3339(value.pointer("/token/expiry")?.as_str()?).ok()
    }

    pub(super) fn keychain_error(error: EntryError) -> AuthErrorCode {
        match error {
            EntryError::Conflict => AuthErrorCode::Conflict,
            _ => AuthErrorCode::KeychainUnavailable,
        }
    }
}
