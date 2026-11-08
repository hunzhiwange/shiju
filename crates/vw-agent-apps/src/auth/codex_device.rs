//! Execution-host adapter for Codex device authorization.

use super::{
    copilot::{read_json, utc_now, valid_token},
    credential_file::AccountCredential,
    flow::{AuthAccountScope, AuthDevicePoll, AuthFlowTiming, AuthFlows},
};
use crate::providers::codex_login_material::parse_codex_jwt_claims;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, Request, Response, StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::{
        AuthAccount, AuthCredentialState, AuthErrorCode, AuthFlow, AuthFlowId, AuthLoginMethod,
        AuthService, AuthStartRequest, AuthStartResponse,
    },
};

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const USER_AGENT: &str = "cc-switch-codex-oauth";
const USER_CODE_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/usercode";
const POLL_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/token";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const VERIFY_URL: &str = "https://auth.openai.com/codex/device";
const REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";

pub struct CodexDeviceAuth {
    client: Client,
}

/// The response is safe to return to the caller. The OAuth grant stays on the host.
pub struct CodexDeviceStart {
    pub response: AuthStartResponse,
    flow_id: AuthFlowId,
    grant: Arc<Mutex<Option<DeviceGrant>>>,
}

#[derive(Clone)]
struct DeviceGrant {
    device_auth_id: String,
    user_code: String,
}

#[derive(Deserialize)]
struct Challenge {
    device_auth_id: String,
    user_code: String,
    interval: Option<Value>,
    expires_in: Option<u64>,
}

#[derive(Deserialize)]
struct AuthorizationCode {
    authorization_code: String,
    code_verifier: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    id_token: String,
}

#[derive(Deserialize)]
struct OAuthError {
    error: String,
}

enum Exchange {
    Pending,
    Expired,
    Authorized(TokenResponse),
}

