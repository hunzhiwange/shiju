//! Execution-host adapter for xAI device authorization.

use super::{
    codex_device::send,
    copilot::{read_json_limited, utc_now},
    credential_file::XaiCredential,
    flow::{AuthAccountScope, AuthDevicePoll, AuthFlowTiming, AuthFlows},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, Request, Url};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{
        AuthAccount, AuthCredentialState, AuthErrorCode, AuthFlow, AuthFlowId, AuthService,
        AuthStartRequest, AuthStartResponse,
    },
};

const ISSUER: &str = "https://auth.x.ai";
const DISCOVERY_URL: &str = "https://auth.x.ai/.well-known/openid-configuration";
pub(super) const CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";
pub(super) const SCOPE: &str = "openid profile email offline_access grok-cli:access api:access";
const DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const POLL_SAFETY_MARGIN: u64 = 3;
const MAX_POLL_INTERVAL: u64 = 60;
const MAX_DEVICE_LIFETIME: u64 = 24 * 60 * 60;
pub(super) const MAX_RESPONSE_BYTES: usize = 64 * 1024;

pub struct XaiDeviceAuth {
    client: Client,
}

/// Only the challenge is public. The discovered token endpoint and grant stay on the host.
pub struct XaiDeviceStart {
    pub response: AuthStartResponse,
    flow_id: AuthFlowId,
    grant: Arc<Mutex<Option<DeviceGrant>>>,
}

#[derive(Clone)]
struct DeviceGrant {
    token_endpoint: Url,
    device_code: String,
}

#[derive(Deserialize)]
pub(super) struct Discovery {
    issuer: String,
    token_endpoint: String,
    device_authorization_endpoint: String,
}

impl Discovery {
    pub(super) fn token_endpoint(&self) -> Result<Url, AuthErrorCode> {
        trusted_url(&self.token_endpoint, &["auth.x.ai"])
    }
}

#[derive(Deserialize)]
struct Challenge {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: Option<u64>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    id_token: Option<String>,
    expires_in: Option<i64>,
}

#[derive(Deserialize)]
struct Claims {
    sub: Option<String>,
    email: Option<String>,
    preferred_username: Option<String>,
    name: Option<String>,
}

impl XaiDeviceAuth {
    pub fn new() -> Result<Self, AuthErrorCode> {
        Ok(Self { client: oauth_client()? })
    }

    pub async fn start(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
    ) -> Result<XaiDeviceStart, AuthErrorCode> {
        self.start_with(flows, scope, request, |_| {}).await
    }

