//! Official Grok billing for the authorized CLI slot or a managed xAI account.

mod grpc;

use std::{io::Read, path::Path};

use chrono::{DateTime, NaiveDateTime, SecondsFormat};
use reqwest::{ClientBuilder, RequestBuilder, header::HeaderValue};
use serde::Deserialize;
use serde_json::{Map, Value};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthErrorCode, AuthService},
    app_providers::paths::ConfigPathState,
    apps_usage::{
        UsageCredentialState, UsageExtraUsage, UsageNumber, UsageQueryData, UsageQueryError,
        UsageQueryErrorCode, UsageQueryKind, UsageQueryOutcome, UsageQueryResult, UsageQuotaWindow,
        UsageSource, UsageSubscription, UsageWallet,
    },
};

use crate::{
    auth::{credential_file::CredentialFileStore, xai_refresh::XaiRefresh},
    providers::{paths::open_beneath, validation::header_value},
    usage::{
        binding::{UsageAccount, UsageCredentialRef},
        coding::zhipu::{client_builder, read_json_response, transport_error},
    },
};

const MAX_NATIVE_FILE_BYTES: u64 = 256 * 1024;
const WEB_ORIGIN: &str = "https://grok.com";
const PROXY_ORIGIN: &str = "https://cli-chat-proxy.grok.com";

/// Subscription and prepaid balance remain distinct. Endpoint errors are available
/// even if the other endpoint supplied usable billing data.
#[derive(Debug)]
pub struct GrokQuota {
    pub subscription: UsageQueryResult,
    pub wallet: Option<UsageWallet>,
    pub endpoint_errors: Vec<UsageQueryError>,
}

pub(super) struct BillingOrigins<'a> {
    pub proxy: &'a str,
    pub web: &'a str,
}

/// The caller must authorize the resolved account and access to the fixed Grok
/// billing hosts. Native queries additionally require the authorized host HOME.
pub async fn query_grok(
    credential: &UsageCredentialRef<'_>,
    home: &Path,
    managed_store: &CredentialFileStore,
    now: TimestampMs,
    correlation_id: &str,
) -> Result<GrokQuota, AuthErrorCode> {
    query_at(
        credential,
        home,
        managed_store,
        client_builder(),
        BillingOrigins { proxy: PROXY_ORIGIN, web: WEB_ORIGIN },
        now,
        correlation_id,
    )
    .await
}

