//! Execution-host adapter for GitHub Copilot device authorization.

use super::flow::{AuthAccountScope, AuthFlowTiming, AuthFlows};
use reqwest::{Client, Request, Url, header};
use serde::Deserialize;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthErrorCode, AuthFlow, AuthService, AuthStartRequest, AuthStartResponse},
};

const PUBLIC_AUTHORITY: &str = "github.com";
const PUBLIC_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
const ENTERPRISE_CLIENT_ID: &str = "Ov23li8tweQw6odWQebz";
const USER_AGENT: &str = "GitHubCopilotChat/0.38.2";
const MAX_CHALLENGE_BYTES: usize = 16 * 1024;

pub struct CopilotAuth {
    client: Client,
}

/// Only the response is intended for the Gateway. The grant stays on the execution host.
pub struct CopilotDeviceStart {
    pub response: AuthStartResponse,
    pub grant: Arc<Mutex<Option<CopilotDeviceGrant>>>,
}

/// Device exchange material. Kept out of serialization and debug output.
pub struct CopilotDeviceGrant {
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
        let mut response = self.client.execute(outbound).await.map_err(upstream_failure)?;
        if !response.status().is_success() {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(upstream_failure)? {
            if chunk.len() > MAX_CHALLENGE_BYTES - body.len() {
                return Err(AuthErrorCode::UpstreamFailure);
            }
            body.extend_from_slice(&chunk);
        }
        let challenge: DeviceCodeResponse =
            serde_json::from_slice(&body).map_err(|_| AuthErrorCode::UpstreamFailure)?;

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
            flow_id,
            expires_at,
            retry_after_seconds,
            verification_url,
            user_code: Some(challenge.user_code),
            can_paste: None,
        };
        Ok(CopilotDeviceStart { response, grant })
    }
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
