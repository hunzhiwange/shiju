//! DeepSeek Harness browser authorization on the authorized execution host.

use super::{
    browser_callback::{LoopbackCallback, PreparedCallback},
    copilot::{read_json_limited, utc_now, valid_token},
    flow::{AuthAccountScope, AuthFlowTiming, AuthFlows},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, Request, Url};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{
        AuthAccount, AuthCredentialState, AuthErrorCode, AuthFlow, AuthFlowId, AuthService,
        AuthStartRequest, AuthStartResponse,
    },
};

const PLATFORM_ORIGIN: &str = "https://platform.deepseek.com";
const CALLBACK_PATH: &str = "/oauth/callback";
const LOGIN_LIFETIME: Duration = Duration::from_secs(600);
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// Trusted execution-host values. The device ID belongs to this DSH installation.
pub struct DshDevice {
    pub id: Uuid,
    pub locale: String,
    pub timezone_offset_seconds: i32,
}

impl DshDevice {
    fn wire_locale(&self) -> &'static str {
        if self.locale.starts_with("zh") { "zh_CN" } else { "en_US" }
    }
}

/// Host-private material handed to the scoped grant writer.
pub struct DshGrant {
    token: String,
}

impl DshGrant {
    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn issuer(&self) -> &'static str {
        PLATFORM_ORIGIN
    }
}

#[derive(Deserialize)]
struct Init {
    authorize_url: String,
    authorize_id: String,
    expires_in: Option<f64>,
}

#[derive(Deserialize)]
struct Exchange {
    token: String,
    issuer: Option<String>,
}

struct PendingCancel {
    authorize_id: String,
    verifier: String,
}

/// One DSH browser attempt. Only `response` is returned to the browser client.
pub struct DshLogin {
    pub response: AuthStartResponse,
    flow_id: AuthFlowId,
    redirect_uri: String,
    callback: LoopbackCallback,
    client: Client,
    device: Arc<DshDevice>,
    pending_cancel: Arc<Mutex<Option<PendingCancel>>>,
    polling: tokio::sync::Mutex<()>,
}

impl DshLogin {
    /// The host determines whether the browser can reach its loopback listener.
    pub async fn start(
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        browser_on_execution_host: bool,
        device: DshDevice,
    ) -> Result<Self, AuthErrorCode> {
        Self::start_with(flows, scope, request, browser_on_execution_host, device, |_| {}).await
    }

