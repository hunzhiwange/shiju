//! Zhipu personal Coding Plan balance fallback on the authorized execution host.

use std::time::Duration;

use reqwest::{
    Client, ClientBuilder, StatusCode,
    header::{ACCEPT, ACCEPT_LANGUAGE, AUTHORIZATION, CONTENT_TYPE, HeaderName, HeaderValue},
    redirect::Policy,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use vw_api_types::apps_usage::{UsageNumber, UsageQueryErrorCode, UsageWallet};
use vw_provider_resolver::remote_models::is_real_bearer;

use crate::{providers::validation::header_value, usage::detect::CodingPlanProvider};

const PLAN_PATH: &str = "/api/monitor/usage/quota/limit";
const BALANCE_PATH: &str = "/api/biz/account/query-customer-account-report";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// Probes the authorized personal plan before querying its regional wallet.
/// Returns `Ok(None)` when the plan endpoint reports an existing plan.
/// The caller must authorize the provider and resolve its API key on the execution host.
pub async fn query_zhipu_balance(
    provider: CodingPlanProvider,
    api_key: &str,
) -> Result<Option<UsageWallet>, UsageQueryErrorCode> {
    let (origin, currency) = match provider {
        CodingPlanProvider::ZhipuCn => ("https://open.bigmodel.cn", "CNY"),
        CodingPlanProvider::ZhipuGlobal => ("https://api.z.ai", "USD"),
        _ => return Err(UsageQueryErrorCode::Unsupported),
    };
    let client = client_builder().build().map_err(transport_error)?;
    query_at(&client, origin, currency, api_key).await
}

fn client_builder() -> ClientBuilder {
    Client::builder().redirect(Policy::none()).timeout(Duration::from_secs(15))
}

async fn query_at(
    client: &Client,
    origin: &str,
    currency: &str,
    api_key: &str,
) -> Result<Option<UsageWallet>, UsageQueryErrorCode> {
    use UsageQueryErrorCode::*;

    if !is_real_bearer(api_key) || api_key.chars().any(char::is_control) {
        return Err(CredentialUnavailable);
    }
    let mut authorization = header_value(api_key.trim()).map_err(|_| CredentialUnavailable)?;
    authorization.set_sensitive(true);

    let plan: PlanResponse =
        get_json(client, origin, PLAN_PATH, CONTENT_TYPE, &authorization).await?;
    match plan {
        PlanResponse { success: false, code: Some(500), msg: Some(ref msg), data: None }
            if is_no_plan_message(msg) => {}
        PlanResponse { success: false, .. } => return Err(UpstreamRejected),
        PlanResponse { success: true, data: Some(data), .. }
            if data.get("limits").is_some_and(Value::is_array) =>
        {
            return Ok(None);
        }
        _ => return Err(InvalidResponse),
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

#[derive(Deserialize)]
struct PlanResponse {
    success: bool,
    code: Option<u32>,
    msg: Option<String>,
    data: Option<Value>,
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

async fn get_json<T: DeserializeOwned>(
    client: &Client,
    origin: &str,
    path: &str,
    json_header: HeaderName,
    authorization: &HeaderValue,
) -> Result<T, UsageQueryErrorCode> {
    use UsageQueryErrorCode::*;

    let mut response = client
        .get(format!("{origin}{path}"))
        .header(AUTHORIZATION, authorization.clone())
        .header(json_header, "application/json")
        .header(ACCEPT_LANGUAGE, "en-US,en")
        .send()
        .await
        .map_err(transport_error)?;

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

fn amount(data: &Value, field: &str) -> Option<f64> {
    let value = data.get(field)?;
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}

fn transport_error(error: reqwest::Error) -> UsageQueryErrorCode {
    if error.is_timeout() { UsageQueryErrorCode::Timeout } else { UsageQueryErrorCode::Transport }
}

#[cfg(test)]
#[path = "zhipu_tests.rs"]
mod tests;
