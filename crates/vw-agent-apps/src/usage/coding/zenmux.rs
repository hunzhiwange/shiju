//! ZenMux Coding Plan queries against the authorized quota endpoint.

use chrono::DateTime;
use reqwest::{
    ClientBuilder, Url,
    header::{ACCEPT, AUTHORIZATION},
};
use serde::{Deserialize, Deserializer, de};
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
use crate::providers::validation::header_value;

/// Queries the configured ZenMux quota endpoint using credentials resolved on the execution host.
/// The caller must authorize the endpoint and the use of the credential.
pub async fn query_zenmux(
    endpoint: &str,
    api_key: &str,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> UsageQueryResult {
    let endpoint = Url::parse(endpoint)
        .ok()
        .filter(|url| {
            url.scheme() == "https"
                && url.host_str() == Some("zenmux.ai")
                && url.port().is_none()
                && url.username().is_empty()
                && url.password().is_none()
                && url.fragment().is_none()
        })
        .ok_or(UsageQueryErrorCode::MissingConfiguration);
    query_at(client_builder(), endpoint, api_key, queried_at, correlation_id).await
}

async fn query_at(
    builder: ClientBuilder,
    endpoint: Result<Url, UsageQueryErrorCode>,
    api_key: &str,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> UsageQueryResult {
    use UsageCredentialState::{Expired, NotFound, Valid};
    use UsageQueryErrorCode::{CredentialUnavailable, UpstreamUnauthorized};

    let request = endpoint.and_then(|url| {
        if !is_real_bearer(api_key) || api_key.chars().any(char::is_control) {
            return Err(CredentialUnavailable);
        }
        let mut authorization = header_value(&format!("Bearer {}", api_key.trim()))
            .map_err(|_| CredentialUnavailable)?;
        authorization.set_sensitive(true);
        let client = builder.build().map_err(transport_error)?;
        Ok(client.get(url).header(AUTHORIZATION, authorization).header(ACCEPT, "application/json"))
    });
    let (queried_at, result) = match request {
        Ok(request) => (Some(queried_at), read_json(request).await.and_then(parse_plan)),
        Err(code) => (None, Err(code)),
    };
    let (credential_state, credential_message, outcome) = match result {
        Ok((windows, plan_info)) => (
            Valid,
            plan_info,
            UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) },
        ),
        Err(code) => {
            let (state, message) = match code {
                UpstreamUnauthorized => (Expired, "ZenMux rejected the credential"),
                CredentialUnavailable => (NotFound, "ZenMux credential is unavailable"),
                _ => (Valid, "ZenMux Coding Plan query failed"),
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
        source: UsageSource::Provider("zenmux".into()),
        queried_at,
        credential_state,
        credential_message,
        outcome,
    }
}

#[derive(Deserialize)]
struct PlanResponse {
    success: bool,
    data: Option<Value>,
}

#[derive(Deserialize)]
struct PlanData {
    quota_5_hour: Option<Quota>,
    quota_7_day: Option<Quota>,
    plan: Option<Plan>,
    account_status: Option<String>,
}

#[derive(Deserialize)]
struct Plan {
    tier: Option<String>,
}

#[derive(Deserialize)]
struct Quota {
    #[serde(default, deserialize_with = "deserialize_number")]
    usage_percentage: Option<UsageNumber>,
    #[serde(default, deserialize_with = "deserialize_number")]
    used_value_usd: Option<UsageNumber>,
    #[serde(default, deserialize_with = "deserialize_number")]
    max_value_usd: Option<UsageNumber>,
    resets_at: Option<String>,
}

fn deserialize_number<'de, D>(deserializer: D) -> Result<Option<UsageNumber>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum WireNumber {
        Number(f64),
        Text(String),
    }

    let number = Option::<WireNumber>::deserialize(deserializer)?;
    let value = match number {
        Some(WireNumber::Number(value)) => value,
        Some(WireNumber::Text(value)) => value.parse::<f64>().map_err(de::Error::custom)?,
        None => return Ok(None),
    };
    UsageNumber::try_from(value).map(Some).map_err(de::Error::custom)
}

fn parse_plan(
    response: PlanResponse,
) -> Result<(Vec<UsageQuotaWindow>, Option<String>), UsageQueryErrorCode> {
    use UsageQueryErrorCode::{InvalidResponse, UpstreamRejected};

    if !response.success {
        return Err(UpstreamRejected);
    }
    let data: PlanData = serde_json::from_value(response.data.ok_or(InvalidResponse)?)
        .map_err(|_| InvalidResponse)?;

    let mut windows = Vec::with_capacity(2);
    for (name, quota) in [("five_hour", data.quota_5_hour), ("weekly_limit", data.quota_7_day)] {
        if let Some(quota) = quota {
            windows.push(quota.into_window(name)?);
        }
    }
    let first = windows.first_mut().ok_or(InvalidResponse)?;
    let tier = data.plan.and_then(|plan| plan.tier).filter(|tier| !tier.is_empty());
    let plan_info = tier.map(|tier| {
        first.plan_label = Some(format!("ZenMux·{}", tier.to_uppercase()));
        match data.account_status.filter(|status| !status.is_empty()) {
            Some(status) => format!("{tier} ({status})"),
            None => tier,
        }
    });
    Ok((windows, plan_info))
}

impl Quota {
    fn into_window(self, name: &str) -> Result<UsageQuotaWindow, UsageQueryErrorCode> {
        use UsageQueryErrorCode::InvalidResponse;

        let Self { usage_percentage, used_value_usd: used, max_value_usd: total, resets_at } = self;
        if usage_percentage.is_none() && used.is_none() && total.is_none() && resets_at.is_none() {
            return Err(InvalidResponse);
        }
        if let Some(reset) = &resets_at {
            DateTime::parse_from_rfc3339(reset).map_err(|_| InvalidResponse)?;
        }
        let utilization = usage_percentage
            .map(|ratio| UsageNumber::try_from(f64::from(ratio) * 100.0))
            .transpose()
            .map_err(|_| InvalidResponse)?;
        let remaining = total
            .zip(used)
            .map(|(total, used)| UsageNumber::try_from(f64::from(total) - f64::from(used)))
            .transpose()
            .map_err(|_| InvalidResponse)?;
        Ok(UsageQuotaWindow {
            name: name.into(),
            plan_label: None,
            utilization,
            used,
            total,
            remaining,
            unit: used.or(total).map(|_| "USD".into()),
            resets_at,
        })
    }
}

#[cfg(test)]
#[path = "zenmux_tests.rs"]
mod tests;
