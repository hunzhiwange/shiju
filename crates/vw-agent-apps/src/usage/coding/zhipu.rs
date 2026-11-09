//! Personal Zhipu Coding Plan quota queries against the official regional endpoints.

use std::time::Duration;

use chrono::DateTime;
use reqwest::{
    Client, ClientBuilder, RequestBuilder, StatusCode,
    header::{ACCEPT_LANGUAGE, AUTHORIZATION, CONTENT_TYPE, HeaderName, HeaderValue},
    redirect::Policy,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use vw_api_types::{
    TimestampMs,
    apps_usage::{
        UsageCredentialState, UsageNumber, UsageQueryData, UsageQueryError, UsageQueryErrorCode,
        UsageQueryKind, UsageQueryOutcome, UsageQueryResult, UsageQuotaWindow, UsageSource,
    },
};
use vw_provider_resolver::remote_models::is_real_bearer;

use crate::{providers::validation::header_value, usage::detect::CodingPlanProvider};

pub(super) const CN_ORIGIN: &str = "https://open.bigmodel.cn";
pub(super) const PLAN_PATH: &str = "/api/monitor/usage/quota/limit";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// Queries a personal plan using credentials resolved on the authorized execution host.
/// Returns `None` only when the upstream explicitly reports that the account has no plan.
pub async fn query_zhipu(
    provider: CodingPlanProvider,
    api_key: &str,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> Option<UsageQueryResult> {
    query_at(client_builder(), personal_origin(provider), api_key, queried_at, correlation_id).await
}

pub(in crate::usage) fn personal_origin(
    provider: CodingPlanProvider,
) -> Result<&'static str, UsageQueryErrorCode> {
    match provider {
        CodingPlanProvider::ZhipuCn => Ok(CN_ORIGIN),
        CodingPlanProvider::ZhipuGlobal => Ok("https://api.z.ai"),
        _ => Err(UsageQueryErrorCode::Unsupported),
    }
}

pub(in crate::usage) fn client_builder() -> ClientBuilder {
    Client::builder().redirect(Policy::none()).timeout(Duration::from_secs(15))
}

pub(in crate::usage) fn authorization(api_key: &str) -> Result<HeaderValue, UsageQueryErrorCode> {
    use UsageQueryErrorCode::CredentialUnavailable;

    if !is_real_bearer(api_key) || api_key.chars().any(char::is_control) {
        return Err(CredentialUnavailable);
    }
    let mut header = header_value(api_key.trim()).map_err(|_| CredentialUnavailable)?;
    header.set_sensitive(true);
    Ok(header)
}

async fn query_at(
    builder: ClientBuilder,
    origin: Result<&str, UsageQueryErrorCode>,
    api_key: &str,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> Option<UsageQueryResult> {
    let request = origin.and_then(|origin| {
        let authorization = authorization(api_key)?;
        let client = builder.build().map_err(transport_error)?;
        Ok((origin, authorization, client))
    });
    let (queried_at, result) = match request {
        Ok((origin, authorization, client)) => {
            let result = match query_personal_plan(&client, origin, &authorization).await {
                Ok(Some(data)) => parse_windows(data),
                Ok(None) => return None,
                Err(code) => Err(code),
            };
            (Some(queried_at), result)
        }
        Err(code) => (None, Err(code)),
    };
    Some(plan_result("zhipu", queried_at, correlation_id, result))
}

pub(super) fn plan_result(
    provider_id: &str,
    queried_at: Option<TimestampMs>,
    correlation_id: &str,
    result: Result<Vec<UsageQuotaWindow>, UsageQueryErrorCode>,
) -> UsageQueryResult {
    use UsageCredentialState::*;
    use UsageQueryErrorCode::*;

    let (credential_state, credential_message, outcome) = match result {
        Ok(windows) => {
            (Valid, None, UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) })
        }
        Err(code) => {
            let (state, message) = match code {
                UpstreamUnauthorized => (Expired, "Zhipu rejected the credential"),
                CredentialUnavailable => (NotFound, "Zhipu credential is unavailable"),
                MissingConfiguration => {
                    (NotFound, "Zhipu team requires a valid organization and project ID")
                }
                _ => (Valid, "Zhipu Coding Plan query failed"),
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
        source: UsageSource::Provider(provider_id.into()),
        queried_at,
        credential_state,
        credential_message,
        outcome,
    }
}

pub(in crate::usage) async fn query_personal_plan(
    client: &Client,
    origin: &str,
    authorization: &HeaderValue,
) -> Result<Option<Value>, UsageQueryErrorCode> {
    match get_json(client, origin, PLAN_PATH, CONTENT_TYPE, authorization).await? {
        PlanResponse { success: false, code: Some(500), msg: Some(ref msg), data: None }
            if is_no_plan_message(msg) =>
        {
            Ok(None)
        }
        response => response.into_data().map(Some),
    }
}

#[derive(Deserialize)]
struct PlanData {
    level: Option<String>,
    limits: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuotaLimit {
    unit: Option<i64>,
    percentage: Option<UsageNumber>,
    next_reset_time: Option<i64>,
}

pub(super) fn parse_windows(data: Value) -> Result<Vec<UsageQuotaWindow>, UsageQueryErrorCode> {
    use UsageQueryErrorCode::InvalidResponse;

    let PlanData { level, limits } = serde_json::from_value(data).map_err(|_| InvalidResponse)?;
    let mut five_hour = None;
    let mut weekly = None;
    let mut unclassified = Vec::new();
    for item in limits {
        let kind = item.get("type").and_then(Value::as_str).ok_or(InvalidResponse)?;
        if !kind.eq_ignore_ascii_case("TOKENS_LIMIT") && !kind.eq_ignore_ascii_case("CREDIT_LIMIT")
        {
            continue;
        }
        let mut limit: QuotaLimit = serde_json::from_value(item).map_err(|_| InvalidResponse)?;
        limit.next_reset_time = limit.next_reset_time.filter(|millis| *millis > 0);
        match limit.unit {
            Some(3) if five_hour.is_none() => five_hour = Some(limit),
            Some(6) if weekly.is_none() => weekly = Some(limit),
            _ => unclassified.push(limit),
        }
    }

    // Legacy windows have no unit. A five-hour window at zero usage may have no reset time.
    unclassified.sort_by_key(|limit| limit.next_reset_time);
    let mut unclassified = unclassified.into_iter();
    let mut windows = Vec::new();
    for (name, slot) in [("five_hour", five_hour), ("weekly_limit", weekly)] {
        if let Some(limit) = slot.or_else(|| unclassified.next()) {
            let resets_at = limit
                .next_reset_time
                .map(|millis| {
                    DateTime::from_timestamp_millis(millis)
                        .map(|time| time.to_rfc3339())
                        .ok_or(InvalidResponse)
                })
                .transpose()?;
            windows.push(UsageQuotaWindow {
                name: name.into(),
                plan_label: level.clone(),
                utilization: limit.percentage,
                used: None,
                total: None,
                remaining: None,
                unit: None,
                resets_at,
            });
        }
    }
    Ok(windows)
}

#[derive(Deserialize)]
pub(super) struct PlanResponse {
    success: bool,
    code: Option<u32>,
    msg: Option<String>,
    data: Option<Value>,
}

impl PlanResponse {
    pub(super) fn into_data(self) -> Result<Value, UsageQueryErrorCode> {
        use UsageQueryErrorCode::*;

        match self {
            Self { success: false, .. } => Err(UpstreamRejected),
            Self { success: true, data: Some(data), .. }
                if data.get("limits").is_some_and(Value::is_array) =>
            {
                Ok(data)
            }
            _ => Err(InvalidResponse),
        }
    }
}

fn is_no_plan_message(message: &str) -> bool {
    // Upstream reuses business code 500, so require a full known absence message.
    [
        "当前用户不存在coding plan",
        "The current user does not have a coding plan",
        "Coding Plan not exist",
    ]
    .iter()
    .any(|known| message.trim().eq_ignore_ascii_case(known))
}

pub(in crate::usage) async fn get_json<T: DeserializeOwned>(
    client: &Client,
    origin: &str,
    path: &str,
    json_header: HeaderName,
    authorization: &HeaderValue,
) -> Result<T, UsageQueryErrorCode> {
    let request = client
        .get(format!("{origin}{path}"))
        .header(AUTHORIZATION, authorization.clone())
        .header(json_header, "application/json")
        .header(ACCEPT_LANGUAGE, "en-US,en");
    read_json(request).await
}

pub(super) async fn read_json<T: DeserializeOwned>(
    request: RequestBuilder,
) -> Result<T, UsageQueryErrorCode> {
    use UsageQueryErrorCode::*;

    let mut response = request.send().await.map_err(transport_error)?;

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
    serde_json::from_slice(&body).map_err(|_| InvalidResponse)
}

pub(in crate::usage) fn transport_error(error: reqwest::Error) -> UsageQueryErrorCode {
    if error.is_timeout() { UsageQueryErrorCode::Timeout } else { UsageQueryErrorCode::Transport }
}

#[cfg(test)]
#[path = "zhipu_tests.rs"]
mod tests;
