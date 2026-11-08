//! Execution-host adapter for GitHub Copilot device authorization.

use super::flow::{AuthAccountScope, AuthDevicePoll, AuthFlowTiming, AuthFlows};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, Request, Response, StatusCode, Url, header};
use serde::{Deserialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{
        AuthAccount, AuthCredentialState, AuthErrorCode, AuthFlow, AuthFlowId, AuthService,
        AuthStartRequest, AuthStartResponse,
    },
};

const PUBLIC_AUTHORITY: &str = "github.com";
const PUBLIC_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
const ENTERPRISE_CLIENT_ID: &str = "Ov23li8tweQw6odWQebz";
const USER_AGENT: &str = "GitHubCopilotChat/0.38.2";
const EDITOR_VERSION: &str = "vscode/1.110.1";
const PLUGIN_VERSION: &str = "copilot-chat/0.38.2";
const MAX_RESPONSE_BYTES: usize = 16 * 1024;

pub struct CopilotAuth {
    client: Client,
}

/// Only the response is intended for the Gateway. The grant stays on the execution host.
pub struct CopilotDeviceStart {
    pub response: AuthStartResponse,
    flow_id: AuthFlowId,
    pub(super) grant: Arc<Mutex<Option<CopilotDeviceGrant>>>,
}

/// Device exchange material. Kept out of serialization and debug output.
pub(super) struct CopilotDeviceGrant {
    pub authority: String,
    pub device_code: String,
}

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
struct DeviceTokenResponse {
    access_token: Option<String>,
    token_type: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct GithubUser {
    id: u64,
    login: String,
}

#[derive(Deserialize)]
struct CopilotSubscription {
    token: String,
    expires_at: i64,
}

enum Exchange {
    Pending,
    SlowDown,
    Expired,
    Authorized(GithubUser, String),
}

impl CopilotAuth {
    pub fn new() -> Result<Self, AuthErrorCode> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        Ok(Self { client })
    }

    /// The caller supplies an authorized host scope and a host-owned list of canonical
    /// enterprise authorities. Client input can select an authority from that list.
    pub async fn start(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        trusted_enterprise_authorities: &[&str],
    ) -> Result<CopilotDeviceStart, AuthErrorCode> {
        self.start_with(flows, scope, request, trusted_enterprise_authorities, |_| {}).await
    }

