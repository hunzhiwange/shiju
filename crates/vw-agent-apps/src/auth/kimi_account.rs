//! Host-owned selection and refresh of an existing Kimi OAuth profile.

mod binding;
mod logout;
mod native;

use super::{
    codex_device::send,
    copilot::{read_json_limited, utc_now},
    credential_file::{AccountCredential, CredentialFileStore, KimiCredential},
    flow::AuthAccountScope,
    kimi::{
        CLIENT_ID, KimiDeviceAuth, MAX_RESPONSE_BYTES, MAX_TOKEN_LIFETIME, endpoints, identify,
        identity_field, seconds, token,
    },
};
use crate::providers::{
    kimi::{self, KimiFiles},
    live::{
        commit::{ExecutionHostId, Mode, ProviderTransactions},
        write_guard::WriteError,
    },
};
use native::NativeSlot;
use reqwest::{Request, StatusCode};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Arc};
use toml_edit::{DocumentMut, value};
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthRegion, AuthService},
};

/// The scoped metadata verified when the managed account signed in.
pub struct KimiAccountTarget<'a> {
    pub scope: AuthAccountScope<'a>,
    pub account_id: &'a str,
    pub region: AuthRegion,
    pub identity_sha256: [u8; 32],
}

/// A native profile resolved by the authorized execution host.
pub struct KimiNativeTarget<'a> {
    pub home: &'a Path,
    pub transactions: &'a Arc<ProviderTransactions>,
    pub provider_id: &'a str,
}

fn resolve_default_provider(
    credentials: &CredentialFileStore,
    account: &KimiAccountTarget<'_>,
    home: &Path,
    transactions: &Arc<ProviderTransactions>,
) -> Result<String, AuthErrorCode> {
    let KimiAccountTarget { scope, account_id, region, .. } = *account;
    let saved = logout::read_managed_grant(credentials, scope, account_id)?
        .ok_or(AuthErrorCode::ReauthRequired)?;
    if saved.region != region {
        return Err(AuthErrorCode::IdentityMismatch);
    }
    let config = kimi::read(home).map_err(write_error)?.ok_or(AuthErrorCode::NotFound)?;
    let profiles = kimi::declarations(Some(&config)).map_err(write_error)?;
    if transactions.pending(AppId::Kimi).map_err(write_error)?.is_some() {
        return Err(AuthErrorCode::Conflict);
    }
    let state = transactions.state(AppId::Kimi).map_err(write_error)?;
    let mut selected = None;
    for (provider_id, profile) in &profiles {
        if !kimi::official(&profile.declaration["provider"]) {
            continue;
        }
        let native = KimiNativeTarget { home, transactions, provider_id };
        native.check_scope(scope)?;
        let matches = match native.binding(&state, scope.tenant, scope.principal) {
            Ok(Some(binding)) => binding.account_id == account_id && binding.region == region,
            Ok(None) => match oauth_slot(&profile.declaration["provider"], region) {
                Ok(slot) => logout::read_grant(&NativeSlot::open(home, slot)?)?
                    .is_some_and(|grant| shares_grant(&grant, &saved)),
                Err(AuthErrorCode::IdentityMismatch | AuthErrorCode::Unsupported) => false,
                Err(error) => return Err(error),
            },
            Err(AuthErrorCode::Forbidden) => false,
            Err(error) => return Err(error),
        };
        if matches && selected.replace(provider_id.clone()).is_some() {
            return Err(AuthErrorCode::Conflict);
        }
    }
    if transactions.state(AppId::Kimi).map_err(write_error)? != state
        || transactions.pending(AppId::Kimi).map_err(write_error)?.is_some()
        || kimi::read(home).map_err(write_error)?.as_ref() != Some(&config)
    {
        return Err(AuthErrorCode::Conflict);
    }
    selected.ok_or(AuthErrorCode::ReauthRequired)
}

pub(super) enum Selection<'a> {
    Explicit,
    Default,
    Bound(&'a Value),
}

