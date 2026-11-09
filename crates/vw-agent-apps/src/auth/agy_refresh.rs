//! Scoped AGY token refresh and native credential reconciliation.

use super::{
    agy_account::{
        AgyNativeTarget, credential_account_id, decode, execution_home, keychain_error,
        locked_keychain, managed, native_home, rotation_is_newer, save_managed,
        validate_account_id, validate_scope,
    },
    agy_login::{post_token, token_client, token_expiry, token_identity},
    copilot::{utc_now, valid_token},
    flow::AuthAccountScope,
    keychain::KeychainEntry,
};
use reqwest::{Client, Request};
use security_framework::os::macos::keychain::SecKeychain;
use serde_json::{Value, json};
use std::{fs::File, path::Path};
use vw_api_types::agent_apps_auth::{AuthCredentialState, AuthErrorCode};

/// Refresh a managed AGY account on its authorized macOS execution host.
/// The returned access token is private host material.
pub async fn refresh_account(
    scope: AuthAccountScope<'_>,
    account_id: &str,
) -> Result<String, AuthErrorCode> {
    refresh_with(scope, account_id, &token_client()?, |_| {}).await
}

pub(super) async fn refresh_with(
    scope: AuthAccountScope<'_>,
    account_id: &str,
    client: &Client,
    route: impl FnOnce(&mut Request),
) -> Result<String, AuthErrorCode> {
    validate_scope(scope)?;
    validate_account_id(account_id)?;
    let home = execution_home()?;
    let (refresh_token, version) = {
        let (_lock, keychain) = lock_for_refresh(&home).await?;
        let (record, _, version) = managed(&keychain, scope, account_id)?;
        let refresh_token = record
            .credential
            .pointer("/token/refresh_token")
            .and_then(Value::as_str)
            .ok_or(AuthErrorCode::ReauthRequired)?;
        (refresh_token.to_owned(), version)
    };

    let outcome = match post_token(
        client,
        &[("grant_type", "refresh_token"), ("refresh_token", refresh_token.as_str())],
        route,
    )
    .await
    {
        result @ (Ok(_) | Err(AuthErrorCode::ReauthRequired)) => result,
        Err(error) => return Err(error),
    };

    let (_lock, keychain) = lock_for_refresh(&home).await?;
    let (mut record, item, current) =
        managed(&keychain, scope, account_id).map_err(|error| match error {
            AuthErrorCode::NotFound | AuthErrorCode::ReauthRequired => AuthErrorCode::Conflict,
            error => error,
        })?;
    if current != version {
        return Err(AuthErrorCode::Conflict);
    }

    let tokens = match outcome {
        Ok(tokens) => tokens,
        Err(error) => {
            record.account.credential_state = AuthCredentialState::ReauthRequired;
            save_managed(&keychain, record, item)?;
            return Err(error);
        }
    };

    if !valid_token(&tokens.access_token)
        || tokens.token_type.as_deref().is_some_and(|kind| !kind.eq_ignore_ascii_case("Bearer"))
    {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    let (expires_in, expiry) = token_expiry(utc_now()?, tokens.expires_in)?;
    if let Some(id_token) = tokens.id_token.filter(|value| !value.is_empty()) {
        if !valid_token(&id_token) || token_identity(&id_token).is_none() {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        record.credential["id_token"] = json!(id_token);
    }
    let token =
        record.credential["token"].as_object_mut().ok_or(AuthErrorCode::StorageUnavailable)?;
    if let Some(refresh_token) = tokens.refresh_token.filter(|value| !value.is_empty()) {
        if !valid_token(&refresh_token) {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        token.insert("refresh_token".into(), json!(refresh_token));
    }
    token.insert("access_token".into(), json!(tokens.access_token));
    token.insert("expiry".into(), json!(expiry));
    token.insert("expires_in".into(), json!(expires_in));

    if credential_account_id(scope, &record.credential).as_deref() != Some(account_id) {
        return Err(AuthErrorCode::IdentityMismatch);
    }
    save_managed(&keychain, record, item)?;
    Ok(tokens.access_token)
}

// The native quota subprocess can hold a shared lock across an await.
async fn lock_for_refresh(home: &Path) -> Result<(File, SecKeychain), AuthErrorCode> {
    let home = home.to_path_buf();
    tokio::task::spawn_blocking(move || locked_keychain(&home))
        .await
        .map_err(|_| AuthErrorCode::StorageUnavailable)?
}

/// Absorb a newer native credential for the account already bound by host policy.
/// Run this blocking operation off the async runtime.
pub fn reconcile_native_account(
    scope: AuthAccountScope<'_>,
    account_id: &str,
    native: AgyNativeTarget<'_>,
) -> Result<(), AuthErrorCode> {
    let entry = KeychainEntry::new("gemini", "antigravity").map_err(keychain_error)?;
    reconcile_in(scope, account_id, native, &entry)
}

pub(super) fn reconcile_in(
    scope: AuthAccountScope<'_>,
    account_id: &str,
    native: AgyNativeTarget<'_>,
    entry: &KeychainEntry<'_>,
) -> Result<(), AuthErrorCode> {
    let home = native_home(scope, native)?;
    validate_account_id(account_id)?;
    let (_lock, keychain) = locked_keychain(&home)?;
    let (mut record, item, _) = managed(&keychain, scope, account_id)?;
    let before = entry.read().map_err(keychain_error)?;
    let live = before.as_deref().and_then(decode).ok_or(AuthErrorCode::Conflict)?;
    if credential_account_id(scope, &live).as_deref() != Some(account_id) {
        return Err(AuthErrorCode::Conflict);
    }
    if rotation_is_newer(&live, &record.credential)? {
        if entry.read().map_err(keychain_error)? != before {
            return Err(AuthErrorCode::Conflict);
        }
        record.credential = live;
        save_managed(&keychain, record, item)?;
    }
    Ok(())
}
