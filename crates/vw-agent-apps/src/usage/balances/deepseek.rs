//! DeepSeek wallet lookup on the authorized execution host.

use std::time::Duration;

use reqwest::{
    Client, ClientBuilder, StatusCode,
    header::{ACCEPT, AUTHORIZATION},
    redirect::Policy,
};
use serde::{Deserialize, Deserializer, de};
use vw_api_types::apps_usage::{UsageNumber, UsageQueryErrorCode, UsageWallet};
use vw_provider_resolver::remote_models::is_real_bearer;

use crate::providers::validation::header_value;

const ORIGIN: &str = "https://api.deepseek.com";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Debug, PartialEq)]
pub struct DeepSeekBalance {
    /// Whether DeepSeek reports enough balance to call its API.
    pub is_available: Option<bool>,
    pub wallets: Vec<UsageWallet>,
}

/// The execution host resolves and authorizes use of the provider key before calling.
/// HTTP 401 and 403 return `UpstreamUnauthorized` without any wallet data.
pub async fn query_deepseek(api_key: &str) -> Result<DeepSeekBalance, UsageQueryErrorCode> {
    let client = client_builder().build().map_err(transport_error)?;
    query_at(&client, ORIGIN, api_key).await
}

/// Uses the same fixed endpoint with the execution host's transport settings.
pub async fn query_deepseek_with_transport(
    transport: ClientBuilder,
    api_key: &str,
) -> Result<DeepSeekBalance, UsageQueryErrorCode> {
    let client = transport
        .https_only(true)
        .redirect(Policy::none())
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(transport_error)?;
    query_at(&client, ORIGIN, api_key).await
}

fn client_builder() -> ClientBuilder {
    Client::builder().redirect(Policy::none()).timeout(Duration::from_secs(15))
}

async fn query_at(
    client: &Client,
    origin: &str,
    api_key: &str,
) -> Result<DeepSeekBalance, UsageQueryErrorCode> {
    use UsageQueryErrorCode::*;

    if !is_real_bearer(api_key) || api_key.chars().any(char::is_control) {
        return Err(CredentialUnavailable);
    }
    let mut bearer =
        header_value(&format!("Bearer {}", api_key.trim())).map_err(|_| CredentialUnavailable)?;
    bearer.set_sensitive(true);
    let mut response = client
        .get(format!("{origin}/user/balance"))
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
    let balance: BalanceResponse = serde_json::from_slice(&body).map_err(|_| InvalidResponse)?;
    let wallets = balance
        .balance_infos
        .into_iter()
        .map(|wallet| {
            if wallet.currency.len() != 3
                || !wallet.currency.bytes().all(|b| b.is_ascii_uppercase())
            {
                return Err(InvalidResponse);
            }
            Ok(UsageWallet {
                currency: wallet.currency,
                // DeepSeek's total_balance is the current combined balance.
                remaining: wallet.total_balance,
                total: None,
                used: None,
                granted: wallet.granted_balance,
                topped_up: wallet.topped_up_balance,
                detail_url: Some("https://platform.deepseek.com/usage".into()),
            })
        })
        .collect::<Result<_, _>>()?;
    Ok(DeepSeekBalance { is_available: balance.is_available, wallets })
}

fn transport_error(error: reqwest::Error) -> UsageQueryErrorCode {
    if error.is_timeout() { UsageQueryErrorCode::Timeout } else { UsageQueryErrorCode::Transport }
}

#[derive(Deserialize)]
struct BalanceResponse {
    is_available: Option<bool>,
    balance_infos: Vec<WalletResponse>,
}

#[derive(Deserialize)]
struct WalletResponse {
    currency: String,
    #[serde(default, deserialize_with = "deserialize_amount")]
    total_balance: Option<UsageNumber>,
    #[serde(default, deserialize_with = "deserialize_amount")]
    granted_balance: Option<UsageNumber>,
    #[serde(default, deserialize_with = "deserialize_amount")]
    topped_up_balance: Option<UsageNumber>,
}

fn deserialize_amount<'de, D>(deserializer: D) -> Result<Option<UsageNumber>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Amount {
        Number(f64),
        Text(String),
    }

    let value = match Option::<Amount>::deserialize(deserializer)? {
        None => return Ok(None),
        Some(Amount::Number(value)) => value,
        Some(Amount::Text(value)) => value.parse::<f64>().map_err(de::Error::custom)?,
    };
    UsageNumber::try_from(value).map(Some).map_err(de::Error::custom)
}

#[cfg(test)]
#[path = "deepseek_tests.rs"]
mod tests;
