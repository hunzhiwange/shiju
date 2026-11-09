//! Volcengine Ark Agent Plan and Coding Plan queries on the authorized execution host.

use chrono::{DateTime, SecondsFormat};
use reqwest::{Request, Response, StatusCode, Url};
use serde_json::Value;
use vw_api_types::{
    TimestampMs,
    apps_usage::{
        UsageCredentialState, UsageNumber, UsageQueryData, UsageQueryError, UsageQueryErrorCode,
        UsageQueryErrorCode::*, UsageQueryKind, UsageQueryOutcome, UsageQueryResult,
        UsageQuotaWindow, UsageSource,
    },
};

use super::{
    volcengine_sign::sign_volcengine_request,
    zhipu::{client_builder, transport_error},
};

const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// Queries personal Ark plan usage using the account AK/SK resolved on the execution host.
/// The caller authorizes the use of the credentials and access to the Ark control plane.
pub async fn query_volcengine(
    base_url: &str,
    access_key_id: &str,
    secret_access_key: &str,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> UsageQueryResult {
    let client = match client_builder().build() {
        Ok(client) => client,
        Err(error) => return query_result(None, Err(transport_error(error)), correlation_id),
    };
    query_at(
        base_url,
        access_key_id,
        secret_access_key,
        queried_at,
        correlation_id,
        async |request| client.execute(request).await,
    )
    .await
}

pub(super) async fn query_at(
    base_url: &str,
    access_key_id: &str,
    secret_access_key: &str,
    queried_at: TimestampMs,
    correlation_id: &str,
    mut send: impl AsyncFnMut(Request) -> Result<Response, reqwest::Error>,
) -> UsageQueryResult {
    let Some(now) = DateTime::from_timestamp_millis(queried_at.0) else {
        return query_result(None, Err(MissingConfiguration), correlation_id);
    };
    let region = region(base_url);
    let mut soft_error = None;
    for (action, parse) in [
        ("GetAFPUsage", parse_agent_plan as fn(&Value) -> _),
        ("GetCodingPlanUsage", parse_coding_plan),
    ] {
        let request = match sign_volcengine_request(
            access_key_id,
            secret_access_key,
            &region,
            action,
            b"",
            now,
        ) {
            Ok(request) => request,
            Err(code) => return query_result(None, Err(code), correlation_id),
        };
        let result = match send(request).await {
            Ok(response) => read_result(response).await.and_then(|body| parse(&body)),
            Err(error) => Err(transport_error(error)),
        };
        match result {
            Ok(windows) if !windows.is_empty() => {
                return query_result(Some(queried_at), Ok(windows), correlation_id);
            }
            Ok(_) => {}
            // Retain a retryable HTTP failure when neither probe provides usable quota.
            Err(code @ (UpstreamRateLimited | UpstreamUnavailable)) => soft_error = Some(code),
            Err(code @ (InvalidResponse | UpstreamRejected)) => {
                soft_error.get_or_insert(code);
            }
            Err(code) => return query_result(Some(queried_at), Err(code), correlation_id),
        }
    }
    query_result(Some(queried_at), soft_error.map_or_else(|| Ok(Vec::new()), Err), correlation_id)
}

fn region(base_url: &str) -> String {
    Url::parse(base_url)
        .ok()
        .and_then(|url| {
            let region = url.host_str()?.strip_prefix("ark.")?.strip_suffix(".volces.com")?;
            let (area, location) = region.split_once('-')?;
            (matches!(area, "cn" | "ap")
                && !location.is_empty()
                && location.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'))
            .then(|| region.to_owned())
        })
        .unwrap_or_else(|| "cn-beijing".into())
}

async fn read_result(mut response: Response) -> Result<Value, UsageQueryErrorCode> {
    let http_error = match response.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(UpstreamUnauthorized),
        StatusCode::TOO_MANY_REQUESTS => Some(UpstreamRateLimited),
        status if status.is_server_error() => Some(UpstreamUnavailable),
        status if status.is_success() => None,
        _ => Some(UpstreamRejected),
    };

    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        if chunk.len() > MAX_RESPONSE_BYTES - bytes.len() {
            return Err(http_error.unwrap_or(InvalidResponse));
        }
        bytes.extend_from_slice(&chunk);
    }
    let mut body: Value =
        serde_json::from_slice(&bytes).map_err(|_| http_error.unwrap_or(InvalidResponse))?;
    let error = body
        .pointer("/ResponseMetadata/Error")
        .filter(|value| !value.is_null())
        .or_else(|| body.get("Error").filter(|value| !value.is_null()));
    if let Some(error) = error {
        if error.get("Code").and_then(Value::as_str).is_some_and(is_auth_code) {
            return Err(UpstreamUnauthorized);
        }
        return Err(http_error.unwrap_or(UpstreamRejected));
    }
    if let Some(code) = http_error {
        return Err(code);
    }

    let result = body.get_mut("Result").map(Value::take).unwrap_or(body);
    if result.is_object() || result.is_null() { Ok(result) } else { Err(InvalidResponse) }
}

fn is_auth_code(code: &str) -> bool {
    // The gateway can append a dot-delimited qualifier to its error codes.
    let code = code.split_once('.').map_or(code, |(code, _)| code);
    matches!(
        code,
        "AccessDenied"
            | "Forbidden"
            | "InvalidAccessKey"
            | "InvalidAccessKeyId"
            | "InvalidAuthorization"
            | "SignatureDoesNotMatch"
            | "Unauthorized"
    )
}

