//! Kimi Coding Plan usage from the official regional API.

use std::time::Duration;

use chrono::DateTime;
use reqwest::{
    Client, ClientBuilder, Request, RequestBuilder, StatusCode,
    header::{ACCEPT, AUTHORIZATION},
    redirect::Policy,
};
use serde_json::Value;
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::AuthRegion,
    apps_usage::{
        UsageCredentialState, UsageNumber, UsageQueryData, UsageQueryError, UsageQueryErrorCode,
        UsageQueryKind, UsageQueryOutcome, UsageQueryResult, UsageQuotaWindow, UsageSource,
    },
};
use vw_provider_resolver::remote_models::is_real_bearer;

use crate::providers::validation::header_value;

const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// Queries the Kimi Coding Plan for a region resolved on the execution host.
/// The caller must authorize the query, resolve its credential, and supply the query time.
pub async fn query_kimi(
    region: AuthRegion,
    api_key: &str,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> UsageQueryResult {
    query_at(client_builder(), api_root(region), api_key, queried_at, correlation_id).await
}

fn api_root(region: AuthRegion) -> &'static str {
    match region {
        AuthRegion::MainlandCn => "https://api.kimi.com/coding/v1",
        AuthRegion::Global => "https://api.kimi.ai/coding/v1",
    }
}

fn client_builder() -> ClientBuilder {
    Client::builder().redirect(Policy::none()).timeout(Duration::from_secs(15))
}

async fn query_at(
    builder: ClientBuilder,
    api_root: &str,
    api_key: &str,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> UsageQueryResult {
    use UsageCredentialState::*;
    use UsageQueryErrorCode::*;

    let (queried_at, result) = match usage_request(builder, api_root, api_key) {
        Ok(request) => (Some(queried_at), fetch_windows(request, |_| {}).await),
        Err(code) => (None, Err(code)),
    };
    let (credential_state, credential_message, outcome) = match result {
        Ok(windows) => {
            (Valid, None, UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) })
        }
        Err(code) => {
            let (state, message) = match code {
                UpstreamUnauthorized => (Expired, "Kimi rejected the credential"),
                CredentialUnavailable => (NotFound, "Kimi credential is unavailable"),
                _ => (Valid, "Kimi Coding Plan query failed"),
            };
            let credential_message = (state != Valid).then(|| message.into());
            let outcome = UsageQueryOutcome::Failure {
                kind: UsageQueryKind::CodingPlan,
                error: UsageQueryError {
                    code,
                    message: message.into(),
                    correlation_id: correlation_id.into(),
                },
            };
            (state, credential_message, outcome)
        }
    };
    UsageQueryResult {
        source: UsageSource::Provider("kimi".into()),
        queried_at,
        credential_state,
        credential_message,
        outcome,
    }
}

fn usage_request(
    builder: ClientBuilder,
    api_root: &str,
    api_key: &str,
) -> Result<RequestBuilder, UsageQueryErrorCode> {
    use UsageQueryErrorCode::CredentialUnavailable;

    if !is_real_bearer(api_key) || api_key.chars().any(char::is_control) {
        return Err(CredentialUnavailable);
    }
    let mut bearer =
        header_value(&format!("Bearer {}", api_key.trim())).map_err(|_| CredentialUnavailable)?;
    bearer.set_sensitive(true);
    let client = builder.build().map_err(transport_error)?;
    Ok(client
        .get(format!("{api_root}/usages"))
        .header(AUTHORIZATION, bearer)
        .header(ACCEPT, "application/json"))
}

#[cfg(target_os = "macos")]
pub(in crate::usage) async fn query_windows(
    region: AuthRegion,
    access_token: &str,
    route: impl FnOnce(&mut Request),
) -> Result<Vec<UsageQuotaWindow>, UsageQueryErrorCode> {
    let request = usage_request(client_builder(), api_root(region), access_token)?;
    fetch_windows(request, route).await
}