impl KimiDeviceAuth {
    /// Select a unique scoped binding or exact native grant for the managed account.
    pub async fn use_default_account(
        &self,
        credentials: &CredentialFileStore,
        account: KimiAccountTarget<'_>,
        home: &Path,
        transactions: &Arc<ProviderTransactions>,
    ) -> Result<(), AuthErrorCode> {
        self.use_default_account_with(credentials, account, home, transactions, |_| {}).await
    }

    pub(super) async fn use_default_account_with(
        &self,
        credentials: &CredentialFileStore,
        account: KimiAccountTarget<'_>,
        home: &Path,
        transactions: &Arc<ProviderTransactions>,
        route: impl FnMut(&mut Request),
    ) -> Result<(), AuthErrorCode> {
        let provider_id = resolve_default_provider(credentials, &account, home, transactions)?;
        let native = KimiNativeTarget { home, transactions, provider_id: &provider_id };
        self.select_account(credentials, account, native, Selection::Default, || true, route).await
    }

    /// Select one of the existing native provider's model aliases after verifying the account.
    pub async fn use_account(
        &self,
        credentials: &CredentialFileStore,
        account: KimiAccountTarget<'_>,
        native: KimiNativeTarget<'_>,
    ) -> Result<(), AuthErrorCode> {
        self.use_account_with(credentials, account, native, |_| {}).await
    }

    /// Consume a provider binding only while its catalog connection and native binding agree.
    pub async fn use_bound_account(
        &self,
        credentials: &CredentialFileStore,
        account: KimiAccountTarget<'_>,
        native: KimiNativeTarget<'_>,
        catalog_connection: &Value,
        catalog_unchanged: impl Fn() -> bool + Send + 'static,
    ) -> Result<(), AuthErrorCode> {
        self.select_account(
            credentials,
            account,
            native,
            Selection::Bound(catalog_connection),
            catalog_unchanged,
            |_| {},
        )
        .await
    }

    pub(super) async fn use_account_with(
        &self,
        credentials: &CredentialFileStore,
        account: KimiAccountTarget<'_>,
        native: KimiNativeTarget<'_>,
        route: impl FnMut(&mut Request),
    ) -> Result<(), AuthErrorCode> {
        self.select_account(credentials, account, native, Selection::Explicit, || true, route).await
    }

    pub(super) async fn select_account(
        &self,
        credentials: &CredentialFileStore,
        account: KimiAccountTarget<'_>,
        native: KimiNativeTarget<'_>,
        selection: Selection<'_>,
        catalog_unchanged: impl Fn() -> bool + Send + 'static,
        mut route: impl FnMut(&mut Request),
    ) -> Result<(), AuthErrorCode> {
        let KimiAccountTarget { scope, account_id, region, identity_sha256 } = account;
        native.check_scope(scope)?;
        crate::providers::validate_identity(account_id).map_err(|_| AuthErrorCode::InvalidInput)?;
        let lease =
            credentials.lease_kimi(scope, account_id, false).map_err(binding::lease_error)?;
        let AccountCredential::KimiOauth(saved) =
            lease.read().map_err(|_| AuthErrorCode::StorageUnavailable)?
        else {
            return Err(AuthErrorCode::StorageUnavailable);
        };
        if saved.region != region {
            return Err(AuthErrorCode::IdentityMismatch);
        }

        let before =
            kimi::read(native.home).map_err(write_error)?.ok_or(AuthErrorCode::NotFound)?;
        if native.transactions.pending(AppId::Kimi).map_err(write_error)?.is_some() {
            let home = native.home.to_owned();
            let transactions = Arc::clone(native.transactions);
            tokio::task::spawn_blocking(move || {
                let mut files = KimiFiles::open(&home).map_err(write_error)?;
                transactions.recover(&mut files.guard).map_err(write_error)
            })
            .await
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)??;
            return Err(AuthErrorCode::Conflict);
        }
        let expected = native.transactions.state(AppId::Kimi).map_err(write_error)?;
        if expected.pointer.is_some()
            || expected.mode_tenant.as_ref().is_some_and(|tenant| tenant != scope.tenant)
            || expected.mode.as_ref().is_some_and(|mode| mode.attached || mode.mode != Mode::Direct)
        {
            return Err(AuthErrorCode::Conflict);
        }
        let binding = native.binding(&expected, scope.tenant, scope.principal)?;
        if let Selection::Bound(connection) = selection {
            binding.ok_or(AuthErrorCode::NotFound)?;
            binding::check_catalog(&before, native.provider_id, connection)?;
            if !catalog_unchanged() {
                return Err(AuthErrorCode::Conflict);
            }
        }
        let (slot, next_config) = select_model(&before, native.provider_id, region)?;
        if binding.is_some_and(|binding| {
            binding.account_id != account_id || binding.region != region || binding.slot != slot
        }) {
            return Err(AuthErrorCode::IdentityMismatch);
        }
        let slot = NativeSlot::open(native.home, &slot)?;
        let original = slot.read()?;
        if matches!(selection, Selection::Default)
            && binding.is_none()
            && !shares_grant(&original, &saved)
        {
            return Err(AuthErrorCode::Conflict);
        }
        let current =
            self.resolve_native(&slot, &original, &saved, identity_sha256, &mut route).await?;
        let rotated = shares_grant(&original, &saved) && original != current;

