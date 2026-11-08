//! Execution-host adapter for Kimi's regional device authorization.

use super::{
    codex_device::send,
    copilot::{read_json_limited, utc_now},
    credential_file::KimiCredential,
    flow::{AuthAccountScope, AuthDevicePoll, AuthFlowTiming, AuthFlows},
    xai::valid_token,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, Request, RequestBuilder, Url};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{
        AuthAccount, AuthCredentialState, AuthErrorCode, AuthFlow, AuthFlowId, AuthRegion,
        AuthService, AuthStartRequest, AuthStartResponse,
    },
    id::TenantScope,
};

const CLIENT_ID: &str = "17e5f671-d194-4dfb-9706-5516cb48c098";
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_TOKEN_LIFETIME: u64 = 31_536_000;
const MAX_FLOW_LIFETIME: u64 = 86_400;
const MAX_INTERVAL: u64 = 300;

type ScopeKey = (TenantScope, String, String);
type StartSlot = Arc<AsyncMutex<Option<AuthFlowId>>>;

/// Keep one adapter alongside the host's flow registry. Starts serialize only
/// with other starts from the same owner and execution host.
pub struct KimiDeviceAuth {
    client: Client,
    starts: Mutex<HashMap<ScopeKey, StartSlot>>,
}

pub struct KimiDeviceStart {
    pub response: AuthStartResponse,
    flow_id: AuthFlowId,
    grant: Arc<Mutex<Option<DeviceGrant>>>,
}

/// Resolved by the host from an existing account in the authorized scope.
pub struct KimiLoginTarget {
    pub region: AuthRegion,
    pub device_id: String,
}

#[derive(Clone)]
struct DeviceGrant {
    region: AuthRegion,
    device_id: String,
    code: String,
}

struct Endpoints {
    oauth: &'static str,
    api: &'static str,
    domain: &'static str,
    identity_prefix: &'static str,
}

fn endpoints(region: AuthRegion) -> Endpoints {
    match region {
        AuthRegion::MainlandCn => Endpoints {
            oauth: "https://auth.kimi.com",
            api: "https://api.kimi.com/coding/v1",
            domain: "kimi.com",
            identity_prefix: "mainland-cn",
        },
        AuthRegion::Global => Endpoints {
            oauth: "https://auth.kimi.ai",
            api: "https://api.kimi.ai/coding/v1",
            domain: "kimi.ai",
            identity_prefix: "global",
        },
    }
}

impl KimiDeviceAuth {
    pub fn new() -> Result<Self, AuthErrorCode> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("shiju/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        Ok(Self { client, starts: Mutex::default() })
    }

    pub async fn start(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        target: impl FnOnce(&str) -> Result<KimiLoginTarget, AuthErrorCode>,
    ) -> Result<KimiDeviceStart, AuthErrorCode> {
        self.start_with(flows, scope, request, target, |_| {}).await
    }