async fn fetch_windows(
    request: RequestBuilder,
    route: impl FnOnce(&mut Request),
) -> Result<Vec<UsageQuotaWindow>, UsageQueryErrorCode> {
    use UsageQueryErrorCode::*;

    let (client, request) = request.build_split();
    let mut request = request.map_err(transport_error)?;
    route(&mut request);
    let mut response = client.execute(request).await.map_err(transport_error)?;
    match response.status() {
        status if status.is_success() => {}
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(UpstreamUnauthorized),
        StatusCode::TOO_MANY_REQUESTS => return Err(UpstreamRateLimited),
        status if status.is_server_error() => return Err(UpstreamUnavailable),
        _ => return Err(UpstreamRejected),
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        if chunk.len() > MAX_RESPONSE_BYTES - body.len() {
            return Err(InvalidResponse);
        }
        body.extend_from_slice(&chunk);
    }
    let payload: Value = serde_json::from_slice(&body).map_err(|_| InvalidResponse)?;
    let payload = payload.as_object().ok_or(InvalidResponse)?;
    let mut windows = Vec::new();
    if let Some(limits) = payload.get("limits").filter(|value| !value.is_null()) {
        for limit in limits.as_array().ok_or(InvalidResponse)? {
            let limit = limit.as_object().ok_or(InvalidResponse)?;
            if let Some(detail) = limit.get("detail").filter(|value| !value.is_null()) {
                windows.push(parse_window("five_hour", detail)?);
            }
        }
    }
    if let Some(usage) = payload.get("usage").filter(|value| !value.is_null()) {
        windows.push(parse_window("weekly_limit", usage)?);
    }
    Ok(windows)
}

fn parse_window(name: &str, value: &Value) -> Result<UsageQuotaWindow, UsageQueryErrorCode> {
    use UsageQueryErrorCode::InvalidResponse;

    let detail = value.as_object().ok_or(InvalidResponse)?;
    let total = parse_amount(detail.get("limit"))?;
    let remaining = parse_amount(detail.get("remaining"))?;
    let (used, utilization) = match (total, remaining) {
        (Some(total), Some(remaining)) => {
            let limit = f64::from(total);
            let used = (limit - f64::from(remaining)).max(0.0);
            let utilization = if limit > 0.0 { used / limit * 100.0 } else { 0.0 };
            (
                Some(UsageNumber::try_from(used).map_err(|_| InvalidResponse)?),
                Some(UsageNumber::try_from(utilization).map_err(|_| InvalidResponse)?),
            )
        }
        _ => (None, None),
    };
    Ok(UsageQuotaWindow {
        name: name.into(),
        plan_label: None,
        utilization,
        used,
        total,
        remaining,
        unit: None,
        resets_at: parse_reset_time(detail.get("resetTime"))?,
    })
}

fn parse_amount(value: Option<&Value>) -> Result<Option<UsageNumber>, UsageQueryErrorCode> {
    use UsageQueryErrorCode::InvalidResponse;

    let amount = match value {
        Some(Value::Number(number)) => number.as_f64(),
        Some(Value::String(text)) => text.parse().ok(),
        None | Some(Value::Null) => return Ok(None),
        _ => None,
    };
    amount.and_then(|amount| UsageNumber::try_from(amount).ok()).map(Some).ok_or(InvalidResponse)
}

fn parse_reset_time(value: Option<&Value>) -> Result<Option<String>, UsageQueryErrorCode> {
    use UsageQueryErrorCode::InvalidResponse;

    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if let Some(text) = value.as_str() {
        DateTime::parse_from_rfc3339(text).map_err(|_| InvalidResponse)?;
        return Ok(Some(text.into()));
    }
    let timestamp = value.as_i64().ok_or(InvalidResponse)?;
    if timestamp <= 0 {
        return Ok(None);
    }
    // Kimi reports epoch seconds and milliseconds in the same field.
    let millis = if timestamp < 1_000_000_000_000 { timestamp * 1000 } else { timestamp };
    DateTime::from_timestamp_millis(millis)
        .map(|time| Some(time.to_rfc3339()))
        .ok_or(InvalidResponse)
}

fn transport_error(error: reqwest::Error) -> UsageQueryErrorCode {
    if error.is_timeout() { UsageQueryErrorCode::Timeout } else { UsageQueryErrorCode::Transport }
}

#[cfg(test)]
#[path = "kimi_tests.rs"]
mod tests;