pub(super) async fn query_at(
    credential: &UsageCredentialRef<'_>,
    home: &Path,
    managed_store: &CredentialFileStore,
    builder: ClientBuilder,
    origins: BillingOrigins<'_>,
    now: TimestampMs,
    correlation_id: &str,
) -> Result<GrokQuota, AuthErrorCode> {
    use UsageCredentialState::*;
    use UsageQueryErrorCode::*;

    let scope = credential.scope();
    if scope.service != AuthService::XaiOauth {
        return Err(AuthErrorCode::Unsupported);
    }
    let (source, auth) = match credential.account() {
        UsageAccount::NativeLogin => {
            ("grokbuild", read_native(home, now).map_err(|state| (state, CredentialUnavailable)))
        }
        UsageAccount::Managed(id) => (
            "xai_oauth",
            match XaiRefresh::new()?.access_token(managed_store, scope, id).await {
                Ok(token) => bearer(&token).map_err(|state| (state, CredentialUnavailable)),
                Err(AuthErrorCode::NotFound) => Err((NotFound, CredentialUnavailable)),
                Err(AuthErrorCode::ReauthRequired) => Err((Expired, CredentialUnavailable)),
                Err(AuthErrorCode::UpstreamFailure) => Err((RefreshPending, UpstreamUnavailable)),
                Err(error) => return Err(error),
            },
        ),
    };

    let source = UsageSource::Official(source.into());
    let (bearer, client) = match auth {
        Ok(bearer) => match builder.build() {
            Ok(client) => (bearer, client),
            Err(error) => {
                return Ok(failure(source, None, Valid, transport_error(error), correlation_id));
            }
        },
        Err((state, code)) => return Ok(failure(source, None, state, code, correlation_id)),
    };
    let proxy_request = client
        .get(format!("{}/v1/billing?format=credits", origins.proxy))
        .header("authorization", bearer.clone())
        .header("accept", "application/json")
        .header("x-xai-token-auth", "xai-grok-cli")
        .header("x-grok-client-mode", "cli");
    let web_request = client
        .post(format!("{}/grok_api_v2.GrokBuildBilling/GetGrokCreditsConfig", origins.web))
        .header("authorization", bearer)
        .header("origin", WEB_ORIGIN)
        .header("referer", "https://grok.com/?_s=usage")
        .header("accept", "*/*")
        .header("content-type", "application/grpc-web+proto")
        .header("x-grpc-web", "1")
        .header("x-user-agent", "connect-es/2.1.1")
        .header("user-agent", "ShiJu")
        .body(vec![0; 5]);
    let (proxy, web) = tokio::join!(query_proxy(proxy_request), grpc::query(web_request, now));
    let endpoint_errors: Vec<_> = [
        proxy.as_ref().err().map(|code| query_error(*code, "Grok CLI billing", correlation_id)),
        web.as_ref().err().map(|error| {
            let mut detail = query_error(error.code, "Grok web billing", correlation_id);
            if let Some(status) = error.status {
                detail.message.push_str(&format!(" (gRPC status {status})"));
            }
            detail
        }),
    ]
    .into_iter()
    .flatten()
    .collect();
    let mut data = match (proxy, web) {
        (Ok(data), Ok(web)) => data.with_web_window(web),
        (Ok(data), Err(_)) => data,
        (Err(_), Ok(window)) => BillingData { current: Some(window), ..BillingData::default() },
        (Err(proxy), Err(web)) => {
            let mut result = failure(
                source,
                Some(now),
                if proxy == UpstreamUnauthorized || web.code == UpstreamUnauthorized {
                    Expired
                } else {
                    Valid
                },
                if web.code == UpstreamUnauthorized { web.code } else { proxy },
                correlation_id,
            );
            result.endpoint_errors = endpoint_errors;
            return Ok(result);
        }
    };
    let wallet = data.wallet.take();
    Ok(GrokQuota {
        subscription: UsageQueryResult {
            source,
            queried_at: Some(now),
            credential_state: Valid,
            credential_message: None,
            outcome: UsageQueryOutcome::Success {
                data: UsageQueryData::OfficialSubscription(data.subscription()),
            },
        },
        wallet,
        endpoint_errors,
    })
}

fn failure(
    source: UsageSource,
    queried_at: Option<TimestampMs>,
    state: UsageCredentialState,
    code: UsageQueryErrorCode,
    correlation_id: &str,
) -> GrokQuota {
    use UsageCredentialState::*;

    let message = match state {
        NotFound => Some("Grok login credentials are unavailable"),
        Expired => Some("Grok credentials have expired or were rejected"),
        RefreshPending => Some("Grok credentials require refresh"),
        ParseError => Some("Grok credentials are invalid"),
        Valid => None,
    };
    let mut error = query_error(code, "Grok billing", correlation_id);
    if let Some(message) = message {
        error.message = message.into();
    }
    GrokQuota {
        subscription: UsageQueryResult {
            source,
            queried_at,
            credential_state: state,
            credential_message: message.map(str::to_owned),
            outcome: UsageQueryOutcome::Failure {
                kind: UsageQueryKind::OfficialSubscription,
                error,
            },
        },
        wallet: None,
        endpoint_errors: Vec::new(),
    }
}

fn query_error(code: UsageQueryErrorCode, endpoint: &str, correlation_id: &str) -> UsageQueryError {
    use UsageQueryErrorCode::*;

    let reason = match code {
        Timeout => "request timed out",
        Transport => "transport failed",
        UpstreamUnauthorized => "credentials were rejected",
        UpstreamRateLimited => "request was rate limited",
        UpstreamUnavailable => "service is unavailable",
        UpstreamRejected => "request was rejected",
        InvalidResponse => "response was invalid",
        _ => "credentials are unavailable",
    };
    UsageQueryError {
        code,
        message: format!("{endpoint}: {reason}"),
        correlation_id: correlation_id.into(),
    }
}

