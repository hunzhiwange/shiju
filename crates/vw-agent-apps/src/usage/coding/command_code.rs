//! Command Code credit balances and quota windows from the fixed control plane.

use chrono::{DateTime, SecondsFormat};
use reqwest::{
    Client, ClientBuilder, StatusCode, Url,
    header::{ACCEPT, AUTHORIZATION, HeaderValue},
};
use serde_json::Value;
use vw_api_types::{
    TimestampMs,
    apps_usage::{
        UsageCredentialState, UsageNumber, UsageQueryData, UsageQueryError, UsageQueryErrorCode,
        UsageQueryKind, UsageQueryOutcome, UsageQueryResult, UsageQuotaWindow, UsageSource,
        UsageWallet,
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

use UsageQueryErrorCode::*;

#[derive(Debug)]
pub struct CommandCodeUsage {
    pub balance: UsageQueryResult,
    pub coding_plan: UsageQueryResult,
    /// Additional errors for individual windows or the monthly summary.
    pub window_errors: Vec<UsageQueryError>,
}

/// Queries the same control plane for the Claude and Codex inference bases.
/// The caller authorizes access to api.commandcode.ai and resolves the credential.
pub async fn query_command_code(
    base_url: &str,
    api_key: &str,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> CommandCodeUsage {
    query_at(
        client_builder(),
        "https://api.commandcode.ai",
        base_url,
        api_key,
        queried_at,
        correlation_id,
    )
    .await
}

async fn query_at(
    builder: ClientBuilder,
    origin: &str,
    base_url: &str,
    api_key: &str,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> CommandCodeUsage {
    let start = || {
        let capability = detect_native_usage(base_url, ZhipuPlanSelection::Auto)
            .map_err(|_| MissingConfiguration)?;
        if capability != Some(NativeUsageCapability::CodingPlan(CodingPlanProvider::CommandCode)) {
            return Err(Unsupported);
        }
        if !is_real_bearer(api_key) || api_key.chars().any(char::is_control) {
            return Err(CredentialUnavailable);
        }
        let mut bearer = header_value(&format!("Bearer {}", api_key.trim()))
            .map_err(|_| CredentialUnavailable)?;
        bearer.set_sensitive(true);
        let client = builder.build().map_err(transport_error)?;
        Ok(ControlPlane { client, origin, bearer, correlation_id })
    };
    let (queried_at, failure) = match start() {
        Ok(control) => match control.read_usage(queried_at, api_key.trim()).await {
            Ok(usage) => return usage,
            Err(error) => (Some(queried_at), error),
        },
        Err(code) => {
            (None, query_error(code, "Command Code query could not start", correlation_id))
        }
    };
    CommandCodeUsage {
        balance: query_result(UsageQueryKind::Balance, queried_at, Err(failure.clone())),
        coding_plan: query_result(UsageQueryKind::CodingPlan, queried_at, Err(failure)),
        window_errors: Vec::new(),
    }
}

struct ControlPlane<'a> {
    client: Client,
    origin: &'a str,
    bearer: HeaderValue,
    correlation_id: &'a str,
}

impl ControlPlane<'_> {
    async fn get(&self, path: &str, params: &[(&str, &str)]) -> Result<Value, UsageQueryErrorCode> {
        let mut url =
            Url::parse(&format!("{}{path}", self.origin)).map_err(|_| MissingConfiguration)?;
        if !params.is_empty() {
            url.query_pairs_mut().extend_pairs(params);
        }
        let response = self
            .client
            .get(url)
            .header(ACCEPT, "application/json")
            .header(AUTHORIZATION, self.bearer.clone())
            .send()
            .await
            .map_err(transport_error)?;
        if response.status() == StatusCode::FORBIDDEN {
            return Err(UpstreamRejected);
        }
        let body: Value = read_json_response(response).await?;
        if !body.is_object() {
            return Err(InvalidResponse);
        }
        if body["success"].as_bool() == Some(false) {
            return Err(UpstreamRejected);
        }
        Ok(body)
    }

    async fn read_usage(
        &self,
        queried_at: TimestampMs,
        api_key: &str,
    ) -> Result<CommandCodeUsage, UsageQueryError> {
        let error = |code, message| query_error(code, message, self.correlation_id);
        let identity = self
            .get("/alpha/whoami", &[("limits", "1")])
            .await
            .map_err(|code| error(code, "Command Code identity query failed"))?;
        let org_params = match &identity["org"] {
            Value::Null => Vec::new(),
            org => {
                let id =
                    org["id"].as_str().filter(|id| !id.trim().is_empty()).ok_or_else(|| {
                        error(InvalidResponse, "Command Code organization is invalid")
                    })?;
                vec![("orgId", id)]
            }
        };
        let credits = self
            .get("/alpha/billing/credits", &org_params)
            .await
            .map_err(|code| error(code, "Command Code credit query failed"))?;
        let balance = credit_balance(&credits["credits"])
            .map_err(|code| error(code, "Command Code credit balance is invalid"));
        let mut window_errors = Vec::new();

        let subscription = self.get("/alpha/billing/subscriptions", &org_params).await;
        let subscription = subscription.as_ref().map_err(|&code| code).and_then(|body| {
            let data = &body["data"];
            if !data.is_object() && !data.is_null() {
                return Err(InvalidResponse);
            }
            let since = match &data["currentPeriodStart"] {
                Value::Null => None,
                value => Some(
                    value
                        .as_str()
                        .filter(|time| DateTime::parse_from_rfc3339(time).is_ok())
                        .ok_or(InvalidResponse)?,
                ),
            };
            Ok((data, since))
        });

        let plan_label = subscription
            .ok()
            .and_then(|(data, _)| data["planId"].as_str())
            .or_else(|| credits["credits"]["planId"].as_str())
            .filter(|label| {
                !label.is_empty()
                    && label.len() <= 64
                    && !label.contains(api_key)
                    && label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
            });
        let limits = credits.get("windowLimits").or_else(|| credits["credits"].get("windowLimits"));
        let mut windows = Vec::with_capacity(3);
        if let Some(limits) = limits
            && limits["limited"].as_bool() == Some(true)
        {
            for (key, name, message) in [
                ("fiveHour", "five_hour", "Command Code five-hour window is invalid"),
                ("weekly", "weekly_limit", "Command Code weekly window is invalid"),
            ] {
                if let Some(value) = limits.get(key).filter(|value| !value.is_null()) {
                    match rolling_window(name, plan_label, value) {
                        Ok(window) => windows.push(window),
                        Err(code) => window_errors.push(error(code, message)),
                    }
                }
            }
        }

        let monthly = match (subscription, &balance) {
            // A failed lookup cannot establish the period for a monthly summary.
            (Err(code), _) => Err(error(code, "Command Code subscription query failed")),
            (_, Err(failure)) => Err(failure.clone()),
            (Ok((data, since)), Ok((_, pool_remaining))) => {
                let mut params: Vec<_> = since.map(|time| ("since", time)).into_iter().collect();
                params.extend(org_params);
                self.get("/alpha/usage/summary", &params)
                    .await
                    .and_then(|summary| monthly_window(plan_label, data, *pool_remaining, &summary))
                    .map_err(|code| error(code, "Command Code monthly usage query failed"))
            }
        };
        let coding_plan = match monthly {
            Ok(window) => {
                windows.push(window);
                Ok(UsageQueryData::CodingPlan(windows))
            }
            Err(failure) if failure.code == UpstreamUnauthorized => return Err(failure),
            Err(failure) if windows.is_empty() => Err(failure),
            Err(failure) => {
                window_errors.push(failure);
                Ok(UsageQueryData::CodingPlan(windows))
            }
        };
        Ok(CommandCodeUsage {
            balance: query_result(
                UsageQueryKind::Balance,
                Some(queried_at),
                balance.map(|(wallet, _)| UsageQueryData::Balance(vec![wallet])),
            ),
            coding_plan: query_result(UsageQueryKind::CodingPlan, Some(queried_at), coding_plan),
            window_errors,
        })
    }
}

fn credit_balance(credits: &Value) -> Result<(UsageWallet, f64), UsageQueryErrorCode> {
    let mut has_amount = false;
    let mut remaining = 0.0;
    let mut pool_remaining = 0.0;
    for key in ["monthlyCredits", "purchasedCredits", "freeCredits"] {
        if !credits[key].is_null() {
            let amount = number(&credits[key])?;
            has_amount = true;
            remaining += amount;
            pool_remaining += amount.max(0.0);
        }
    }
    if !has_amount {
        return Err(InvalidResponse);
    }
    Ok((
        UsageWallet {
            currency: "USD".into(),
            remaining: Some(finite(remaining)?),
            total: None,
            used: None,
            granted: None,
            topped_up: None,
        },
        f64::from(finite(pool_remaining)?),
    ))
}

fn rolling_window(
    name: &str,
    plan_label: Option<&str>,
    data: &Value,
) -> Result<UsageQuotaWindow, UsageQueryErrorCode> {
    let used = number(&data["used"])?;
    let cap = number(&data["cap"])?;
    if cap <= 0.0 {
        return Err(InvalidResponse);
    }
    Ok(UsageQuotaWindow {
        name: name.into(),
        plan_label: plan_label.map(str::to_owned),
        utilization: Some(finite(used / cap * 100.0)?),
        used: Some(finite(used)?),
        total: Some(finite(cap)?),
        remaining: Some(finite(cap - used)?),
        unit: Some("USD".into()),
        resets_at: reset_time(&data["resetAt"]),
    })
}

fn monthly_window(
    plan_label: Option<&str>,
    subscription: &Value,
    pool_remaining: f64,
    summary: &Value,
) -> Result<UsageQuotaWindow, UsageQueryErrorCode> {
    let spent = number(&summary["totalCost"])?;
    let pool = f64::from(finite(spent + pool_remaining)?);
    let percent = if pool > 0.0 { spent / pool * 100.0 } else { 0.0 };
    Ok(UsageQuotaWindow {
        name: "monthly".into(),
        plan_label: plan_label.map(str::to_owned),
        utilization: Some(finite(percent)?),
        used: None,
        total: None,
        remaining: None,
        unit: Some("USD".into()),
        resets_at: reset_time(&subscription["currentPeriodEnd"]),
    })
}

fn number(value: &Value) -> Result<f64, UsageQueryErrorCode> {
    let number = value.as_f64().or_else(|| value.as_str()?.parse().ok()).ok_or(InvalidResponse)?;
    finite(number).map(f64::from)
}

fn finite(value: f64) -> Result<UsageNumber, UsageQueryErrorCode> {
    UsageNumber::try_from(value).map_err(|_| InvalidResponse)
}

fn reset_time(value: &Value) -> Option<String> {
    if let Some(time) = value.as_str() {
        return DateTime::parse_from_rfc3339(time).ok().map(|_| time.to_owned());
    }
    let timestamp = value.as_i64().filter(|&timestamp| timestamp > 0)?;
    let millis = if timestamp < 1_000_000_000_000 { timestamp * 1000 } else { timestamp };
    DateTime::from_timestamp_millis(millis)
        .map(|time| time.to_rfc3339_opts(SecondsFormat::AutoSi, true))
}

fn query_error(code: UsageQueryErrorCode, message: &str, correlation_id: &str) -> UsageQueryError {
    let message = match code {
        UpstreamUnauthorized => "Command Code rejected the credential",
        CredentialUnavailable => "Command Code credential is unavailable",
        _ => message,
    };
    UsageQueryError { code, message: message.into(), correlation_id: correlation_id.into() }
}

fn query_result(
    kind: UsageQueryKind,
    queried_at: Option<TimestampMs>,
    data: Result<UsageQueryData, UsageQueryError>,
) -> UsageQueryResult {
    use UsageCredentialState::{Expired, NotFound, Valid};

    let (credential_state, credential_message, outcome) = match data {
        Ok(data) => (Valid, None, UsageQueryOutcome::Success { data }),
        Err(error) => {
            let state = match error.code {
                UpstreamUnauthorized => Expired,
                CredentialUnavailable => NotFound,
                _ => Valid,
            };
            let message = (state != Valid).then(|| error.message.clone());
            (state, message, UsageQueryOutcome::Failure { kind, error })
        }
    };
    UsageQueryResult {
        source: UsageSource::Provider("command_code".into()),
        queried_at,
        credential_state,
        credential_message,
        outcome,
    }
}

#[cfg(test)]
#[path = "command_code_tests.rs"]
mod tests;
