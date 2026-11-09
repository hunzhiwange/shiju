//! SiliconFlow wallet lookup on the authorized execution host.

use std::time::Duration;

use reqwest::{
    Client, ClientBuilder, StatusCode,
    header::{ACCEPT, AUTHORIZATION},
    redirect::Policy,
};
use serde::{Deserialize, Deserializer, de};
use vw_api_types::apps_usage::{UsageNumber, UsageQueryErrorCode, UsageWallet};
use vw_provider_resolver::remote_models::is_real_bearer;

use crate::{providers::validation::header_value, usage::detect::BalanceProvider};

const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// The execution host must authorize the key for the selected SiliconFlow region before calling.
pub async fn query_siliconflow(
    provider: BalanceProvider,
    api_key: &str,
) -> Result<UsageWallet, UsageQueryErrorCode> {
    let (url, currency) = endpoint(provider)?;
    query_at(client_builder(), url, currency, api_key).await
}

fn endpoint(
    provider: BalanceProvider,
) -> Result<(&'static str, &'static str), UsageQueryErrorCode> {
    match provider {
        BalanceProvider::SiliconFlowCn => Ok(("https://api.siliconflow.cn/v1/user/info", "CNY")),
        BalanceProvider::SiliconFlowGlobal => {
            Ok(("https://api.siliconflow.com/v1/user/info", "USD"))
        }
        _ => Err(UsageQueryErrorCode::Unsupported),
    }
}

fn client_builder() -> ClientBuilder {
    Client::builder().redirect(Policy::none()).timeout(Duration::from_secs(15))
}

async fn query_at(
    client: ClientBuilder,
    url: &str,
    currency: &str,
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
        .get(url)
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
    if response.code.is_some_and(|code| code != 20000) || response.status == Some(false) {
        return Err(UpstreamRejected);
    }
    let balance = response.data.ok_or(InvalidResponse)?;
    if balance.total_balance.is_none()
        && balance.balance.is_none()
        && balance.charge_balance.is_none()
    {
        return Err(InvalidResponse);
    }

    Ok(UsageWallet {
        currency: currency.into(),
        // totalBalance is the current combined balance.
        remaining: balance.total_balance,
        total: None,
        used: None,
        granted: balance.balance,
        topped_up: balance.charge_balance,
    })
}

fn transport_error(error: reqwest::Error) -> UsageQueryErrorCode {
    if error.is_timeout() { UsageQueryErrorCode::Timeout } else { UsageQueryErrorCode::Transport }
}

#[derive(Deserialize)]
struct BalanceResponse {
    code: Option<i64>,
    status: Option<bool>,
    data: Option<BalanceData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BalanceData {
    #[serde(default, deserialize_with = "deserialize_amount")]
    total_balance: Option<UsageNumber>,
    #[serde(default, deserialize_with = "deserialize_amount")]
    balance: Option<UsageNumber>,
    #[serde(default, deserialize_with = "deserialize_amount")]
    charge_balance: Option<UsageNumber>,
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
#[path = "siliconflow_tests.rs"]
mod tests;
