//! OpenRouter credit balance lookup on the authorized execution host.

use std::time::Duration;

use reqwest::{
    Client, ClientBuilder, StatusCode,
    header::{ACCEPT, AUTHORIZATION},
    redirect::Policy,
};
use serde_json::Value;
use vw_api_types::apps_usage::{UsageNumber, UsageQueryErrorCode, UsageWallet};
use vw_provider_resolver::remote_models::is_real_bearer;

use crate::providers::validation::header_value;

const ORIGIN: &str = "https://openrouter.ai";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// The execution host must resolve and authorize the OpenRouter management key before calling.
pub async fn query_openrouter(api_key: &str) -> Result<UsageWallet, UsageQueryErrorCode> {
    let client = client_builder().build().map_err(transport_error)?;
    query_at(&client, ORIGIN, api_key).await
}

fn client_builder() -> ClientBuilder {
    Client::builder().redirect(Policy::none()).timeout(Duration::from_secs(15))
}

async fn query_at(
    client: &Client,
    origin: &str,
    api_key: &str,
) -> Result<UsageWallet, UsageQueryErrorCode> {
    use UsageQueryErrorCode::*;

    if !is_real_bearer(api_key) || api_key.chars().any(char::is_control) {
        return Err(CredentialUnavailable);
    }
    let mut bearer =
        header_value(&format!("Bearer {}", api_key.trim())).map_err(|_| CredentialUnavailable)?;
    bearer.set_sensitive(true);
    let mut response = client
        .get(format!("{origin}/api/v1/credits"))
        .header(AUTHORIZATION, bearer)
        .header(ACCEPT, "application/json")
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

    let response: Value = serde_json::from_slice(&body).map_err(|_| InvalidResponse)?;
    let data = response.get("data").unwrap_or(&response);
    let total = amount(data, "total_credits").ok_or(InvalidResponse)?;
    let used = amount(data, "total_usage").ok_or(InvalidResponse)?;
    let remaining =
        UsageNumber::try_from(f64::from(total) - f64::from(used)).map_err(|_| InvalidResponse)?;

    Ok(UsageWallet {
        currency: "USD".into(),
        total: Some(total),
        used: Some(used),
        remaining: Some(remaining),
        granted: None,
        topped_up: None,
    })
}

fn amount(data: &Value, field: &str) -> Option<UsageNumber> {
    let value = data.get(field)?;
    let number = match value {
        Value::Number(number) => number.as_f64()?,
        Value::String(text) => text.parse().ok()?,
        _ => return None,
    };
    UsageNumber::try_from(number).ok()
}

fn transport_error(error: reqwest::Error) -> UsageQueryErrorCode {
    if error.is_timeout() { UsageQueryErrorCode::Timeout } else { UsageQueryErrorCode::Transport }
}

#[cfg(test)]
#[path = "openrouter_tests.rs"]
mod tests;