        let tenant = scope.tenant.clone();
        let home = native.home.to_owned();
        let transactions = Arc::clone(native.transactions);
        tokio::task::spawn_blocking(move || {
            let mut files = KimiFiles::open(&home).map_err(write_error)?;
            if !catalog_unchanged() {
                return Err(AuthErrorCode::Conflict);
            }
            if rotated {
                // Only synchronize a rotation from a slot that started with the
                // managed grant. A separately signed-in CLI account stays separate.
                let expires_at = expiry(&current)?
                    .checked_mul(1_000)
                    .ok_or(AuthErrorCode::StorageUnavailable)?;
                if slot.read()? != current {
                    return Err(AuthErrorCode::Conflict);
                }
                lease
                    .save(&AccountCredential::KimiOauth(KimiCredential {
                        access_token: token(&current, "access_token")?.to_owned(),
                        refresh_token: token(&current, "refresh_token")?.to_owned(),
                        expires_at: TimestampMs(expires_at),
                        ..saved
                    }))
                    .map_err(|_| AuthErrorCode::StorageUnavailable)?;
            }
            let _lease = lease;
            let expires_at = expiry(&current)?;
            files
                .apply(
                    &transactions,
                    &tenant,
                    &expected,
                    "config.toml",
                    Some(&before),
                    next_config,
                    || {
                        catalog_unchanged()
                            && utc_now().is_ok_and(|now| expires_at > now.0 / 1_000)
                            && slot.read().is_ok_and(|latest| latest == current)
                    },
                )
                .map_err(write_error)
        })
        .await
        .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?
    }

    async fn resolve_native(
        &self,
        slot: &NativeSlot,
        original: &Value,
        saved: &KimiCredential,
        identity: [u8; 32],
        route: &mut impl FnMut(&mut Request),
    ) -> Result<Value, AuthErrorCode> {
        let now = utc_now()?.0 / 1_000;
        let expires = expiry(original)?;
        let refresh =
            token(original, "refresh_token").map_err(|_| AuthErrorCode::ReauthRequired)?;
        let access = token(original, "access_token").map_err(|_| AuthErrorCode::ReauthRequired)?;
        let same_grant = shares_grant(original, saved);
        let device = if same_grant {
            saved.device_id.clone()
        } else {
            if expires <= now {
                return Err(AuthErrorCode::ReauthRequired);
            }
            slot.device_id()?
        };
        let buffer = (original["expires_in"].as_i64().unwrap_or_default() / 2).max(300);
        if expires.saturating_sub(now) >= buffer {
            self.verify_identity(saved.region, &device, identity, access, route).await?;
            return Ok(original.clone());
        }
        // A separate CLI grant must prove its identity before we refresh it.
        if !same_grant {
            self.verify_identity(saved.region, &device, identity, access, route).await?;
        }
        if slot.read()? != *original {
            return Err(AuthErrorCode::Conflict);
        }

        let refreshed = self.refresh_native(saved.region, &device, original, refresh, route).await;
        // A CLI process may already have replaced this generation. Recheck on
        // both success and rejection and never clobber a newer verified rotation.
        let current = slot.read()?;
        if current != *original {
            let current_access =
                token(&current, "access_token").map_err(|_| AuthErrorCode::ReauthRequired)?;
            let current_refresh =
                token(&current, "refresh_token").map_err(|_| AuthErrorCode::ReauthRequired)?;
            if current_refresh == refresh || expiry(&current)? <= expires.max(utc_now()?.0 / 1_000)
            {
                return Err(AuthErrorCode::Conflict);
            }
            self.verify_identity(saved.region, &device, identity, current_access, route).await?;
            return Ok(current);
        }
        let refreshed = match refreshed {
            Ok(refreshed) => refreshed,
            Err(AuthErrorCode::ReauthRequired) => {
                let mut revoked = original.clone();
                revoked["access_token"] = json!("");
                revoked["refresh_token"] = json!("");
                revoked["expires_at"] = json!(0);
                revoked["expires_in"] = json!(0);
                slot.replace(original, &revoked)?;
                return Err(AuthErrorCode::ReauthRequired);
            }
            Err(error) => return Err(error),
        };
        self.verify_identity(
            saved.region,
            &device,
            identity,
            token(&refreshed, "access_token")?,
            route,
        )
        .await?;
        slot.replace(original, &refreshed)?;
        Ok(refreshed)
    }

    async fn refresh_native(
        &self,
        region: AuthRegion,
        device: &str,
        original: &Value,
        refresh: &str,
        route: &mut impl FnMut(&mut Request),
    ) -> Result<Value, AuthErrorCode> {
        let region = endpoints(region);
        let response = send(
            &self.client,
            identify(self.client.post(format!("{}/api/oauth/token", region.oauth)), device).form(
                &[
                    ("client_id", CLIENT_ID),
                    ("grant_type", "refresh_token"),
                    ("refresh_token", refresh),
                ],
            ),
            route,
        )
        .await?;
        let status = response.status();
        if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
            return Err(AuthErrorCode::ReauthRequired);
        }
        let payload: Value = read_json_limited(response, MAX_RESPONSE_BYTES).await?;
        if status == StatusCode::BAD_REQUEST && payload["error"] == "invalid_grant" {
            return Err(AuthErrorCode::ReauthRequired);
        }
        if status != StatusCode::OK {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let lifetime = seconds(&payload, "expires_in", None, MAX_TOKEN_LIFETIME)?;
        let mut refreshed = original.clone();
        refreshed["access_token"] = json!(token(&payload, "access_token")?);
        refreshed["refresh_token"] = json!(token(&payload, "refresh_token")?);
        refreshed["expires_at"] = json!(utc_now()?.0 / 1_000 + lifetime as i64);
        refreshed["expires_in"] = json!(lifetime);
        for field in ["scope", "token_type"] {
            if let Some(value) = payload[field].as_str() {
                refreshed[field] = json!(value);
            }
        }
        Ok(refreshed)
    }

    async fn verify_identity(
        &self,
        region: AuthRegion,
        device: &str,
        expected: [u8; 32],
        access: &str,
        route: &mut impl FnMut(&mut Request),
    ) -> Result<(), AuthErrorCode> {
        let region = endpoints(region);
        let response = send(
            &self.client,
            identify(self.client.get(format!("{}/me", region.api)), device).bearer_auth(access),
            route,
        )
        .await?;
        match response.status() {
            StatusCode::OK => {}
            StatusCode::UNAUTHORIZED => return Err(AuthErrorCode::ReauthRequired),
            _ => return Err(AuthErrorCode::UpstreamFailure),
        }
        let me: Value = read_json_limited(response, MAX_RESPONSE_BYTES).await?;
        let subject = identity_field(&me["user_id"])
            .or_else(|| identity_field(&me["id"]))
            .ok_or(AuthErrorCode::UpstreamFailure)?;
        let actual: [u8; 32] =
            Sha256::digest(format!("{}:{subject}", region.identity_prefix)).into();
        if actual != expected {
            return Err(AuthErrorCode::IdentityMismatch);
        }
        Ok(())
    }
}

