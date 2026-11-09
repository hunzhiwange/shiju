//! OpenCode Go quota queries on the authorized execution host.

use chrono::DateTime;
use reqwest::{
    ClientBuilder, RequestBuilder, StatusCode,
    header::{ACCEPT, AUTHORIZATION},
};
use serde_json::Value;
use vw_api_types::{
    TimestampMs,
    apps_usage::{
        UsageCredentialState, UsageNumber, UsageQueryData, UsageQueryError, UsageQueryErrorCode,
        UsageQueryKind, UsageQueryOutcome, UsageQueryResult, UsageQuotaWindow, UsageSource,
    },
};
use vw_provider_resolver::remote_models::is_real_bearer;

use super::zhipu::{client_builder, read_json_response, transport_error};
use crate::{
    providers::validation::header_value,
    usage::detect::{
        CodingPlanProvider, NativeUsageCapability, ZhipuPlanSelection, detect_native_usage,
    },
};

/// Queries the fixed OpenCode Go usage endpoint for either supported Go inference base.
/// The caller authorizes access to opencode.ai and the use of the resolved credential.
pub async fn query_opencode_go(
    base_url: &str,
    api_key: &str,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> UsageQueryResult {
    query_at(client_builder(), "https://opencode.ai", base_url, api_key, queried_at, correlation_id)
        .await
}

async fn query_at(
    builder: ClientBuilder,
    origin: &str,
    base_url: &str,
    api_key: &str,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> UsageQueryResult {
    use UsageCredentialState::{Expired, NotFound, Valid};
    use UsageQueryErrorCode::*;

    let capability =
        detect_native_usage(base_url, ZhipuPlanSelection::Auto).map_err(|_| MissingConfiguration);
    let request = capability.and_then(|capability| {
        if capability != Some(NativeUsageCapability::CodingPlan(CodingPlanProvider::OpenCodeGo)) {
            return Err(Unsupported);
        }
        if !is_real_bearer(api_key) || api_key.chars().any(char::is_control) {
            return Err(CredentialUnavailable);
        }
        let mut authorization = header_value(&format!("Bearer {}", api_key.trim()))
            .map_err(|_| CredentialUnavailable)?;
        authorization.set_sensitive(true);
        let client = builder.build().map_err(transport_error)?;
        Ok(client
            .get(format!("{origin}/zen/go/v1/usage"))
            .header(AUTHORIZATION, authorization)
            .header(ACCEPT, "application/json"))
    });
    let (queried_at, result) = match request {
        Ok(request) => (Some(queried_at), read_plan(request).await),
        Err(code) => (None, Err(QueryFailure::Request(code))),
    };
    let (credential_state, credential_message, outcome) = match result {
        Ok(windows) => {
            (Valid, None, UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) })
        }
        Err(failure) => {
            let (code, state, message) = match failure {
                QueryFailure::NoSubscription => {
                    (UpstreamRejected, Valid, "The OpenCode workspace has no Go subscription")
                }
                QueryFailure::Request(code) => match code {
                    UpstreamUnauthorized => (code, Expired, "OpenCode Go rejected the credential"),
                    CredentialUnavailable => {
                        (code, NotFound, "OpenCode Go credential is unavailable")
                    }
                    Unsupported => (code, Valid, "The provider has no OpenCode Go quota endpoint"),
                    _ => (code, Valid, "OpenCode Go quota query failed"),
                },
            };
            let outcome = UsageQueryOutcome::Failure {
                kind: UsageQueryKind::CodingPlan,
                error: UsageQueryError {
                    code,
                    message: message.into(),
                    correlation_id: correlation_id.into(),
                },
            };
            (state, (state != Valid).then(|| message.into()), outcome)
        }
    };
    UsageQueryResult {
        source: UsageSource::Provider("opencode_go".into()),
        queried_at,
        credential_state,
        credential_message,
        outcome,
    }
}

enum QueryFailure {
    NoSubscription,
    Request(UsageQueryErrorCode),
}

async fn read_plan(request: RequestBuilder) -> Result<Vec<UsageQuotaWindow>, QueryFailure> {
    let response =
        request.send().await.map_err(|error| QueryFailure::Request(transport_error(error)))?;
    if response.status() == StatusCode::FORBIDDEN {
        return Err(QueryFailure::NoSubscription);
    }
    let body: Value = read_json_response(response).await.map_err(QueryFailure::Request)?;
    let usage = &body["usage"];
    let mut windows = Vec::with_capacity(3);
    for (key, name) in
        [("rolling", "five_hour"), ("weekly", "weekly_limit"), ("monthly", "monthly")]
    {
        let window = &usage[key];
        let value = &window["percent"];
        let Some(percent) = value.as_f64().or_else(|| value.as_str()?.parse().ok()) else {
            continue;
        };
        let Ok(utilization) = UsageNumber::try_from(percent) else {
            continue;
        };
        // At zero utilization the upstream reset time is a placeholder.
        let resets_at = window["resetsAt"]
            .as_str()
            .filter(|reset| percent > 0.0 && DateTime::parse_from_rfc3339(reset).is_ok())
            .map(str::to_owned);
        windows.push(UsageQuotaWindow {
            name: name.into(),
            plan_label: None,
            utilization: Some(utilization),
            used: None,
            total: None,
            remaining: None,
            unit: Some("USD".into()),
            resets_at,
        });
    }
    if windows.is_empty() {
        return Err(QueryFailure::Request(UsageQueryErrorCode::InvalidResponse));
    }
    Ok(windows)
}

#[cfg(test)]
#[path = "opencode_go_tests.rs"]
mod tests;