    pub(super) async fn start_with(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        mut route: impl FnMut(&mut Request),
    ) -> Result<XaiDeviceStart, AuthErrorCode> {
        scope.validate()?;
        if scope.service != AuthService::XaiOauth {
            return Err(AuthErrorCode::Unsupported);
        }
        request.validate_for(scope.service)?;
        flows.reap_expired(Instant::now())?;

        let discovery = discover(&self.client, &mut route).await?;
        let token_endpoint = discovery.token_endpoint()?;
        let device_endpoint =
            trusted_url(&discovery.device_authorization_endpoint, &["auth.x.ai"])?;

        let sent_at = Instant::now();
        let sent_at_utc = utc_now()?;
        let response = send(
            &self.client,
            self.client.post(device_endpoint).form(&[("client_id", CLIENT_ID), ("scope", SCOPE)]),
            &mut route,
        )
        .await?;
        if !response.status().is_success() {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let challenge: Challenge = read_json_limited(response, MAX_RESPONSE_BYTES).await?;
        if !valid_token(&challenge.device_code)
            || !valid_token(&challenge.user_code)
            || !challenge.user_code.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || challenge.user_code == challenge.device_code
        {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let verification_url = trusted_url(
            challenge.verification_uri_complete.as_deref().unwrap_or(&challenge.verification_uri),
            &["auth.x.ai", "accounts.x.ai"],
        )?;
        if verification_url.as_str().contains(&challenge.device_code)
            || verification_url
                .query_pairs()
                .any(|(key, value)| key == "device_code" || value.contains(&challenge.device_code))
        {
            return Err(AuthErrorCode::UpstreamFailure);
        }

        let lifetime = Duration::from_secs(challenge.expires_in.clamp(1, MAX_DEVICE_LIFETIME));
        let deadline = sent_at + lifetime;
        if deadline <= Instant::now() {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let timing = AuthFlowTiming {
            deadline,
            expires_at: TimestampMs(sent_at_utc.0.saturating_add(lifetime.as_millis() as i64)),
            retry_after_seconds: challenge.interval.unwrap_or(5).clamp(1, MAX_POLL_INTERVAL)
                + POLL_SAFETY_MARGIN,
        };
        let grant = Arc::new(Mutex::new(Some(DeviceGrant {
            token_endpoint,
            device_code: challenge.device_code,
        })));
        let cleanup = Arc::clone(&grant);
        let flow = flows.start_with_cleanup(scope, request, timing, move || {
            *cleanup.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)? = None;
            Ok(())
        })?;
        let AuthFlow::Pending { flow_id, expires_at, retry_after_seconds } = flow else {
            unreachable!("registering a flow always returns its pending snapshot");
        };
        let response = AuthStartResponse::Pending {
            flow_id: flow_id.clone(),
            expires_at,
            retry_after_seconds,
            verification_url: verification_url.into(),
            user_code: Some(challenge.user_code),
            can_paste: None,
        };
        Ok(XaiDeviceStart { response, flow_id, grant })
    }

    /// Commit the credential only while its original scoped flow is still pending.
    pub async fn poll(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        started: &XaiDeviceStart,
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            &AuthAccount,
            &str,
            XaiCredential,
        ) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        self.poll_with(flows, scope, started, Instant::now, |_| {}, persist).await
    }

    pub(super) async fn poll_with(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        started: &XaiDeviceStart,
        clock: impl Fn() -> Instant,
        mut route: impl FnMut(&mut Request),
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            &AuthAccount,
            &str,
            XaiCredential,
        ) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        let result = flows
            .poll_device(
                scope,
                &started.flow_id,
                clock,
                || async {
                    let grant = match started.grant.lock() {
                        Ok(grant) => grant.clone().ok_or(AuthErrorCode::Conflict),
                        Err(_) => Err(AuthErrorCode::ExecutionHostUnavailable),
                    };
                    match grant {
                        Ok(grant) => self
                            .exchange(scope, grant, &mut route)
                            .await
                            .unwrap_or_else(|error| AuthDevicePoll::Complete(Err(error))),
                        Err(error) => AuthDevicePoll::Complete(Err(error)),
                    }
                },
                |bound, _, account, (identity, credential)| {
                    persist(bound, account, &identity, credential)
                },
            )
            .await?;
        if !matches!(result, AuthFlow::Pending { .. })
            && let Ok(mut grant) = started.grant.lock()
        {
            *grant = None;
        }
        Ok(result)
    }

    async fn exchange(
        &self,
        scope: AuthAccountScope<'_>,
        grant: DeviceGrant,
        route: &mut impl FnMut(&mut Request),
    ) -> Result<AuthDevicePoll<(String, XaiCredential)>, AuthErrorCode> {
        let response = send(
            &self.client,
            self.client.post(grant.token_endpoint).form(&[
                ("grant_type", DEVICE_GRANT_TYPE),
                ("client_id", CLIENT_ID),
                ("device_code", &grant.device_code),
            ]),
            route,
        )
        .await?;
        let status = response.status();
        let payload: serde_json::Value = read_json_limited(response, MAX_RESPONSE_BYTES).await?;
        match payload.get("error").and_then(serde_json::Value::as_str) {
            Some("authorization_pending") => Ok(AuthDevicePoll::Pending),
            Some("slow_down") => {
                Ok(AuthDevicePoll::SlowDownCapped(MAX_POLL_INTERVAL + POLL_SAFETY_MARGIN))
            }
            Some("expired_token") => Ok(AuthDevicePoll::Expired),
            Some("access_denied") => Err(AuthErrorCode::AuthorizationDenied),
            Some(_) => Err(AuthErrorCode::UpstreamFailure),
            None if status.is_success() => {
                let tokens =
                    serde_json::from_value(payload).map_err(|_| AuthErrorCode::UpstreamFailure)?;
                Ok(AuthDevicePoll::Complete(complete_account(scope, tokens)))
            }
            None => Err(AuthErrorCode::UpstreamFailure),
        }
    }
}