    /// Run a due exchange for the original host attempt. The commit must save the
    /// verified identity and OAuth token in the bound scope before returning success.
    pub async fn poll(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        started: &CopilotDeviceStart,
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            &AuthAccount,
            &str,
            &str,
        ) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        self.poll_with(flows, scope, started, Instant::now, |_| {}, persist).await
    }

    pub(super) async fn poll_with(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        started: &CopilotDeviceStart,
        clock: impl Fn() -> Instant,
        mut route: impl FnMut(&mut Request),
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            &AuthAccount,
            &str,
            &str,
        ) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        let result = flows
            .poll_device(
                scope,
                &started.flow_id,
                clock,
                || async {
                    let (authority, device_code) = match started.grant.lock() {
                        Ok(grant) => match grant.as_ref() {
                            Some(grant) => (grant.authority.clone(), grant.device_code.clone()),
                            None => return AuthDevicePoll::Complete(Err(AuthErrorCode::Conflict)),
                        },
                        Err(_) => {
                            return AuthDevicePoll::Complete(Err(
                                AuthErrorCode::ExecutionHostUnavailable,
                            ));
                        }
                    };
                    match self.exchange(&authority, &device_code, &mut route).await {
                        Ok(Exchange::Pending) => AuthDevicePoll::Pending,
                        Ok(Exchange::SlowDown) => AuthDevicePoll::SlowDown,
                        Ok(Exchange::Expired) => AuthDevicePoll::Expired,
                        Ok(Exchange::Authorized(user, token)) => {
                            let identity = format!("{authority}/{}", user.id);
                            AuthDevicePoll::Complete(
                                copilot_account(scope, authority, user.login, &identity)
                                    .map(|account| (account, (identity, token))),
                            )
                        }
                        Err(error) => AuthDevicePoll::Complete(Err(error)),
                    }
                },
                |bound, _, account, (identity, token)| persist(bound, account, &identity, &token),
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
        authority: &str,
        device_code: &str,
        route: &mut impl FnMut(&mut Request),
    ) -> Result<Exchange, AuthErrorCode> {
        let public = authority == PUBLIC_AUTHORITY;
        let client_id = if public { PUBLIC_CLIENT_ID } else { ENTERPRISE_CLIENT_ID };
        let failed = |_| AuthErrorCode::UpstreamFailure;
        let mut outbound = self
            .client
            .post(format!("https://{authority}/login/oauth/access_token"))
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .form(&[
                ("client_id", client_id),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .build()
            .map_err(failed)?;
        route(&mut outbound);
        let response = self.client.execute(outbound).await.map_err(failed)?;
        let status = response.status();
        if !(status.is_success() || status == StatusCode::BAD_REQUEST) {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let response: DeviceTokenResponse = read_json(response).await?;
        if let Some(error) = response.error {
            if response.access_token.is_some() {
                return Err(AuthErrorCode::UpstreamFailure);
            }
            return match error.as_str() {
                "authorization_pending" => Ok(Exchange::Pending),
                "slow_down" => Ok(Exchange::SlowDown),
                "expired_token" => Ok(Exchange::Expired),
                "access_denied" => Err(AuthErrorCode::AuthorizationDenied),
                _ => Err(AuthErrorCode::UpstreamFailure),
            };
        }
        let token = response
            .access_token
            .filter(|token| valid_token(token) && token != device_code)
            .ok_or(AuthErrorCode::UpstreamFailure)?;
        if !status.is_success()
            || response.token_type.is_some_and(|kind| !kind.eq_ignore_ascii_case("bearer"))
        {
            return Err(AuthErrorCode::UpstreamFailure);
        }

        let api = if public {
            "https://api.github.com".to_owned()
        } else {
            format!("https://{authority}/api/v3")
        };
        let user: GithubUser = self.github_json(&format!("{api}/user"), &token, route).await?;
        if user.id == 0
            || user.login.is_empty()
            || user.login.len() > 128
            || !user
                .login
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            || user.login.contains(&token)
            || user.login.contains(device_code)
        {
            return Err(AuthErrorCode::UpstreamFailure);
        }

        if public {
            let subscription: CopilotSubscription = self
                .github_json(&format!("{api}/copilot_internal/v2/token"), &token, route)
                .await?;
            if !valid_token(&subscription.token)
                || subscription.expires_at <= utc_now()?.0 / 1_000
                || user.login.contains(&subscription.token)
            {
                return Err(AuthErrorCode::UpstreamFailure);
            }
        }
        Ok(Exchange::Authorized(user, token))
    }

    async fn github_json<T: DeserializeOwned>(
        &self,
        url: &str,
        token: &str,
        route: &mut impl FnMut(&mut Request),
    ) -> Result<T, AuthErrorCode> {
        let failed = |_| AuthErrorCode::UpstreamFailure;
        let mut auth = header::HeaderValue::from_str(&format!("token {token}"))
            .map_err(|_| AuthErrorCode::UpstreamFailure)?;
        auth.set_sensitive(true);
        let mut outbound = self
            .client
            .get(url)
            .header(header::AUTHORIZATION, auth)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .header("Editor-Version", EDITOR_VERSION)
            .header("Editor-Plugin-Version", PLUGIN_VERSION)
            .build()
            .map_err(failed)?;
        route(&mut outbound);
        let response = self.client.execute(outbound).await.map_err(failed)?;
        if !response.status().is_success() {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        read_json(response).await
    }

    pub(super) async fn start_with(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        mut request: AuthStartRequest,
        trusted_enterprise_authorities: &[&str],
        route: impl FnOnce(&mut Request),
    ) -> Result<CopilotDeviceStart, AuthErrorCode> {
        scope.validate()?;
        if scope.service != AuthService::GithubCopilot {
            return Err(AuthErrorCode::Unsupported);
        }
        request.validate_for(scope.service)?;
        let authority = canonical_authority(request.github_domain.as_deref())?;
        let client_id = if authority == PUBLIC_AUTHORITY {
            PUBLIC_CLIENT_ID
        } else if trusted_enterprise_authorities.contains(&authority.as_str()) {
            ENTERPRISE_CLIENT_ID
        } else {
            return Err(AuthErrorCode::InvalidInput);
        };

        let upstream_failure = |_| AuthErrorCode::UpstreamFailure;
        let mut outbound = self
            .client
            .post(format!("https://{authority}/login/device/code"))
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(format!("client_id={client_id}&scope=read%3Auser"))
            .build()
            .map_err(upstream_failure)?;
        route(&mut outbound);
        let sent_at = Instant::now();
        let sent_at_utc = SystemTime::now();
        let response = self.client.execute(outbound).await.map_err(upstream_failure)?;
        if !response.status().is_success() {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let challenge: DeviceCodeResponse = read_json(response).await?;

        let verification_url = format!("https://{authority}/login/device");
        if challenge.verification_uri != verification_url
            || challenge.device_code.is_empty()
            || !challenge.device_code.bytes().all(|byte| byte.is_ascii_graphic())
            || challenge.user_code.is_empty()
            || !challenge.user_code.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || challenge.user_code == challenge.device_code
            || challenge.interval == 0
        {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let lifetime = Duration::from_secs(challenge.expires_in);
        let deadline = sent_at
            .checked_add(lifetime)
            .filter(|deadline| *deadline > Instant::now())
            .ok_or(AuthErrorCode::UpstreamFailure)?;
        let expires_at = sent_at_utc
            .checked_add(lifetime)
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
            .map(TimestampMs)
            .ok_or(AuthErrorCode::UpstreamFailure)?;

        request.github_domain = Some(authority.clone());
        let grant = Arc::new(Mutex::new(Some(CopilotDeviceGrant {
            authority,
            device_code: challenge.device_code,
        })));
        let pending_grant = Arc::clone(&grant);
        let timing =
            AuthFlowTiming { expires_at, deadline, retry_after_seconds: challenge.interval };
        let flow = flows
            .start_with_cleanup(scope, request, timing, move || {
                let mut grant =
                    pending_grant.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
                *grant = None;
                Ok(())
            })
            .map_err(|code| match code {
                AuthErrorCode::InvalidInput => AuthErrorCode::UpstreamFailure,
                other => other,
            })?;
        let AuthFlow::Pending { flow_id, expires_at, retry_after_seconds } = flow else {
            unreachable!("registering a flow always returns its pending snapshot");
        };
        let response = AuthStartResponse::Pending {
            flow_id: flow_id.clone(),
            expires_at,
            retry_after_seconds,
            verification_url,
            user_code: Some(challenge.user_code),
            can_paste: None,
        };
        Ok(CopilotDeviceStart { response, flow_id, grant })
    }
}

async fn read_json<T: DeserializeOwned>(mut response: Response) -> Result<T, AuthErrorCode> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| AuthErrorCode::UpstreamFailure)? {
        if chunk.len() > MAX_RESPONSE_BYTES - body.len() {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| AuthErrorCode::UpstreamFailure)
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RESPONSE_BYTES
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn utc_now() -> Result<TimestampMs, AuthErrorCode> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
        .map(TimestampMs)
        .ok_or(AuthErrorCode::ExecutionHostUnavailable)
}

fn copilot_account(
    scope: AuthAccountScope<'_>,
    authority: String,
    login: String,
    identity: &str,
) -> Result<AuthAccount, AuthErrorCode> {
    // The opaque ID is stable for the same verified upstream identity within this owner.
    let key = serde_json::to_vec(&(
        scope.tenant.platform_id(),
        scope.tenant.company_id(),
        scope.principal,
        scope.host,
        scope.service,
        identity,
    ))
    .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
    Ok(AuthAccount {
        account_id: URL_SAFE_NO_PAD.encode(Sha256::digest(key)),
        service: AuthService::GithubCopilot,
        label: login,
        credential_state: AuthCredentialState::Ready,
        is_default: false,
        authenticated_at: Some(utc_now()?),
        login_expires_at: None,
        github_domain: Some(authority),
        region: None,
    })
}

fn canonical_authority(input: Option<&str>) -> Result<String, AuthErrorCode> {
    let input = input.unwrap_or(PUBLIC_AUTHORITY).trim();
    let (host, port) = input.split_once(':').unwrap_or((input, "443"));
    if !port.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AuthErrorCode::InvalidInput);
    }
    let port =
        port.parse::<u16>().ok().filter(|port| *port != 0).ok_or(AuthErrorCode::InvalidInput)?;
    if !host.contains('.')
        || host.len() > 253
        || host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return Err(AuthErrorCode::InvalidInput);
    }
    let host = host.to_ascii_lowercase();
    let url =
        Url::parse(&format!("https://{host}:{port}/")).map_err(|_| AuthErrorCode::InvalidInput)?;
    if url.domain() != Some(host.as_str()) || (host == PUBLIC_AUTHORITY && port != 443) {
        return Err(AuthErrorCode::InvalidInput);
    }
    if port == 443 { Ok(host) } else { Ok(format!("{host}:{port}")) }
}
