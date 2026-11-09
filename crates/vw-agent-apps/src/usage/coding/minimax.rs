//! MiniMax Coding Plan quota queries against the official regional endpoints.

use chrono::DateTime;
use reqwest::{
    ClientBuilder,
    header::{AUTHORIZATION, CONTENT_TYPE},
};
use serde::Deserialize;
use serde_json::Value;
use vw_api_types::{
    TimestampMs,
    apps_usage::{
        UsageCredentialState, UsageNumber, UsageQueryData, UsageQueryError, UsageQueryErrorCode,
        UsageQueryKind, UsageQueryOutcome, UsageQueryResult, UsageQuotaWindow, UsageSource,
    },
};
use vw_provider_resolver::remote_models::is_real_bearer;

use super::zhipu::{client_builder, read_json, transport_error};
use crate::{providers::validation::header_value, usage::detect::CodingPlanProvider};

const PLAN_PATH: &str = "/v1/api/openplatform/coding_plan/remains";

/// Queries the MiniMax region resolved from the provider configuration.
/// The caller must authorize the query and resolve the API key on the execution host.
pub async fn query_minimax(
    provider: CodingPlanProvider,
    api_key: &str,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> UsageQueryResult {
    query_at(client_builder(), minimax_origin(provider), api_key, queried_at, correlation_id).await
}

fn minimax_origin(provider: CodingPlanProvider) -> Result<&'static str, UsageQueryErrorCode> {
    match provider {
        CodingPlanProvider::MiniMaxCn => Ok("https://api.minimaxi.com"),
        CodingPlanProvider::MiniMaxGlobal => Ok("https://api.minimax.io"),
        _ => Err(UsageQueryErrorCode::Unsupported),
    }
}

async fn query_at(
    builder: ClientBuilder,
    origin: Result<&str, UsageQueryErrorCode>,
    api_key: &str,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> UsageQueryResult {
    use UsageCredentialState::*;
    use UsageQueryErrorCode::*;

    let request = origin.and_then(|origin| {
        if !is_real_bearer(api_key) || api_key.chars().any(char::is_control) {
            return Err(CredentialUnavailable);
        }
        let mut authorization = header_value(&format!("Bearer {}", api_key.trim()))
            .map_err(|_| CredentialUnavailable)?;
        authorization.set_sensitive(true);
        let client = builder.build().map_err(transport_error)?;
        Ok(client
            .get(format!("{origin}{PLAN_PATH}"))
            .header(AUTHORIZATION, authorization)
            .header(CONTENT_TYPE, "application/json"))
    });
    let (queried_at, result) = match request {
        Ok(request) => {
            (Some(queried_at), read_json::<Value>(request).await.and_then(parse_windows))
        }
        Err(code) => (None, Err(code)),
    };
    let (credential_state, credential_message, outcome) = match result {
        Ok(windows) => {
            (Valid, None, UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) })
        }
        Err(code) => {
            let (state, message) = match code {
                UpstreamUnauthorized => (Expired, "MiniMax rejected the credential"),
                CredentialUnavailable => (NotFound, "MiniMax credential is unavailable"),
                _ => (Valid, "MiniMax Coding Plan query failed"),
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
        source: UsageSource::Provider("minimax".into()),
        queried_at,
        credential_state,
        credential_message,
        outcome,
    }
}

#[derive(Deserialize)]
struct GeneralQuota {
    current_interval_remaining_percent: Option<UsageNumber>,
    current_interval_total_count: Option<UsageNumber>,
    current_interval_usage_count: Option<UsageNumber>,
    end_time: Option<i64>,
    current_weekly_status: Option<i64>,
    current_weekly_remaining_percent: Option<UsageNumber>,
    current_weekly_total_count: Option<UsageNumber>,
    current_weekly_usage_count: Option<UsageNumber>,
    weekly_end_time: Option<i64>,
}

fn parse_windows(body: Value) -> Result<Vec<UsageQuotaWindow>, UsageQueryErrorCode> {
    use UsageQueryErrorCode::{InvalidResponse, UpstreamRejected};

    if let Some(base_resp) = body.get("base_resp") {
        match base_resp.get("status_code").and_then(Value::as_i64) {
            Some(0) => {}
            Some(_) => return Err(UpstreamRejected),
            None => return Err(InvalidResponse),
        }
    }

    let model_remains =
        body.get("model_remains").and_then(Value::as_array).ok_or(InvalidResponse)?;
    let general = model_remains
        .iter()
        .find(|item| item.get("model_name").and_then(Value::as_str) == Some("general"))
        .ok_or(InvalidResponse)?;
    let quota = GeneralQuota::deserialize(general).map_err(|_| InvalidResponse)?;

    let mut windows = Vec::with_capacity(2);
    if let Some(percent) = quota.current_interval_remaining_percent {
        windows.push(quota_window(
            "five_hour",
            percent,
            quota.current_interval_total_count,
            quota.current_interval_usage_count,
            quota.end_time,
        )?);
    }
    // Weekly status 2 is an exhausted limit. Status 3 means the plan has no weekly limit.
    if matches!(quota.current_weekly_status, Some(1 | 2))
        && let Some(percent) = quota.current_weekly_remaining_percent
    {
        windows.push(quota_window(
            "weekly_limit",
            percent,
            quota.current_weekly_total_count,
            quota.current_weekly_usage_count,
            quota.weekly_end_time,
        )?);
    }
    if windows.is_empty() {
        return Err(InvalidResponse);
    }
    Ok(windows)
}

fn quota_window(
    name: &str,
    remaining_percent: UsageNumber,
    total: Option<UsageNumber>,
    remaining: Option<UsageNumber>,
    reset_ms: Option<i64>,
) -> Result<UsageQuotaWindow, UsageQueryErrorCode> {
    use UsageQueryErrorCode::InvalidResponse;

    // Percentage-based plans can send zero placeholders in the count fields.
    let total = total.filter(|count| f64::from(*count) > 0.0);
    // Despite its name, the upstream usage_count holds the remaining count.
    let remaining = total.and(remaining);
    let used = if let Some((total, remaining)) = total.zip(remaining) {
        let (total, remaining) = (f64::from(total), f64::from(remaining));
        if !(0.0..=total).contains(&remaining) {
            return Err(InvalidResponse);
        }
        Some(UsageNumber::try_from(total - remaining).map_err(|_| InvalidResponse)?)
    } else {
        None
    };
    let utilization =
        UsageNumber::try_from(100.0 - f64::from(remaining_percent)).map_err(|_| InvalidResponse)?;
    let resets_at = reset_ms
        .map(|ms| DateTime::from_timestamp_millis(ms).ok_or(InvalidResponse))
        .transpose()?
        .map(|time| time.to_rfc3339());
    Ok(UsageQuotaWindow {
        name: name.into(),
        plan_label: None,
        utilization: Some(utilization),
        used,
        total,
        remaining,
        unit: total.map(|_| "requests".into()),
        resets_at,
    })
}

#[cfg(test)]
#[path = "minimax_tests.rs"]
mod tests;