fn read_native(home: &Path, now: TimestampMs) -> Result<HeaderValue, UsageCredentialState> {
    use UsageCredentialState::*;

    let path = home.join(".grok/auth.json");
    let file = open_beneath(home, &path, false).map_err(|error| match error {
        ConfigPathState::Absent => NotFound,
        _ => ParseError,
    })?;
    let mut bytes = Vec::new();
    file.take(MAX_NATIVE_FILE_BYTES + 1).read_to_end(&mut bytes).map_err(|_| ParseError)?;
    if bytes.len() as u64 > MAX_NATIVE_FILE_BYTES {
        return Err(ParseError);
    }
    let entries: Map<String, Value> = serde_json::from_slice(&bytes).map_err(|_| ParseError)?;
    let mut oidc = None;
    let mut legacy = None;
    for (scope, entry) in &entries {
        let Some(key) = entry["key"].as_str().filter(|key| !key.is_empty()) else {
            continue;
        };
        if scope.starts_with("https://auth.x.ai::") {
            oidc = Some((entry, key));
        } else if scope == "https://accounts.x.ai/sign-in" {
            legacy = Some((entry, key));
        }
    }
    let (entry, key) = oidc.or(legacy).ok_or(ParseError)?;
    let bearer = bearer(key)?;
    if let Some(expiry) = entry.get("expires_at").filter(|value| !value.is_null()) {
        let expiry = expiry.as_str().ok_or(ParseError)?;
        let expiry = DateTime::parse_from_rfc3339(expiry)
            .map(|value| value.timestamp_millis())
            .or_else(|_| {
                NaiveDateTime::parse_from_str(expiry, "%Y-%m-%dT%H:%M:%S%.f")
                    .map(|value| value.and_utc().timestamp_millis())
            })
            .map_err(|_| ParseError)?;
        if expiry <= now.0 {
            return Err(
                if entry["refresh_token"].as_str().is_some_and(|token| !token.is_empty()) {
                    RefreshPending
                } else {
                    Expired
                },
            );
        }
    }
    Ok(bearer)
}

fn bearer(token: &str) -> Result<HeaderValue, UsageCredentialState> {
    use UsageCredentialState::ParseError;

    if token.is_empty() || token.len() > 16384 || !token.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(ParseError);
    }
    let mut value = header_value(&format!("Bearer {token}")).map_err(|_| ParseError)?;
    value.set_sensitive(true);
    Ok(value)
}

async fn query_proxy(request: RequestBuilder) -> Result<BillingData, UsageQueryErrorCode> {
    let response = request.send().await.map_err(transport_error)?;
    if response.status() == reqwest::StatusCode::REQUEST_TIMEOUT {
        return Err(UsageQueryErrorCode::Timeout);
    }
    let billing: ProxyResponse = read_json_response(response).await?;
    billing.into_data()
}

