//! StepFun balance lookup on the authorized execution host.

use std::time::Duration;

use reqwest::{
    Client, ClientBuilder, StatusCode,
    header::{ACCEPT, AUTHORIZATION},
    redirect::Policy,
};
use serde::Deserialize;
use vw_api_types::apps_usage::{UsageNumber, UsageQueryErrorCode, UsageWallet};
use vw_provider_resolver::remote_models::is_real_bearer;

use crate::providers::validation::header_value;

const ORIGIN: &str = "https://api.stepfun.com";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// The execution host resolves and authorizes use of the provider key before calling.
pub async fn query_stepfun(api_key: &str) -> Result<UsageWallet, UsageQueryErrorCode> {
    query_at(client_builder(), ORIGIN, api_key).await
}

fn client_builder() -> ClientBuilder {
    Client::builder().redirect(Policy::none()).timeout(Duration::from_secs(15))
}

async fn query_at(
    client: ClientBuilder,
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
        .build()
        .map_err(transport_error)?
        .get(format!("{origin}/v1/accounts"))
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
    let response: BalanceResponse = serde_json::from_slice(&body).map_err(|_| InvalidResponse)?;
    let balance = match response.balance {
        BalanceAmount::Number(value) => value,
        BalanceAmount::Text(value) => value.parse().map_err(|_| InvalidResponse)?,
    };
    let remaining = UsageNumber::try_from(balance).map_err(|_| InvalidResponse)?;
    Ok(UsageWallet {
        currency: "CNY".into(),
        remaining: Some(remaining),
        total: None,
        used: None,
        granted: None,
        topped_up: None,
        detail_url: None,
    })
}

fn transport_error(error: reqwest::Error) -> UsageQueryErrorCode {
    if error.is_timeout() { UsageQueryErrorCode::Timeout } else { UsageQueryErrorCode::Transport }
}

#[derive(Deserialize)]
struct BalanceResponse {
    balance: BalanceAmount,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BalanceAmount {
    Number(f64),
    Text(String),
}

#[cfg(test)]
#[path = "stepfun_tests.rs"]
mod tests;
