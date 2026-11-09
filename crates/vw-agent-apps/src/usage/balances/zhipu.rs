//! Zhipu personal Coding Plan balance fallback on the authorized execution host.

use reqwest::{Client, header::ACCEPT};
use serde_json::Value;
use vw_api_types::apps_usage::{UsageNumber, UsageQueryErrorCode, UsageWallet};

use crate::usage::{
    coding::zhipu::{
        authorization, client_builder, get_json, personal_origin, query_personal_plan,
        transport_error,
    },
    detect::CodingPlanProvider,
};

const BALANCE_PATH: &str = "/api/biz/account/query-customer-account-report";

/// Probes the authorized personal plan before querying its regional wallet.
/// Returns `Ok(None)` when the plan endpoint reports an existing plan.
/// The caller must authorize the provider and resolve its API key on the execution host.
pub async fn query_zhipu_balance(
    provider: CodingPlanProvider,
    api_key: &str,
) -> Result<Option<UsageWallet>, UsageQueryErrorCode> {
    let origin = personal_origin(provider)?;
    let currency = if provider == CodingPlanProvider::ZhipuCn { "CNY" } else { "USD" };
    let client = client_builder().build().map_err(transport_error)?;
    query_at(&client, origin, currency, api_key).await
}

async fn query_at(
    client: &Client,
    origin: &str,
    currency: &str,
    api_key: &str,
) -> Result<Option<UsageWallet>, UsageQueryErrorCode> {
    use UsageQueryErrorCode::*;

    let authorization = authorization(api_key)?;
    if query_personal_plan(client, origin, &authorization).await?.is_some() {
        return Ok(None);
    }

    let balance: Value = get_json(client, origin, BALANCE_PATH, ACCEPT, &authorization).await?;
    if balance.get("success").and_then(Value::as_bool) == Some(false) {
        return Err(UpstreamRejected);
    }
    let data = balance.get("data").filter(|data| data.is_object()).unwrap_or(&balance);
    let remaining = amount(data, "availableBalance")
        .or_else(|| amount(data, "balance"))
        .and_then(|amount| UsageNumber::try_from(amount).ok())
        .ok_or(InvalidResponse)?;
    Ok(Some(UsageWallet {
        currency: currency.into(),
        remaining: Some(remaining),
        total: None,
        used: None,
        granted: None,
        topped_up: None,
    }))
}

fn amount(data: &Value, field: &str) -> Option<f64> {
    let value = data.get(field)?;
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}

#[cfg(test)]
#[path = "zhipu_tests.rs"]
mod tests;
