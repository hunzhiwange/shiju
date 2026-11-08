//! Host-internal resolution of a scoped Codex access token.
use super::{
    codex_device::{CLIENT_ID, TOKEN_URL, oauth_client, send, token_subject, workspace_id},
    copilot::{read_json, utc_now, valid_token},
    credential_file::{AccountCredential, CredentialFileStore},
    flow::AuthAccountScope,
};
use crate::providers::{codex_login_material::parse_codex_jwt_claims, validate_identity};
use reqwest::{Client, Request, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthService},
    id::TenantScope,
};

const REFRESH_BUFFER_MS: i64 = 60_000;
const DEFAULT_LIFETIME_SECONDS: i64 = 3600;

/// Share this adapter across requests on the execution host.
pub struct CodexRefresh {
    client: Client,
    slots: Mutex<HashMap<AccountKey, RefreshSlot>>,
}

#[derive(PartialEq, Eq, Hash)]
struct AccountKey {
    tenant: TenantScope,
    principal: String,
    host: String,
    id: String,
}

type RefreshSlot = Arc<tokio::sync::Mutex<Option<CachedGeneration>>>;

struct CachedGeneration {
    digest: [u8; 32],
    state: RefreshState,
}

enum RefreshState {
    Ready { refresh_at: i64 },
    ReauthRequired,
}

#[derive(Deserialize)]
struct RefreshResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_in: Option<i64>,
}

impl CodexRefresh {
    pub fn new() -> Result<Self, AuthErrorCode> {
        Ok(Self { client: oauth_client()?, slots: Mutex::default() })
    }

    /// Resolve a managed account's access token for use on the authorized host.
    pub async fn access_token(
        &self,
        credentials: &CredentialFileStore,
        scope: AuthAccountScope<'_>,
        account_id: &str,
    ) -> Result<String, AuthErrorCode> {
        self.access_token_with(credentials, scope, account_id, |_| {}).await
    }

    pub(super) async fn access_token_with(
        &self,
        credentials: &CredentialFileStore,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        mut route: impl FnMut(&mut Request),
    ) -> Result<String, AuthErrorCode> {
        scope.validate()?;
        if scope.service != AuthService::CodexOauth || !cfg!(unix) {
            return Err(AuthErrorCode::Unsupported);
        }
        validate_identity(account_id).map_err(|_| AuthErrorCode::InvalidInput)?;
        let key = AccountKey {
            tenant: scope.tenant.clone(),
            principal: scope.principal.to_owned(),
            host: scope.host.to_owned(),
            id: account_id.to_owned(),
        };
        let slot = {
            let mut slots =
                self.slots.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
            Arc::clone(slots.entry(key).or_default())
        };
        let mut cached = slot.lock().await;
        // Reread after acquiring the account mutex. The file lease excludes other
        // managed writers through the upstream exchange and atomic publication.
        let lease = credentials
            .for_owner(scope.principal)
            .and_then(|owner| {
                owner.lease(scope.tenant, scope.host, AppId::Codex, account_id, false)
            })
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let read = || match lease.read() {
            Ok(AccountCredential::CodexAuth(value)) => Ok(value),
            _ => Err(AuthErrorCode::StorageUnavailable),
        };
        let current = read()?;
        let digest = generation_digest(&current);
        let tokens = &current["tokens"];
        let access = tokens["access_token"].as_str().ok_or(AuthErrorCode::StorageUnavailable)?;
        let now = utc_now()?.0;
        match cached.as_ref().filter(|entry| entry.digest == digest) {
            Some(CachedGeneration { state: RefreshState::ReauthRequired, .. }) => {
                return Err(AuthErrorCode::ReauthRequired);
            }
            Some(CachedGeneration { state: RefreshState::Ready { refresh_at }, .. }) => {
                if now < *refresh_at {
                    return Ok(access.to_owned());
                }
            }
            None => {
                if jwt_expiry(access).is_some_and(|expiry| expiry > now + REFRESH_BUFFER_MS) {
                    return Ok(access.to_owned());
                }
            }
        }

        let refresh = tokens["refresh_token"]
            .as_str()
            .filter(|token| valid_token(token))
            .ok_or(AuthErrorCode::ReauthRequired)?;
        let old_claims = tokens["id_token"]
            .as_str()
            .and_then(parse_codex_jwt_claims)
            .ok_or(AuthErrorCode::ReauthRequired)?;
        let subject = token_subject(&old_claims).ok_or(AuthErrorCode::ReauthRequired)?;
        let result = self.exchange(refresh, &mut route).await;
        if read()? != current {
            return Err(AuthErrorCode::Conflict);
        }
        let refreshed = match result {
            Err(AuthErrorCode::ReauthRequired) => {
                *cached = Some(CachedGeneration { digest, state: RefreshState::ReauthRequired });
                return Err(AuthErrorCode::ReauthRequired);
            }
            result => result?,
        };
        let received_at = utc_now()?.0;
        let expires_at = expires_at(&refreshed, received_at)?;
        let refresh_at = expires_at - REFRESH_BUFFER_MS.min((expires_at - received_at) / 2);
        let updated = refreshed_credential(current, &refreshed, subject, received_at)?;
        let digest = generation_digest(&updated);
        let updated = AccountCredential::CodexAuth(updated);
        updated.validate(AppId::Codex).map_err(|_| AuthErrorCode::UpstreamFailure)?;
        lease.save(&updated).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        *cached = Some(CachedGeneration { digest, state: RefreshState::Ready { refresh_at } });
        Ok(refreshed.access_token)
    }

