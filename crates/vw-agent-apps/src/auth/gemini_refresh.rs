//! Host-internal resolution of a scoped Gemini access token.

use super::{
    credential_file::{AccountCredential, CredentialFileStore, GeminiCredential},
    credential_lease::{refresh_publication_error, refresh_publication_lease},
};
use app_auth_core::response::utc_now;
use app_auth_gemini::protocol::{GeminiOAuth, RefreshResponse};
use app_provider_validation::validate_identity;
use reqwest::Request;
use {
    vw_api_transport::TimestampMs,
    vw_api_app_identity::AppId,
    vw_api_app_auth::AuthErrorCode,
    vw_api_transport::id::TenantScope,
};

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
    oauth: GeminiOAuth,
}

impl GeminiRefresh {
    pub fn new() -> Result<Self, AuthErrorCode> {
        Ok(Self { oauth: GeminiOAuth::new()? })
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
        let result = match self.oauth.exchange(
            &current.client_id,
            &current.client_secret,
            &current.refresh_token,
            &mut route,
        ).await {
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