impl CodexDeviceAuth {
    pub fn new() -> Result<Self, AuthErrorCode> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .user_agent(USER_AGENT)
            .build()
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        Ok(Self { client })
    }

    pub async fn start(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
    ) -> Result<CodexDeviceStart, AuthErrorCode> {
        self.start_with(flows, scope, request, |_| {}).await
    }

    pub(super) async fn start_with(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        mut route: impl FnMut(&mut Request),
    ) -> Result<CodexDeviceStart, AuthErrorCode> {
        scope.validate()?;
        if scope.service != AuthService::CodexOauth
            || request.method != AuthLoginMethod::DeviceCode
            || request.target_account_id.is_some()
        {
            return Err(AuthErrorCode::Unsupported);
        }
        request.validate_for(scope.service)?;
        flows.reap_expired(Instant::now())?;

        let sent_at = Instant::now();
        let sent_at_utc = utc_now()?;
        let response = self
            .send(
                self.client.post(USER_CODE_URL).json(&json!({"client_id": CLIENT_ID})),
                &mut route,
            )
            .await?;
        if !response.status().is_success() {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let challenge: Challenge = read_json(response).await?;
        if !valid_token(&challenge.device_auth_id)
            || challenge.user_code.is_empty()
            || !challenge.user_code.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || challenge.user_code == challenge.device_auth_id
        {
            return Err(AuthErrorCode::UpstreamFailure);
        }

        let lifetime = Duration::from_secs(challenge.expires_in.unwrap_or(900));
        let deadline = sent_at
            .checked_add(lifetime)
            .filter(|deadline| *deadline > Instant::now())
            .ok_or(AuthErrorCode::UpstreamFailure)?;
        let expires_at = i64::try_from(lifetime.as_millis())
            .ok()
            .and_then(|millis| sent_at_utc.0.checked_add(millis))
            .map(TimestampMs)
            .ok_or(AuthErrorCode::UpstreamFailure)?;
        let interval = challenge
            .interval
            .as_ref()
            .and_then(|interval| interval.as_u64().or_else(|| interval.as_str()?.parse().ok()))
            .unwrap_or(5);
        let timing = AuthFlowTiming {
            deadline,
            expires_at,
            retry_after_seconds: interval.max(1).saturating_add(3),
        };

        let grant = Arc::new(Mutex::new(Some(DeviceGrant {
            device_auth_id: challenge.device_auth_id,
            user_code: challenge.user_code.clone(),
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
            verification_url: VERIFY_URL.into(),
            user_code: Some(challenge.user_code),
            can_paste: None,
        };
        Ok(CodexDeviceStart { response, flow_id, grant })
    }

    /// Commit only within the original flow's scope, after upstream authorization.
    pub async fn poll(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        started: &CodexDeviceStart,
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            &AuthAccount,
            &str,
            &AccountCredential,
        ) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        self.poll_with(flows, scope, started, Instant::now, |_| {}, persist).await
    }

    pub(super) async fn poll_with(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        started: &CodexDeviceStart,
        clock: impl Fn() -> Instant,
        mut route: impl FnMut(&mut Request),
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            &AuthAccount,
            &str,
            &AccountCredential,
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
                    let grant = match grant {
                        Ok(grant) => grant,
                        Err(error) => return AuthDevicePoll::Complete(Err(error)),
                    };
                    match self.exchange(&grant, &mut route).await {
                        Ok(Exchange::Pending) => AuthDevicePoll::Pending,
                        Ok(Exchange::Expired) => AuthDevicePoll::Expired,
                        Ok(Exchange::Authorized(tokens)) => {
                            AuthDevicePoll::Complete(complete_account(scope, tokens))
                        }
                        Err(error) => AuthDevicePoll::Complete(Err(error)),
                    }
                },
                |bound, _, account, (identity, credential)| {
                    persist(bound, account, &identity, &credential)
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
        grant: &DeviceGrant,
        route: &mut impl FnMut(&mut Request),
    ) -> Result<Exchange, AuthErrorCode> {
        let response = self
            .send(
                self.client.post(POLL_URL).json(&json!({
                    "device_auth_id": grant.device_auth_id,
                    "user_code": grant.user_code,
                })),
                route,
            )
            .await?;
        let code: AuthorizationCode = match response.status() {
            StatusCode::GONE => return Ok(Exchange::Expired),
            StatusCode::UNAUTHORIZED => return Err(AuthErrorCode::AuthorizationDenied),
            status if status.is_success() => read_json(response).await?,
            status => {
                if access_denied(response).await {
                    return Err(AuthErrorCode::AuthorizationDenied);
                }
                if matches!(status, StatusCode::FORBIDDEN | StatusCode::NOT_FOUND) {
                    return Ok(Exchange::Pending);
                }
                return Err(AuthErrorCode::UpstreamFailure);
            }
        };
        if !valid_token(&code.authorization_code) || !valid_token(&code.code_verifier) {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let response = self
            .send(
                self.client.post(TOKEN_URL).form(&[
                    ("grant_type", "authorization_code"),
                    ("code", &code.authorization_code),
                    ("redirect_uri", REDIRECT_URI),
                    ("client_id", CLIENT_ID),
                    ("code_verifier", &code.code_verifier),
                ]),
                route,
            )
            .await?;
        match response.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(AuthErrorCode::AuthorizationDenied)
            }
            status if status.is_success() => read_json(response).await.map(Exchange::Authorized),
            _ if access_denied(response).await => Err(AuthErrorCode::AuthorizationDenied),
            _ => Err(AuthErrorCode::UpstreamFailure),
        }
    }

    async fn send(
        &self,
        builder: reqwest::RequestBuilder,
        route: &mut impl FnMut(&mut Request),
    ) -> Result<Response, AuthErrorCode> {
        let mut request = builder.build().map_err(|_| AuthErrorCode::UpstreamFailure)?;
        route(&mut request);
        self.client.execute(request).await.map_err(|_| AuthErrorCode::UpstreamFailure)
    }
}

async fn access_denied(response: Response) -> bool {
    read_json::<OAuthError>(response).await.is_ok_and(|error| error.error == "access_denied")
}

fn workspace_id(claims: &Value) -> Option<&str> {
    claims
        .get("chatgpt_account_id")
        .and_then(Value::as_str)
        .or_else(|| claims.get("https://api.openai.com/auth")?.get("chatgpt_account_id")?.as_str())
}

fn complete_account(
    scope: AuthAccountScope<'_>,
    tokens: TokenResponse,
) -> Result<(AuthAccount, (String, AccountCredential)), AuthErrorCode> {
    let id_claims =
        parse_codex_jwt_claims(&tokens.id_token).ok_or(AuthErrorCode::UpstreamFailure)?;
    let subject = id_claims
        .get("sub")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|subject| !subject.is_empty() && !subject.chars().any(char::is_control))
        .ok_or(AuthErrorCode::UpstreamFailure)?;
    let access_claims = parse_codex_jwt_claims(&tokens.access_token);
    let workspace = workspace_id(&id_claims)
        .or_else(|| access_claims.as_ref().and_then(workspace_id))
        .ok_or(AuthErrorCode::UpstreamFailure)?;
    // A ChatGPT workspace can contain multiple users. Neither claim alone identifies an account.
    let identity =
        serde_json::to_string(&(workspace, subject)).map_err(|_| AuthErrorCode::UpstreamFailure)?;
    let scoped_identity = serde_json::to_vec(&(
        scope.tenant.platform_id(),
        scope.tenant.company_id(),
        scope.principal,
        scope.host,
        scope.service,
        &identity,
    ))
    .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
    let now = utc_now()?;
    let last_refresh = chrono::DateTime::from_timestamp_millis(now.0)
        .ok_or(AuthErrorCode::ExecutionHostUnavailable)?
        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
    let credential = AccountCredential::CodexAuth(json!({
        "auth_mode": "chatgpt",
        "last_refresh": last_refresh,
        "tokens": {
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
            "id_token": tokens.id_token,
            "account_id": workspace,
        },
    }));
    credential.validate(AppId::Codex).map_err(|_| AuthErrorCode::UpstreamFailure)?;
    let account = AuthAccount {
        account_id: URL_SAFE_NO_PAD.encode(Sha256::digest(scoped_identity)),
        service: AuthService::CodexOauth,
        label: "ChatGPT".into(),
        credential_state: AuthCredentialState::Ready,
        is_default: false,
        authenticated_at: Some(now),
        login_expires_at: None,
        github_domain: None,
        region: None,
    };
    Ok((account, (identity, credential)))
}