#[derive(Deserialize)]
struct ProxyResponse {
    config: BillingConfig,
    #[serde(alias = "subscriptionTier")]
    subscription_tier: Option<String>,
    #[serde(alias = "onDemandEnabled")]
    on_demand_enabled: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BillingConfig {
    subscription_tier: Option<String>,
    credit_usage_percent: Option<UsageNumber>,
    current_period: Option<Period>,
    billing_period_end: Option<String>,
    monthly_limit: Option<Cents>,
    used: Option<Cents>,
    prepaid_balance: Option<Cents>,
    on_demand_cap: Option<Cents>,
    on_demand_used: Option<Cents>,
}

#[derive(Deserialize)]
struct Period {
    #[serde(rename = "type")]
    kind: Option<String>,
    end: Option<String>,
}

#[derive(Clone, Copy, Deserialize)]
struct Cents {
    #[serde(default)]
    val: i64,
}

impl Cents {
    fn usd(self) -> UsageNumber {
        number(self.val as f64 / 100.0).expect("converted USD is finite")
    }
}

#[derive(Default)]
struct BillingData {
    plan: Option<String>,
    current: Option<UsageQuotaWindow>,
    monthly: Option<UsageQuotaWindow>,
    wallet: Option<UsageWallet>,
    extra: Option<UsageExtraUsage>,
}

impl ProxyResponse {
    fn into_data(self) -> Result<BillingData, UsageQueryErrorCode> {
        let config = self.config;
        let plan = config.subscription_tier.or(self.subscription_tier).filter(|label| {
            !label.trim().is_empty() && label.len() <= 100 && !label.chars().any(char::is_control)
        });
        let period = config.current_period.as_ref();
        let resets_at = period.and_then(|period| iso(period.end.as_deref()));
        let current = (config.credit_usage_percent.is_some() || resets_at.is_some()).then(|| {
            let name = match period.and_then(|period| period.kind.as_deref()) {
                Some("USAGE_PERIOD_TYPE_WEEKLY") => "weekly_limit",
                Some("USAGE_PERIOD_TYPE_MONTHLY") => "monthly",
                _ => "credits",
            };
            window(name, config.credit_usage_percent, resets_at)
        });
        let used = config.used.map(Cents::usd);
        let total = config.monthly_limit.map(Cents::usd);
        let monthly = (used.is_some() || total.is_some()).then(|| {
            let mut monthly =
                window("monthly", ratio(used, total), iso(config.billing_period_end.as_deref()));
            monthly.used = used;
            monthly.total = total;
            monthly.remaining = used
                .zip(total)
                .and_then(|(used, total)| number(f64::from(total) - f64::from(used)));
            monthly.unit = Some("USD".into());
            monthly
        });
        let wallet = config.prepaid_balance.map(|balance| UsageWallet {
            currency: "USD".into(),
            remaining: Some(balance.usd()),
            total: None,
            used: None,
            granted: None,
            topped_up: None,
        });
        let limit = config.on_demand_cap.map(Cents::usd);
        let used = config.on_demand_used.map(Cents::usd);
        let extra =
            (self.on_demand_enabled.is_some() || limit.is_some() || used.is_some()).then(|| {
                UsageExtraUsage {
                    is_enabled: self
                        .on_demand_enabled
                        .unwrap_or_else(|| limit.is_some_and(|limit| f64::from(limit) > 0.0)),
                    monthly_limit: limit,
                    used_credits: used,
                    utilization: ratio(used, limit),
                    currency: Some("USD".into()),
                }
            });
        if current.is_none() && monthly.is_none() && wallet.is_none() && extra.is_none() {
            return Err(UsageQueryErrorCode::InvalidResponse);
        }
        Ok(BillingData { plan, current, monthly, wallet, extra })
    }
}

impl BillingData {
    fn with_web_window(mut self, web: UsageQuotaWindow) -> Self {
        match &mut self.current {
            Some(current) if current.resets_at.is_some() && current.resets_at == web.resets_at => {
                current.utilization = current.utilization.or(web.utilization);
            }
            Some(_) => {}
            None => self.current = Some(web),
        }
        self
    }

    fn subscription(self) -> UsageSubscription {
        let mut current = self.current;
        let mut monthly = self.monthly;
        if let (Some(current), Some(legacy)) = (&mut current, &monthly)
            && current.name == legacy.name
            && (legacy.resets_at.is_none() || current.resets_at == legacy.resets_at)
        {
            current.utilization = current.utilization.or(legacy.utilization);
            current.used = legacy.used;
            current.total = legacy.total;
            current.remaining = legacy.remaining;
            current.unit.clone_from(&legacy.unit);
            monthly = None;
        }
        let windows = current
            .into_iter()
            .chain(monthly)
            .map(|mut window| {
                window.plan_label.clone_from(&self.plan);
                window
            })
            .collect();
        UsageSubscription {
            subscription_type: self.plan,
            rate_limit_tier: None,
            windows,
            extra_usage: self.extra,
            credits_balance: None,
            reset_credits: None,
        }
    }
}

fn window(
    name: &str,
    utilization: Option<UsageNumber>,
    resets_at: Option<String>,
) -> UsageQuotaWindow {
    UsageQuotaWindow {
        name: name.into(),
        plan_label: None,
        utilization,
        used: None,
        total: None,
        remaining: None,
        unit: None,
        resets_at,
    }
}

fn iso(value: Option<&str>) -> Option<String> {
    DateTime::parse_from_rfc3339(value?)
        .ok()
        .map(|value| value.to_utc().to_rfc3339_opts(SecondsFormat::AutoSi, true))
}

fn ratio(used: Option<UsageNumber>, total: Option<UsageNumber>) -> Option<UsageNumber> {
    let total = f64::from(total?);
    (total > 0.0).then_some(())?;
    number(f64::from(used?) / total * 100.0)
}

fn number(value: f64) -> Option<UsageNumber> {
    UsageNumber::try_from(value).ok()
}