    pub(super) async fn start_with(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        mut request: AuthStartRequest,
        target: impl FnOnce(&str) -> Result<KimiLoginTarget, AuthErrorCode>,
        mut route: impl FnMut(&mut Request),
    ) -> Result<KimiDeviceStart, AuthErrorCode> {
        scope.validate()?;
        if scope.service != AuthService::KimiOauth {
            return Err(AuthErrorCode::Unsupported);
        }
        request.validate_for(scope.service)?;
        let (region, device_id) = if let Some(id) = &request.target_account_id {
            crate::providers::validate_identity(id).map_err(|_| AuthErrorCode::InvalidInput)?;
            let saved = target(id)?;
            if request.region.is_some_and(|region| region != saved.region) {
                return Err(AuthErrorCode::IdentityMismatch);
            }
            if Uuid::parse_str(&saved.device_id).is_err() {
                return Err(AuthErrorCode::StorageUnavailable);
            }
            (saved.region, saved.device_id)
        } else {
            (request.region.ok_or(AuthErrorCode::InvalidInput)?, Uuid::new_v4().to_string())
        };
        request.region = Some(region);

        let key = (scope.tenant.clone(), scope.principal.into(), scope.host.into());
        let slot = self
            .starts
            .lock()
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?
            .entry(key)
            .or_default()
            .clone();
        let mut previous = slot.lock().await;
        flows.reap_expired(Instant::now())?;
        if let Some(flow_id) = &*previous {
            // A flow that finished while the new request was being validated is already terminal.
            match flows.cancel(scope, flow_id, Instant::now) {
                Ok(()) | Err(AuthErrorCode::Conflict) => {}
                Err(error) => return Err(error),
            }
        }
        *previous = None;

        let endpoints = endpoints(region);
        let sent_at = Instant::now();
        let sent_at_utc = utc_now()?;
        let response = send(
            &self.client,
            identify(
                self.client.post(format!("{}/api/oauth/device_authorization", endpoints.oauth)),
                &device_id,
            )
            .form(&[("client_id", CLIENT_ID)]),
            &mut route,
        )
        .await?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let challenge: Value = read_json_limited(response, MAX_RESPONSE_BYTES).await?;
        let code = token(&challenge, "device_code")?;
        let user_code = token(&challenge, "user_code")?;
        let uri = challenge["verification_uri_complete"]
            .as_str()
            .filter(|url| !url.is_empty())
            .or_else(|| challenge["verification_uri"].as_str())
            .ok_or(AuthErrorCode::UpstreamFailure)?;
        let verification_url = verification_url(uri, &endpoints, code)?;
        if user_code.contains(code)
            || user_code.contains(&device_id)
            || verification_url.as_str().contains(&device_id)
        {
            return Err(AuthErrorCode::UpstreamFailure);
        }

        let lifetime = seconds(&challenge, "expires_in", Some(600), MAX_FLOW_LIFETIME)?;
        let retry_after_seconds = seconds(&challenge, "interval", Some(5), MAX_INTERVAL)?;
        let deadline = sent_at + Duration::from_secs(lifetime);
        if deadline <= Instant::now() {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let timing = AuthFlowTiming {
            deadline,
            expires_at: TimestampMs(sent_at_utc.0 + (lifetime as i64) * 1_000),
            retry_after_seconds,
        };
        let grant =
            Arc::new(Mutex::new(Some(DeviceGrant { region, device_id, code: code.into() })));
        let cleanup = Arc::clone(&grant);
        let flow = flows.start_with_cleanup(scope, request, timing, move || {
            *cleanup.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)? = None;
            Ok(())
        })?;
        let AuthFlow::Pending { flow_id, expires_at, retry_after_seconds } = flow else {
            unreachable!("registering a flow always returns its pending snapshot");
        };
        *previous = Some(flow_id.clone());
        let response = AuthStartResponse::Pending {
            flow_id: flow_id.clone(),
            expires_at,
            retry_after_seconds,
            verification_url: verification_url.into(),
            user_code: Some(user_code.into()),
            can_paste: None,
        };
        Ok(KimiDeviceStart { response, flow_id, grant })
    }