fn shares_grant(native: &Value, saved: &KimiCredential) -> bool {
    native["access_token"] == saved.access_token.as_str()
        && native["refresh_token"] == saved.refresh_token.as_str()
}

fn expiry(tokens: &Value) -> Result<i64, AuthErrorCode> {
    tokens["expires_at"].as_i64().filter(|expiry| *expiry > 0).ok_or(AuthErrorCode::ReauthRequired)
}

fn select_model(
    config: &[u8],
    provider_id: &str,
    region: AuthRegion,
) -> Result<(String, Vec<u8>), AuthErrorCode> {
    let mut profiles = kimi::declarations(Some(config)).map_err(write_error)?;
    let profile = profiles.remove(provider_id).ok_or(AuthErrorCode::NotFound)?;
    let slot = oauth_slot(&profile.declaration["provider"], region)?;
    let models = profile.declaration["models"].as_object().ok_or(AuthErrorCode::InvalidInput)?;
    let mut doc = std::str::from_utf8(config)
        .ok()
        .and_then(|text| text.parse::<DocumentMut>().ok())
        .ok_or(AuthErrorCode::InvalidInput)?;
    let current = doc.get("default_model").and_then(toml_edit::Item::as_str);
    if !current.is_some_and(|alias| models.contains_key(alias)) {
        let next = models.keys().next().ok_or(AuthErrorCode::NotFound)?;
        let mut selected = value(next.as_str());
        if let Some(decor) = doc.get("default_model").and_then(toml_edit::Item::as_value) {
            *selected.as_value_mut().expect("TOML string").decor_mut() = decor.decor().clone();
        }
        doc["default_model"] = selected;
    }
    Ok((slot.into(), doc.to_string().into_bytes()))
}

