//! Account-specific Copilot request credentials on the execution host.
use super::{
    copilot::{
        CopilotAuth, PUBLIC_AUTHORITY, canonical_authority, read_json, utc_now, valid_token,
    },
    credential_file::{AccountCredential, CredentialFileStore},
    flow::AuthAccountScope,
};
use reqwest::{Request, Response, StatusCode, Url, header};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use vw_api_types::{
    agent_apps_auth::{AuthErrorCode, AuthService},
    id::TenantScope,
};

const REFRESH_BUFFER_SECONDS: i64 = 60;
const ENTERPRISE_ENDPOINT_TTL_SECONDS: i64 = 300;

/// The endpoint and its sensitive Authorization value belong to the same account.
#[derive(Clone)]
pub struct CopilotSession {
    pub endpoint: Url,
    pub authorization: header::HeaderValue,
}

#[derive(PartialEq, Eq, Hash)]
struct SessionKey {
    tenant: TenantScope,
    principal: String,
    host: String,
    account_id: String,
    authority: String,
}

struct CachedSession {
    oauth_digest: [u8; 32],
    refresh_at: i64,
    session: CopilotSession,
}

type SessionSlot = Arc<tokio::sync::Mutex<Option<CachedSession>>>;

#[derive(Default)]
pub(super) struct CopilotSessions(Mutex<HashMap<SessionKey, SessionSlot>>);

#[derive(Deserialize)]
pub(super) struct CopilotSubscription {
    pub token: String,
    pub expires_at: i64,
}

#[derive(Deserialize)]
struct Discovery {
    endpoints: Option<Endpoints>,
}

#[derive(Deserialize)]
struct Endpoints {
    api: String,
}

impl CopilotAuth {
    /// Resolve an explicit managed account. The authorized host supplies the authority
    /// recorded at login and consumes the result internally.
    pub async fn session(
        &self,
        credentials: &CredentialFileStore,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        authority: &str,
    ) -> Result<CopilotSession, AuthErrorCode> {
        self.session_with(
            credentials,
            scope,
            account_id,
            authority,
            || Ok(utc_now()?.0 / 1_000),
            |_| {},
        )
        .await
    }

    pub(super) async fn session_with(
        &self,
        credentials: &CredentialFileStore,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        authority: &str,
        now: impl Fn() -> Result<i64, AuthErrorCode>,
        mut route: impl FnMut(&mut Request),
    ) -> Result<CopilotSession, AuthErrorCode> {
        scope.validate()?;
        if scope.service != AuthService::GithubCopilot || !cfg!(unix) {
            return Err(AuthErrorCode::Unsupported);
        }
        crate::providers::validate_identity(account_id).map_err(|_| AuthErrorCode::InvalidInput)?;
        let authority = canonical_authority(Some(authority))?;
        let key = SessionKey {
            tenant: scope.tenant.clone(),
            principal: scope.principal.to_owned(),
            host: scope.host.to_owned(),
            account_id: account_id.to_owned(),
            authority: authority.clone(),
        };
        let slot = {
            let mut slots =
                self.sessions.0.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
            Arc::clone(slots.entry(key).or_default())
        };
        let mut cached = slot.lock().await;
        // Serialize local refreshes, then keep the same credential leased until its
        // authorization and endpoint are ready. Recheck the file even on a cache hit.
        let lease = credentials
            .lease_copilot(scope, account_id, false)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let oauth = match lease.read() {
            Ok(AccountCredential::CopilotToken(token)) if valid_token(&token) => token,
            _ => {
                *cached = None;
                return Err(AuthErrorCode::StorageUnavailable);
            }
        };
        let oauth_digest = Sha256::digest(oauth.as_bytes()).into();
        if let Some(entry) = cached.as_ref()
            && entry.oauth_digest == oauth_digest
            && now()? < entry.refresh_at
        {
            return Ok(entry.session.clone());
        }
        *cached = None;

        let public = authority == PUBLIC_AUTHORITY;
        let api = if public {
            "https://api.github.com".to_owned()
        } else {
            format!("https://{authority}/api/v3")
        };
        let (token, expires_at) = if public {
            let request =
                self.github_request(&format!("{api}/copilot_internal/v2/token"), &oauth)?;
            let subscription: CopilotSubscription =
                read_json(self.session_request(request, &mut route).await?).await?;
            if !valid_token(&subscription.token)
                || subscription.token == oauth
                || subscription.expires_at <= now()?
            {
                return Err(AuthErrorCode::UpstreamFailure);
            }
            (subscription.token, Some(subscription.expires_at))
        } else {
            // Enterprise consumes the same account's GitHub OAuth token directly.
            (oauth.clone(), None)
        };

        let mut request = self.github_request(&format!("{api}/copilot_internal/user"), &oauth)?;
        request
            .headers_mut()
            .insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
        request
            .headers_mut()
            .insert("x-github-api-version", header::HeaderValue::from_static("2025-10-01"));
        let discovery: Discovery =
            read_json(self.session_request(request, &mut route).await?).await?;
        let endpoint = session_endpoint(&authority, discovery.endpoints)?;
        let fetched_at = now()?;
        let refresh_at = match expires_at {
            Some(expiry) if expiry > fetched_at => expiry.saturating_sub(REFRESH_BUFFER_SECONDS),
            Some(_) => return Err(AuthErrorCode::UpstreamFailure),
            None => fetched_at.saturating_add(ENTERPRISE_ENDPOINT_TTL_SECONDS),
        };
        let mut authorization = header::HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| AuthErrorCode::UpstreamFailure)?;
        authorization.set_sensitive(true);
        let session = CopilotSession { endpoint, authorization };
        *cached = Some(CachedSession { oauth_digest, refresh_at, session: session.clone() });
        Ok(session)
    }

    async fn session_request(
        &self,
        mut request: Request,
        route: &mut impl FnMut(&mut Request),
    ) -> Result<Response, AuthErrorCode> {
        route(&mut request);
        let response =
            self.client.execute(request).await.map_err(|_| AuthErrorCode::UpstreamFailure)?;
        match response.status() {
            StatusCode::UNAUTHORIZED => Err(AuthErrorCode::ReauthRequired),
            StatusCode::FORBIDDEN => Err(AuthErrorCode::AuthorizationDenied),
            status if status.is_success() => Ok(response),
            _ => Err(AuthErrorCode::UpstreamFailure),
        }
    }
}

fn session_endpoint(authority: &str, discovered: Option<Endpoints>) -> Result<Url, AuthErrorCode> {
    let default = if authority == PUBLIC_AUTHORITY {
        "https://api.githubcopilot.com".to_owned()
    } else {
        format!("https://copilot-api.{authority}")
    };
    let endpoint = match discovered.as_ref() {
        Some(Endpoints { api }) => api.strip_suffix('/').unwrap_or(api),
        None => default.as_str(),
    };
    let allowed = endpoint == default
        || (authority == PUBLIC_AUTHORITY
            && matches!(
                endpoint,
                "https://api.individual.githubcopilot.com"
                    | "https://api.business.githubcopilot.com"
                    | "https://api.enterprise.githubcopilot.com"
            ));
    if !allowed {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    Url::parse(endpoint).map_err(|_| AuthErrorCode::UpstreamFailure)
}
