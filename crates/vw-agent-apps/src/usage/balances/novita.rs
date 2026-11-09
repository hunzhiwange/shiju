//! Novita wallet lookup on the authorized execution host.

use std::time::Duration;

use reqwest::{
    Client, ClientBuilder, StatusCode,
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    redirect::Policy,
};
use serde::Deserialize;
use vw_api_types::apps_usage::{UsageNumber, UsageQueryErrorCode, UsageWallet};
use vw_provider_resolver::remote_models::is_real_bearer;

use crate::providers::validation::header_value;

const ORIGIN: &str = "https://api.novita.ai";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// The execution host must resolve and authorize the Novita API key before calling.
pub async fn query_novita(api_key: &str) -> Result<UsageWallet, UsageQueryErrorCode> {
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
        .get(format!("{origin}/openapi/v1/billing/balance/detail"))
        .header(AUTHORIZATION, bearer)
        .header(ACCEPT, "application/json")
        .header(CONTENT_TYPE, "application/json")
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

    let balance: BalanceResponse = serde_json::from_slice(&body).map_err(|_| InvalidResponse)?;
    Ok(UsageWallet {
        currency: "USD".into(),
        remaining: Some(balance.available_balance.in_usd()?),
        total: None,
        used: None,
        // The balance endpoint has no gift amount. Its credit limit is a borrowing limit.
        granted: None,
        topped_up: balance.cash_balance.map(BalanceAmount::in_usd).transpose()?,
        detail_url: None,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BalanceResponse {
    available_balance: BalanceAmount,
    cash_balance: Option<BalanceAmount>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BalanceAmount {
    Number(f64),
    Text(String),
}

impl BalanceAmount {
    fn in_usd(self) -> Result<UsageNumber, UsageQueryErrorCode> {
        use UsageQueryErrorCode::InvalidResponse;

        let amount = match self {
            Self::Number(amount) => amount,
            Self::Text(amount) => amount.parse::<f64>().map_err(|_| InvalidResponse)?,
        };
        UsageNumber::try_from(amount / 10_000.0).map_err(|_| InvalidResponse)
    }
}

fn transport_error(error: reqwest::Error) -> UsageQueryErrorCode {
    if error.is_timeout() { UsageQueryErrorCode::Timeout } else { UsageQueryErrorCode::Transport }
}

#[cfg(test)]
#[path = "novita_tests.rs"]
mod tests;
