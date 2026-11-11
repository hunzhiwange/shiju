//! Host-internal resolution of a scoped Gemini access token.

use super::{
    codex_device::send,
    copilot::{read_body_limited, utc_now},
    credential_file::{AccountCredential, CredentialFileStore, GeminiCredential},
    credential_lease::{refresh_publication_error, refresh_publication_lease},
};
use crate::providers::validate_identity;
use reqwest::{Client, Request, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use vw_api_types::{
    TimestampMs, agent_apps::AppId, agent_apps_auth::AuthErrorCode, id::TenantScope,
};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const REFRESH_BUFFER_MS: i64 = 60_000;
fn valid_token(token: &str) -> bool {
    !token.is_empty() && token.len() <= 16384 && token.bytes().all(|b| b.is_ascii_graphic())
}
const DEFAULT_LIFETIME_SECONDS: i64 = 3_600;

/// Scope supplied by the authenticated host owner, never request headers.
#[derive(Clone, Copy)]
pub struct GeminiScope<'a> {
    pub tenant: &'a TenantScope,
    pub principal: &'a str,
    pub host: &'a str,
}

pub struct GeminiRefresh {
    client: Client,
}

#[derive(Deserialize)]
struct RefreshResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

impl GeminiRefresh {
    pub fn new() -> Result<Self, AuthErrorCode> {
        Ok(Self {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("ShiJu-Gemini-OAuth")
                .build()
                .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?,
        })
    }

    /// Resolve a managed account's access token for use on the authorized host.
    pub async fn access_token(
        &self,
        credentials: &CredentialFileStore,
        scope: GeminiScope<'_>,
        account_id: &str,
    ) -> Result<String, AuthErrorCode> {
        self.access_token_with(credentials, scope, account_id, |_| {}).await
    }

    pub(super) async fn access_token_with(
        &self,
        credentials: &CredentialFileStore,
        scope: GeminiScope<'_>,
        account_id: &str,
        mut route: impl FnMut(&mut Request),
    ) -> Result<String, AuthErrorCode> {
        if !cfg!(unix) {
            return Err(AuthErrorCode::Unsupported);
        }
        validate_identity(account_id).map_err(|_| AuthErrorCode::InvalidInput)?;
        validate_identity(scope.host).map_err(|_| AuthErrorCode::InvalidInput)?;
        let owner =
            credentials.for_owner(scope.principal).map_err(|_| AuthErrorCode::InvalidInput)?;
        let open = || owner.lease(scope.tenant, scope.host, AppId::Gemini, account_id, false);
        let lease = open().map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let (credential, generation) =
            lease.read_for_refresh().map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let AccountCredential::GeminiOauth(mut current) = credential else {
            return Err(AuthErrorCode::StorageUnavailable);
        };
        if current.requires_reauth {
            return Err(AuthErrorCode::ReauthRequired);
        }
        if current.expires_at.0 > utc_now()?.0.saturating_add(REFRESH_BUFFER_MS) {
            return Ok(current.access_token);
        }

        drop(lease);
        let result = match self.exchange(&current, &mut route).await {
            Ok(refreshed) => {
                current.apply_refresh(refreshed)?;
                Ok(current.access_token.clone())
            }
            Err(AuthErrorCode::ReauthRequired) => {
                current.requires_reauth = true;
                Err(AuthErrorCode::ReauthRequired)
            }
            Err(error) => return Err(error),
        };
        refresh_publication_lease(open)
            .await?
            .save_refreshed(&generation, &AccountCredential::GeminiOauth(current))
            .map_err(refresh_publication_error)?;
        result
    }

    async fn exchange(
        &self,
        current: &GeminiCredential,
        route: &mut impl FnMut(&mut Request),
    ) -> Result<RefreshResponse, AuthErrorCode> {
        let endpoint = "https://oauth2.googleapis.com/token";
        let response = send(
            &self.client,
            self.client.post(endpoint).form(&[
                ("grant_type", "refresh_token"),
                ("client_id", current.client_id.as_str()),
                ("client_secret", current.client_secret.as_str()),
                ("refresh_token", current.refresh_token.as_str()),
            ]),
            route,
        )
        .await?;
        let status = response.status();
        if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
            return Err(AuthErrorCode::ReauthRequired);
        }
        if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        // Only a fully received malformed 400 is a Gemini rejection. A truncated,
        // oversized or unreadable response remains an upstream failure.
        let body = read_body_limited(response, MAX_RESPONSE_BYTES).await?;
        let payload: Value = serde_json::from_slice(&body).map_err(|_| {
            if status == StatusCode::BAD_REQUEST {
                AuthErrorCode::ReauthRequired
            } else {
                AuthErrorCode::UpstreamFailure
            }
        })?;
        match payload.get("error").and_then(Value::as_str) {
            Some("invalid_grant" | "invalid_token") => Err(AuthErrorCode::ReauthRequired),
            None if status.is_success() => {
                serde_json::from_value(payload).map_err(|_| AuthErrorCode::UpstreamFailure)
            }
            _ => Err(AuthErrorCode::UpstreamFailure),
        }
    }
}

impl GeminiCredential {
    fn apply_refresh(&mut self, refreshed: RefreshResponse) -> Result<(), AuthErrorCode> {
        if !valid_token(&refreshed.access_token) {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let now = utc_now()?.0;
        let expires_at = refreshed
            .expires_in
            .unwrap_or(DEFAULT_LIFETIME_SECONDS)
            .checked_mul(1_000)
            .and_then(|duration| now.checked_add(duration))
            .filter(|expires_at| *expires_at > now)
            .ok_or(AuthErrorCode::UpstreamFailure)?;
        if let Some(token) = refreshed.refresh_token.filter(|token| !token.trim().is_empty()) {
            if !valid_token(&token) {
                return Err(AuthErrorCode::UpstreamFailure);
            }
            self.refresh_token = token;
        }
        self.access_token = refreshed.access_token;
        self.expires_at = TimestampMs(expires_at);
        Ok(())
    }
}