fn parse_agent_plan(result: &Value) -> Result<Vec<UsageQuotaWindow>, UsageQueryErrorCode> {
    let label = match result.get("PlanType").and_then(Value::as_str).map(str::trim) {
        Some("Small") => "Agent Plan Small",
        Some("Medium") => "Agent Plan Medium",
        Some("Large") => "Agent Plan Large",
        Some("Max") => "Agent Plan Max",
        _ => "Agent Plan",
    };
    let mut windows = Vec::with_capacity(3);
    for (key, name) in
        [("AFPFiveHour", "five_hour"), ("AFPWeekly", "weekly_limit"), ("AFPMonthly", "monthly")]
    {
        let Some(window) = result.get(key) else { continue };
        let Some(total) = number(window.get("Quota"))? else { continue };
        let quota = f64::from(total);
        if quota <= 0.0 {
            continue;
        }
        let used = number(window.get("Used"))?;
        let utilization = used.map(|used| finite(f64::from(used) / quota * 100.0)).transpose()?;
        let remaining = used.map(|used| finite(quota - f64::from(used))).transpose()?;
        windows.push(UsageQuotaWindow {
            name: name.into(),
            plan_label: windows.is_empty().then(|| label.into()),
            utilization,
            used,
            total: Some(total),
            remaining,
            unit: Some("AFP".into()),
            resets_at: window.get("ResetTime").and_then(reset_time),
        });
    }
    Ok(windows)
}

fn parse_coding_plan(result: &Value) -> Result<Vec<UsageQuotaWindow>, UsageQueryErrorCode> {
    let items =
        ["QuotaUsage", "Usages", "Details"].iter().find_map(|key| result.get(key)?.as_array());
    let Some(items) = items else { return Ok(Vec::new()) };
    let mut windows = Vec::new();
    for item in items {
        let label = ["Level", "Type", "Period", "Label", "Window"]
            .iter()
            .find_map(|key| item.get(key)?.as_str())
            .unwrap_or_default();
        let name = match label.to_ascii_lowercase().as_str() {
            "session" | "5h" | "fivehour" | "five_hour" | "rolling_5h" => "five_hour",
            "weekly" | "week" | "7d" => "weekly_limit",
            "monthly" | "month" => "monthly",
            _ => continue,
        };
        let percent = ["Percent", "UsedPercent", "UsagePercent"]
            .iter()
            .find_map(|key| item.get(key).filter(|value| !value.is_null()));
        windows.push(UsageQuotaWindow {
            name: name.into(),
            plan_label: windows.is_empty().then(|| "Coding Plan".into()),
            utilization: number(percent)?,
            used: None,
            total: None,
            remaining: None,
            unit: None,
            resets_at: item
                .get("ResetTime")
                .or_else(|| item.get("ResetTimestamp"))
                .and_then(reset_time),
        });
    }
    Ok(windows)
}

fn number(value: Option<&Value>) -> Result<Option<UsageNumber>, UsageQueryErrorCode> {
    let Some(value) = value.filter(|value| !value.is_null()) else { return Ok(None) };
    let number = value.as_f64().or_else(|| value.as_str()?.parse().ok()).ok_or(InvalidResponse)?;
    finite(number).map(Some)
}

fn finite(value: f64) -> Result<UsageNumber, UsageQueryErrorCode> {
    UsageNumber::try_from(value).map_err(|_| InvalidResponse)
}

fn reset_time(value: &Value) -> Option<String> {
    let timestamp = value.as_i64().or_else(|| value.as_str()?.parse().ok());
    if let Some(timestamp) = timestamp {
        if timestamp <= 0 {
            return None;
        }
        let millis = if timestamp < 1_000_000_000_000 { timestamp * 1000 } else { timestamp };
        return DateTime::from_timestamp_millis(millis)
            .map(|time| time.to_rfc3339_opts(SecondsFormat::AutoSi, true));
    }
    let time = value.as_str()?;
    DateTime::parse_from_rfc3339(time).ok().map(|_| time.to_owned())
}

fn query_result(
    queried_at: Option<TimestampMs>,
    result: Result<Vec<UsageQuotaWindow>, UsageQueryErrorCode>,
    correlation_id: &str,
) -> UsageQueryResult {
    use UsageCredentialState::{Expired, NotFound, Valid};

    let mut credential_state = Valid;
    let mut credential_message = None;
    let failure = |code, message: &str| UsageQueryOutcome::Failure {
        kind: UsageQueryKind::CodingPlan,
        error: UsageQueryError {
            code,
            message: message.into(),
            correlation_id: correlation_id.into(),
        },
    };
    let outcome = match result {
        Ok(windows) if windows.is_empty() => {
            failure(UpstreamRejected, "No active Volcengine Agent Plan or Coding Plan")
        }
        Ok(windows) => UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) },
        Err(code) => {
            let message = match code {
                CredentialUnavailable => {
                    credential_state = NotFound;
                    "Volcengine account AK/SK is unavailable"
                }
                UpstreamUnauthorized => {
                    credential_state = Expired;
                    "Volcengine rejected the account AK/SK or its Ark usage-query permissions"
                }
                _ => "Volcengine plan query failed",
            };
            credential_message = (credential_state != Valid).then(|| message.into());
            failure(code, message)
        }
    };
    UsageQueryResult {
        source: UsageSource::Provider("volcengine".into()),
        queried_at,
        credential_state,
        credential_message,
        outcome,
    }
}