fn oauth_slot(provider: &Value, region: AuthRegion) -> Result<&str, AuthErrorCode> {
    if !kimi::official(provider) {
        return Err(AuthErrorCode::Unsupported);
    }
    let endpoint = endpoints(region);
    let oauth = &provider["oauth"];
    let oauth_host = oauth.get("oauth_host").and_then(Value::as_str);
    if provider["base_url"].as_str().map(|url| url.trim_end_matches('/')) != Some(endpoint.api)
        || match oauth.get("oauth_host") {
            None => region != AuthRegion::MainlandCn,
            Some(_) => oauth_host != Some(endpoint.oauth),
        }
    {
        return Err(AuthErrorCode::IdentityMismatch);
    }
    if oauth.get("storage").is_some_and(|storage| storage.as_str() != Some("file"))
        || provider.get("api_key_env").is_some()
        || provider.get("env").is_some()
        || provider.get("custom_headers").is_some()
        || provider.get("api_key").is_some_and(|key| key.as_str() != Some(""))
    {
        return Err(AuthErrorCode::Unsupported);
    }
    let key = oauth["key"].as_str().ok_or(AuthErrorCode::InvalidInput)?;
    let slot = key.strip_prefix("oauth/").unwrap_or(key);
    if slot.is_empty()
        || slot.len() > 128
        || !slot.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        return Err(AuthErrorCode::InvalidInput);
    }
    Ok(slot)
}

fn write_error(error: WriteError) -> AuthErrorCode {
    match error {
        WriteError::Conflict | WriteError::Recovered => AuthErrorCode::Conflict,
        WriteError::InvalidResource => AuthErrorCode::InvalidInput,
        WriteError::Io => AuthErrorCode::StorageUnavailable,
    }
}
