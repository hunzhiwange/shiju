use std::time::Duration;

use serde::Deserialize;
use serde_json::{Map, Value};

use super::{ChannelQrStatus, ProviderFetch, ProviderPoll};
use crate::app::agent::gateway::ApiError;

const REGISTRATION_ENDPOINT: &str = "https://accounts.feishu.cn/oauth/v1/app/registration";

#[derive(Deserialize)]
struct InitResponse {
    #[serde(default)]
    supported_auth_methods: Vec<String>,
}

#[derive(Deserialize)]
struct BeginResponse {
    #[serde(default)]
    device_code: String,
    #[serde(default)]
    verification_uri_complete: String,
}

#[derive(Deserialize)]
struct PollResponse {
    #[serde(default)]
    client_id: String,
    #[serde(default)]
    client_secret: String,
    #[serde(default)]
    error: String,
}

pub(super) async fn fetch() -> Result<ProviderFetch, ApiError> {
    fetch_from_endpoint(REGISTRATION_ENDPOINT).await
}

pub(super) async fn poll(state: &str) -> Result<ProviderPoll, ApiError> {
    poll_from_endpoint(REGISTRATION_ENDPOINT, state).await
}

async fn fetch_from_endpoint(endpoint: &str) -> Result<ProviderFetch, ApiError> {
    let client = client()?;
    let init: InitResponse = post_form(&client, endpoint, &[("action", "init")]).await?;
    if !init.supported_auth_methods.iter().any(|method| method == "client_secret") {
        return Err(ApiError::internal("Feishu QR authorization method is unsupported"));
    }

    let begin: BeginResponse = post_form(
        &client,
        endpoint,
        &[
            ("action", "begin"),
            ("archetype", "PersonalAgent"),
            ("auth_method", "client_secret"),
            ("request_user_info", "open_id"),
        ],
    )
    .await?;
    if begin.device_code.trim().is_empty() || begin.verification_uri_complete.trim().is_empty() {
        return Err(ApiError::internal("Feishu QR authorization response is incomplete"));
    }
    let separator = if begin.verification_uri_complete.contains('?') { '&' } else { '?' };
    Ok(ProviderFetch {
        scan_url: format!("{}{separator}source=VibeWindow", begin.verification_uri_complete),
        poll_state: begin.device_code,
    })
}

async fn poll_from_endpoint(endpoint: &str, state: &str) -> Result<ProviderPoll, ApiError> {
    let client = client()?;
    let response: PollResponse =
        post_form(&client, endpoint, &[("action", "poll"), ("device_code", state)]).await?;

    if !response.client_id.trim().is_empty() && !response.client_secret.trim().is_empty() {
        let credentials = Map::from_iter([
            ("app_id".to_string(), Value::String(response.client_id)),
            ("app_secret".to_string(), Value::String(response.client_secret)),
        ]);
        return Ok(ProviderPoll {
            status: ChannelQrStatus::Authorized,
            credentials: Some(credentials),
        });
    }

    let status = match response.error.as_str() {
        "" | "authorization_pending" | "slow_down" => ChannelQrStatus::Pending,
        "expired_token" | "invalid_grant" => ChannelQrStatus::Expired,
        _ => ChannelQrStatus::Failed,
    };
    Ok(ProviderPoll { status, credentials: None })
}

fn client() -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| ApiError::internal("Feishu QR authorization client is unavailable"))
}

async fn post_form<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    endpoint: &str,
    form: &[(&str, &str)],
) -> Result<T, ApiError> {
    let body = form
        .iter()
        .map(|(key, value)| format!("{}={}", urlencoding::encode(key), urlencoding::encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    client
        .post(endpoint)
        .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| ApiError::internal("Feishu QR authorization request failed"))?
        .json()
        .await
        .map_err(|_| ApiError::internal("Feishu QR authorization response was invalid"))
}

#[cfg(test)]
#[path = "feishu_tests.rs"]
mod feishu_tests;