    pub async fn poll(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        started: &KimiDeviceStart,
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            Option<&str>,
            &AuthAccount,
            &str,
            KimiCredential,
        ) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        self.poll_with(flows, scope, started, Instant::now, |_| {}, persist).await
    }

    pub(super) async fn poll_with(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        started: &KimiDeviceStart,
        clock: impl Fn() -> Instant,
        mut route: impl FnMut(&mut Request),
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            Option<&str>,
            &AuthAccount,
            &str,
            KimiCredential,
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
                    let result = match grant {
                        Ok(grant) => self.exchange(scope, grant, &mut route).await,
                        Err(error) => Err(error),
                    };
                    result.unwrap_or_else(|error| AuthDevicePoll::Complete(Err(error)))
                },
                |bound, request, account, (identity, credential)| {
                    persist(
                        bound,
                        request.target_account_id.as_deref(),
                        account,
                        &identity,
                        credential,
                    )
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
    ) -> Result<AuthDevicePoll<(String, KimiCredential)>, AuthErrorCode> {
        let endpoints = endpoints(grant.region);
        let response = send(
            &self.client,
            identify(
                self.client.post(format!("{}/api/oauth/token", endpoints.oauth)),
                &grant.device_id,
            )
            .form(&[
                ("client_id", CLIENT_ID),
                ("device_code", &grant.code),
                ("grant_type", DEVICE_GRANT),
            ]),
            route,
        )
        .await?;
        let status = response.status();
        let payload: Value = read_json_limited(response, MAX_RESPONSE_BYTES).await?;
        if status != reqwest::StatusCode::OK {
            return match payload["error"].as_str() {
                Some("authorization_pending") => Ok(AuthDevicePoll::Pending),
                Some("slow_down") => Ok(AuthDevicePoll::SlowDown),
                Some("expired_token") => Ok(AuthDevicePoll::Expired),
                Some("access_denied") => Err(AuthErrorCode::AuthorizationDenied),
                _ => Err(AuthErrorCode::UpstreamFailure),
            };
        }
        let access_token = token(&payload, "access_token")?;
        let refresh_token = token(&payload, "refresh_token")?;
        let lifetime = seconds(&payload, "expires_in", None, MAX_TOKEN_LIFETIME)?;
        let authenticated_at = utc_now()?;
        let response = send(
            &self.client,
            identify(self.client.get(format!("{}/me", endpoints.api)), &grant.device_id)
                .bearer_auth(access_token),
            route,
        )
        .await?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        let me: Value = read_json_limited(response, MAX_RESPONSE_BYTES).await?;
        let subject = identity_field(&me["user_id"])
            .or_else(|| identity_field(&me["id"]))
            .ok_or(AuthErrorCode::UpstreamFailure)?;
        let identity = format!("{}:{subject}", endpoints.identity_prefix);
        let scoped_identity = serde_json::to_vec(&(
            scope.tenant.platform_id(),
            scope.tenant.company_id(),
            scope.principal,
            scope.host,
            scope.service,
            &identity,
        ))
        .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        let label = identity_field(&me["nickname"])
            .filter(|name| {
                [access_token, refresh_token, &grant.device_id, &grant.code]
                    .iter()
                    .all(|secret| !name.contains(secret))
            })
            .unwrap_or_else(|| "Kimi".into());
        let account = AuthAccount {
            account_id: URL_SAFE_NO_PAD.encode(Sha256::digest(scoped_identity)),
            service: AuthService::KimiOauth,
            label,
            credential_state: AuthCredentialState::Ready,
            is_default: false,
            authenticated_at: Some(authenticated_at),
            login_expires_at: None,
            github_domain: None,
            region: Some(grant.region),
        };
        let credential = KimiCredential {
            access_token: access_token.into(),
            refresh_token: refresh_token.into(),
            expires_at: TimestampMs(authenticated_at.0 + (lifetime as i64) * 1_000),
            region: grant.region,
            device_id: grant.device_id,
        };
        Ok(AuthDevicePoll::Complete(Ok((account, (identity, credential)))))
    }
}

fn identify(request: RequestBuilder, device_id: &str) -> RequestBuilder {
    request
        .header("Accept", "application/json")
        .header("X-Msh-Platform", "kimi_code_cli")
        .header("X-Msh-Version", env!("CARGO_PKG_VERSION"))
        .header("X-Msh-Device-Id", device_id)
        .header("X-Msh-Device-Name", "ShiJu")
        .header(
            "X-Msh-Device-Model",
            format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        )
        .header("X-Msh-Os-Version", std::env::consts::OS)
}

fn token<'a>(payload: &'a Value, field: &str) -> Result<&'a str, AuthErrorCode> {
    payload[field].as_str().filter(|value| valid_token(value)).ok_or(AuthErrorCode::UpstreamFailure)
}

fn seconds(
    payload: &Value,
    field: &str,
    default: Option<u64>,
    max: u64,
) -> Result<u64, AuthErrorCode> {
    let value = match payload.get(field) {
        Some(value) => value.as_u64().or_else(|| value.as_str()?.parse().ok()),
        None => default,
    };
    value.filter(|value| (1..=max).contains(value)).ok_or(AuthErrorCode::UpstreamFailure)
}

fn verification_url(uri: &str, endpoints: &Endpoints, grant: &str) -> Result<Url, AuthErrorCode> {
    let url = Url::parse(uri).map_err(|_| AuthErrorCode::UpstreamFailure)?;
    if url.scheme() != "https"
        || !url.host_str().is_some_and(|host| {
            host == endpoints.domain
                || host
                    .strip_suffix(endpoints.domain)
                    .is_some_and(|prefix| matches!(prefix, "auth." | "www." | "code."))
        })
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.as_str().contains(grant)
        || url.query_pairs().any(|(key, value)| key == "device_code" || value.contains(grant))
    {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    Ok(url)
}

fn identity_field(value: &Value) -> Option<String> {
    match value {
        Value::Number(number) => Some(number.to_string()),
        Value::String(value) => {
            let value = value.trim();
            (!value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control))
                .then(|| value.to_owned())
        }
        _ => None,
    }
}