    async fn exchange(
        &self,
        refresh_token: &str,
        route: &mut impl FnMut(&mut Request),
    ) -> Result<RefreshResponse, AuthErrorCode> {
        let response = send(
            &self.client,
            self.client.post(TOKEN_URL).form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", CLIENT_ID),
                ("scope", "openid profile email"),
            ]),
            route,
        )
        .await?;
        let status = response.status();
        if status.is_success() {
            return read_json(response).await;
        }
        if status == StatusCode::UNAUTHORIZED
            || read_json::<Value>(response).await.is_ok_and(|body| permanent_rejection(&body))
        {
            return Err(AuthErrorCode::ReauthRequired);
        }
        Err(AuthErrorCode::UpstreamFailure)
    }
}

fn permanent_rejection(body: &Value) -> bool {
    let error = &body["error"];
    let code = error.as_str().or_else(|| error["code"].as_str()).or_else(|| body["code"].as_str());
    code.is_some_and(|code| {
        [
            "invalid_grant",
            "refresh_token_expired",
            "refresh_token_reused",
            "refresh_token_invalidated",
        ]
        .iter()
        .any(|known| known.eq_ignore_ascii_case(code))
    })
}

fn generation_digest(credential: &Value) -> [u8; 32] {
    Sha256::digest(credential.to_string().as_bytes()).into()
}

fn jwt_expiry(access_token: &str) -> Option<i64> {
    parse_codex_jwt_claims(access_token)?.get("exp")?.as_i64()?.checked_mul(1000)
}

fn expires_at(response: &RefreshResponse, now: i64) -> Result<i64, AuthErrorCode> {
    let lifetime = response
        .expires_in
        .unwrap_or(DEFAULT_LIFETIME_SECONDS)
        .checked_mul(1000)
        .and_then(|duration| now.checked_add(duration))
        .ok_or(AuthErrorCode::UpstreamFailure)?;
    let expires_at = jwt_expiry(&response.access_token).map_or(lifetime, |exp| exp.min(lifetime));
    if expires_at <= now {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    Ok(expires_at)
}

fn refreshed_credential(
    mut current: Value,
    refreshed: &RefreshResponse,
    old_subject: &str,
    received_at: i64,
) -> Result<Value, AuthErrorCode> {
    let tokens = &mut current["tokens"];
    let workspace = tokens["account_id"].as_str().ok_or(AuthErrorCode::StorageUnavailable)?;
    if !valid_token(&refreshed.access_token) {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    if parse_codex_jwt_claims(&refreshed.access_token)
        .as_ref()
        .and_then(workspace_id)
        .is_some_and(|received| received != workspace)
    {
        return Err(AuthErrorCode::IdentityMismatch);
    }
    if let Some(id_token) = refreshed.id_token.as_deref().filter(|token| !token.trim().is_empty()) {
        let claims = parse_codex_jwt_claims(id_token).ok_or(AuthErrorCode::UpstreamFailure)?;
        let subject = token_subject(&claims).ok_or(AuthErrorCode::UpstreamFailure)?;
        if subject != old_subject || workspace_id(&claims).is_some_and(|id| id != workspace) {
            return Err(AuthErrorCode::IdentityMismatch);
        }
        tokens["id_token"] = id_token.into();
    }
    if let Some(refresh_token) =
        refreshed.refresh_token.as_deref().filter(|token| !token.trim().is_empty())
    {
        tokens["refresh_token"] = refresh_token.into();
    }
    tokens["access_token"] = refreshed.access_token.as_str().into();
    current["last_refresh"] = chrono::DateTime::from_timestamp_millis(received_at)
        .ok_or(AuthErrorCode::ExecutionHostUnavailable)?
        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
        .into();
    Ok(current)
}