pub(super) fn oauth_client() -> Result<Client, AuthErrorCode> {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .user_agent("cc-switch-xai-oauth")
        .build()
        .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)
}

pub(super) async fn discover(
    client: &Client,
    route: &mut impl FnMut(&mut Request),
) -> Result<Discovery, AuthErrorCode> {
    let response = send(client, client.get(DISCOVERY_URL), route).await?;
    if !response.status().is_success() {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    let discovery: Discovery = read_json_limited(response, MAX_RESPONSE_BYTES).await?;
    if discovery.issuer.trim_end_matches('/') != ISSUER {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    Ok(discovery)
}

fn trusted_url(value: &str, allowed_hosts: &[&str]) -> Result<Url, AuthErrorCode> {
    let url = Url::parse(value).map_err(|_| AuthErrorCode::UpstreamFailure)?;
    if url.scheme() != "https"
        || !url.host_str().is_some_and(|host| allowed_hosts.contains(&host))
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    Ok(url)
}

pub(super) fn valid_token(token: &str) -> bool {
    !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_graphic())
}

fn claims(token: &str) -> Option<Claims> {
    let mut segments = token.split('.');
    let (header, payload, signature) = (segments.next()?, segments.next()?, segments.next()?);
    if header.is_empty() || signature.is_empty() || segments.next().is_some() {
        return None;
    }
    let payload = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&payload).ok()
}

fn complete_account(
    scope: AuthAccountScope<'_>,
    tokens: TokenResponse,
) -> Result<(AuthAccount, (String, XaiCredential)), AuthErrorCode> {
    if !valid_token(&tokens.access_token) || !valid_token(&tokens.refresh_token) {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    // These claims come from the host's TLS token exchange, never from caller-provided tokens.
    let claims = tokens
        .id_token
        .as_deref()
        .and_then(claims)
        .or_else(|| claims(&tokens.access_token))
        .ok_or(AuthErrorCode::UpstreamFailure)?;
    let subject = claims
        .sub
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.chars().any(char::is_control))
        .ok_or(AuthErrorCode::UpstreamFailure)?;
    let key = serde_json::to_vec(&(
        scope.tenant.platform_id(),
        scope.tenant.company_id(),
        scope.principal,
        scope.host,
        scope.service,
        subject,
    ))
    .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
    let label = claims
        .email
        .or(claims.preferred_username)
        .or(claims.name)
        .filter(|label| {
            !label.trim().is_empty() && label.len() <= 256 && !label.chars().any(char::is_control)
        })
        .unwrap_or_else(|| "xAI".into());
    let authenticated_at = utc_now()?;
    let lifetime = tokens.expires_in.unwrap_or(3_600).max(1);
    let credential = XaiCredential {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_at: TimestampMs(authenticated_at.0.saturating_add(lifetime.saturating_mul(1_000))),
        requires_reauth: false,
    };
    let account = AuthAccount {
        account_id: URL_SAFE_NO_PAD.encode(Sha256::digest(key)),
        service: AuthService::XaiOauth,
        label,
        credential_state: AuthCredentialState::Ready,
        is_default: false,
        authenticated_at: Some(authenticated_at),
        login_expires_at: None,
        github_domain: None,
        region: None,
    };
    Ok((account, (subject.into(), credential)))
}
