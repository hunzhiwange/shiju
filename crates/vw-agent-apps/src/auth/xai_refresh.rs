//! Host-internal resolution of a scoped xAI access token.

use super::{
    codex_device::send,
    copilot::{read_body_limited, utc_now},
    credential_file::{AccountCredential, CredentialFileStore, XaiCredential},
    credential_lease::{refresh_publication_error, refresh_publication_lease},
    flow::AuthAccountScope,
    xai::{CLIENT_ID, MAX_RESPONSE_BYTES, SCOPE, discover, oauth_client, valid_token},
};
use crate::providers::validate_identity;
use reqwest::{Client, Request, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthErrorCode, AuthService},
};

const REFRESH_BUFFER_MS: i64 = 60_000;
const DEFAULT_LIFETIME_SECONDS: i64 = 3_600;

pub struct XaiRefresh {
    client: Client,
}

#[derive(Deserialize)]
struct RefreshResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

impl XaiRefresh {
    pub fn new() -> Result<Self, AuthErrorCode> {
        Ok(Self { client: oauth_client()? })
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
        if scope.service != AuthService::XaiOauth || !cfg!(unix) {
            return Err(AuthErrorCode::Unsupported);
        }
        validate_identity(account_id).map_err(|_| AuthErrorCode::InvalidInput)?;
        let open = || credentials.lease_xai(scope, account_id, false);
        let lease = open().map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let (credential, generation) =
            lease.read_for_refresh().map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let AccountCredential::XaiOauth(mut current) = credential else {
            return Err(AuthErrorCode::StorageUnavailable);
        };
        if current.requires_reauth {
            return Err(AuthErrorCode::ReauthRequired);
        }
        if current.expires_at.0 > utc_now()?.0.saturating_add(REFRESH_BUFFER_MS) {
            return Ok(current.access_token);
        }

        drop(lease);
        let result = match self.exchange(&current.refresh_token, &mut route).await {
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
            .save_refreshed(&generation, &AccountCredential::XaiOauth(current))
            .map_err(refresh_publication_error)?;
        result
    }

    async fn exchange(
        &self,
        refresh_token: &str,
        route: &mut impl FnMut(&mut Request),
    ) -> Result<RefreshResponse, AuthErrorCode> {
        let endpoint = discover(&self.client, route).await?.token_endpoint()?;
        let response = send(
            &self.client,
            self.client.post(endpoint).form(&[
                ("grant_type", "refresh_token"),
                ("client_id", CLIENT_ID),
                ("refresh_token", refresh_token),
                ("scope", SCOPE),
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
        // Only a fully received malformed 400 is an xAI rejection. A truncated,
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

impl XaiCredential {
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