    pub(super) async fn start_with(
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        browser_on_execution_host: bool,
        device: DshDevice,
        route: impl Fn(&mut Request) + Clone + Send + Sync + 'static,
    ) -> Result<Self, AuthErrorCode> {
        scope.validate()?;
        if scope.service != AuthService::DshAccount {
            return Err(AuthErrorCode::Unsupported);
        }
        request.validate_for(scope.service)?;
        if !browser_on_execution_host {
            return Err(AuthErrorCode::CallbackUnreachable);
        }

        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        let (prepared, challenge) = PreparedCallback::bind(0)?;
        let redirect_uri = format!("http://127.0.0.1:{}{CALLBACK_PATH}", challenge.port);
        let init = platform_call(
            &client,
            &device,
            "auth_init",
            json!({
                "code_challenge": challenge.challenge,
                "code_challenge_method": "S256",
                "state": challenge.state,
                "redirect_uri": redirect_uri,
                "locale": device.wire_locale(),
                "login_source": "web",
            }),
            &route,
        )
        .await?;
        let init: Init =
            serde_json::from_value(init).map_err(|_| AuthErrorCode::UpstreamFailure)?;
        let verification_url = authorization_url(&init.authorize_url)?;
        if !valid_token(&init.authorize_id) {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let lifetime =
            init.expires_in.filter(|seconds| *seconds > 0.0).map_or(LOGIN_LIFETIME, |seconds| {
                Duration::from_secs_f64(seconds.min(LOGIN_LIFETIME.as_secs_f64()))
            });
        let deadline = Instant::now() + lifetime;
        let expires_at = TimestampMs(utc_now()?.0 + lifetime.as_millis() as i64);
        let pending_cancel = Arc::new(Mutex::new(Some(PendingCancel {
            authorize_id: init.authorize_id,
            verifier: prepared.verifier().to_owned(),
        })));
        let callback = prepared.start(CALLBACK_PATH, "dsh-oauth-callback", deadline)?;
        let device = Arc::new(device);
        let mut stop_listener = callback.cleanup();
        let cancel_attempt = Arc::clone(&pending_cancel);
        let cancel_client = client.clone();
        let cancel_device = Arc::clone(&device);
        let runtime = tokio::runtime::Handle::current();
        let cleanup = move || {
            stop_listener()?;
            let attempt = take_cancel(&cancel_attempt)?;
            if let Some(attempt) = attempt {
                // Local cancellation must succeed even if Platform is unavailable.
                let client = cancel_client.clone();
                let device = Arc::clone(&cancel_device);
                let route = route.clone();
                runtime.spawn(async move {
                    let _ = platform_call(
                        &client,
                        &device,
                        "auth_cancel",
                        json!({"authorize_id": attempt.authorize_id, "code_verifier": attempt.verifier}),
                        route,
                    )
                    .await;
                });
            }
            Ok(())
        };
        let flow = flows.replace_with_cleanup(
            scope,
            request,
            AuthFlowTiming { deadline, expires_at, retry_after_seconds: 1 },
            cleanup,
        )?;
        let AuthFlow::Pending { flow_id, expires_at, retry_after_seconds } = flow else {
            unreachable!("registering a flow always returns its pending snapshot");
        };
        let response = AuthStartResponse::Pending {
            flow_id: flow_id.clone(),
            expires_at,
            retry_after_seconds,
            verification_url,
            user_code: None,
            can_paste: None,
        };
        Ok(Self {
            response,
            flow_id,
            redirect_uri,
            callback,
            client,
            device,
            pending_cancel,
            polling: tokio::sync::Mutex::new(()),
        })
    }

    /// The caller must own the native grant slot. The writer runs only while
    /// this flow is eligible to commit under its original scope.
    pub async fn poll(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        persist: impl FnOnce(AuthAccountScope<'_>, &AuthAccount, &DshGrant) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        self.poll_with(flows, scope, |_| {}, persist).await
    }

    pub(super) async fn poll_with(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        route: impl FnOnce(&mut Request),
        persist: impl FnOnce(AuthAccountScope<'_>, &AuthAccount, &DshGrant) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        let Ok(_poll) = self.polling.try_lock() else {
            return flows.snapshot(scope, &self.flow_id, Instant::now);
        };
        let snapshot = flows.snapshot(scope, &self.flow_id, Instant::now)?;
        if !matches!(snapshot, AuthFlow::Pending { .. }) {
            return Ok(snapshot);
        }
        let Some(callback) = self.callback.take()? else {
            return Ok(snapshot);
        };
        let result = match callback {
            Ok(grant) => self.exchange(grant.code, grant.verifier, route).await,
            Err(error) => Err(error),
        };
        let completed = match result {
            Ok(grant) => flows.complete(
                scope,
                &self.flow_id,
                Instant::now,
                account(scope),
                |bound, _, account| persist(bound, account, &grant),
            ),
            Err(error) => {
                flows.complete(scope, &self.flow_id, Instant::now, Err(error), |_, _, _| Ok(()))
            }
        };
        match completed {
            Ok(snapshot) => {
                take_cancel(&self.pending_cancel)?;
                Ok(snapshot)
            }
            Err(AuthErrorCode::Conflict) => {
                let snapshot = flows.snapshot(scope, &self.flow_id, Instant::now)?;
                if matches!(snapshot, AuthFlow::Pending { .. }) {
                    return Err(AuthErrorCode::Conflict);
                }
                Ok(snapshot)
            }
            Err(error) => Err(error),
        }
    }

    pub fn cancel(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
    ) -> Result<(), AuthErrorCode> {
        flows.cancel(scope, &self.flow_id, Instant::now)
    }

    async fn exchange(
        &self,
        code: String,
        verifier: String,
        route: impl FnOnce(&mut Request),
    ) -> Result<DshGrant, AuthErrorCode> {
        let arch = if std::env::consts::ARCH == "aarch64" { "arm64" } else { "x64" };
        let release = Command::new("/usr/bin/uname")
            .arg("-r")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .ok_or(AuthErrorCode::ExecutionHostUnavailable)?;
        let body = platform_call(
            &self.client,
            &self.device,
            "auth_exchange",
            json!({
                "code": code,
                "code_verifier": verifier,
                "redirect_uri": self.redirect_uri,
                "device_id": self.device.id.to_string(),
                "device_model": format!("darwin-{arch}"),
                "os_version": format!("darwin {}", release.trim()),
            }),
            route,
        )
        .await?;
        let grant: Exchange =
            serde_json::from_value(body).map_err(|_| AuthErrorCode::UpstreamFailure)?;
        if !valid_token(&grant.token)
            || grant.issuer.is_some_and(|issuer| issuer.trim_end_matches('/') != PLATFORM_ORIGIN)
        {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        Ok(DshGrant { token: grant.token })
    }
}

fn take_cancel(
    attempt: &Mutex<Option<PendingCancel>>,
) -> Result<Option<PendingCancel>, AuthErrorCode> {
    Ok(attempt.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?.take())
}

fn authorization_url(value: &str) -> Result<String, AuthErrorCode> {
    let url = Url::parse(value).map_err(|_| AuthErrorCode::UpstreamFailure)?;
    if url.origin().ascii_serialization() != PLATFORM_ORIGIN
        || url.path() != "/dsh/authorize"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    Ok(url.into())
}

fn account(scope: AuthAccountScope<'_>) -> Result<AuthAccount, AuthErrorCode> {
    let identity = serde_json::to_vec(&(
        scope.tenant,
        scope.principal,
        scope.host,
        scope.service,
        "deepseek-account-platform/default",
    ))
    .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
    Ok(AuthAccount {
        account_id: URL_SAFE_NO_PAD.encode(Sha256::digest(identity)),
        service: scope.service,
        label: "DeepSeek".into(),
        credential_state: AuthCredentialState::Ready,
        is_default: false,
        authenticated_at: Some(utc_now()?),
        login_expires_at: None,
        github_domain: None,
        region: None,
    })
}

async fn platform_call(
    client: &Client,
    device: &DshDevice,
    method: &str,
    body: Value,
    route: impl FnOnce(&mut Request),
) -> Result<Value, AuthErrorCode> {
    // The three login RPCs carry their PKCE material in JSON. Authenticated
    // account operations use the grant as x-dsh-auth-token and are separate.
    let mut request = client
        .post(format!("{PLATFORM_ORIGIN}/auth-api/v0/dsh/{method}"))
        .header("x-client-platform", "web")
        .header("x-client-bundle-id", "")
        .header("x-client-locale", device.wire_locale())
        .header("x-client-timezone-offset", device.timezone_offset_seconds)
        .json(&body)
        .build()
        .map_err(|_| AuthErrorCode::UpstreamFailure)?;
    route(&mut request);
    let response = client.execute(request).await.map_err(|_| AuthErrorCode::UpstreamFailure)?;
    if !response.status().is_success() {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    let body: Value = read_json_limited(response, MAX_RESPONSE_BYTES).await?;
    if body["code"].as_i64() != Some(0) {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    Ok(body["data"]["biz_data"].clone())
}
