use serde_json::{Map, Value, json};

use super::{ChannelQrStatus, ProviderFetch, ProviderPoll};
use crate::app::agent::gateway::ApiError;

const DINGTALK_API_BASE: &str = "https://oapi.dingtalk.com";
const DINGTALK_SOURCE: &str = "QWENPAW";

pub(super) async fn fetch() -> Result<ProviderFetch, ApiError> {
    fetch_at(DINGTALK_API_BASE).await
}

pub(super) async fn poll(state: &str) -> Result<ProviderPoll, ApiError> {
    poll_at(DINGTALK_API_BASE, state).await
}

async fn fetch_at(base_url: &str) -> Result<ProviderFetch, ApiError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|_| provider_error("DingTalk QR client initialization failed"))?;

    let init = post_json(
        &client,
        &format!("{base_url}/app/registration/init"),
        &json!({"source": DINGTALK_SOURCE}),
        "init",
    )
    .await?;
    validate_errcode(&init, "init")?;
    let nonce = required_string(&init, "nonce", "DingTalk returned empty nonce")?;

    let begin = post_json(
        &client,
        &format!("{base_url}/app/registration/begin"),
        &json!({"nonce": nonce}),
        "begin",
    )
    .await?;
    validate_errcode(&begin, "begin")?;
    let poll_state = required_string(
        &begin,
        "device_code",
        "DingTalk returned empty device_code or verification URI",
    )?;
    let scan_url = required_string(
        &begin,
        "verification_uri_complete",
        "DingTalk returned empty device_code or verification URI",
    )?;

    Ok(ProviderFetch { scan_url, poll_state })
}

async fn poll_at(base_url: &str, state: &str) -> Result<ProviderPoll, ApiError> {
    if state.trim().is_empty() {
        return Err(provider_error("DingTalk device_code must not be empty"));
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| provider_error("DingTalk QR client initialization failed"))?;
    let data = post_json(
        &client,
        &format!("{base_url}/app/registration/poll"),
        &json!({"device_code": state}),
        "poll",
    )
    .await?;
    validate_errcode(&data, "poll")?;

    let status = optional_string(&data, "status").to_ascii_uppercase();
    let client_id = optional_string(&data, "client_id");
    let client_secret = optional_string(&data, "client_secret");
    if !client_id.is_empty() && !client_secret.is_empty() {
        let credentials = Map::from_iter([
            ("client_id".to_string(), Value::String(client_id)),
            ("client_secret".to_string(), Value::String(client_secret)),
        ]);
        return Ok(ProviderPoll {
            status: ChannelQrStatus::Authorized,
            credentials: Some(credentials),
        });
    }

    let status = match status.as_str() {
        "EXPIRED" => ChannelQrStatus::Expired,
        "FAIL" | "FAILED" => ChannelQrStatus::Failed,
        _ => ChannelQrStatus::Pending,
    };
    Ok(ProviderPoll { status, credentials: None })
}

async fn post_json(
    client: &reqwest::Client,
    url: &str,
    body: &Value,
    operation: &str,
) -> Result<Value, ApiError> {
    let response = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|_| provider_error(format!("DingTalk {operation} request failed")))?;
    if !response.status().is_success() {
        return Err(provider_error(format!("DingTalk {operation} request failed")));
    }
    response
        .json()
        .await
        .map_err(|_| provider_error(format!("DingTalk {operation} returned invalid JSON")))
}

fn validate_errcode(data: &Value, operation: &str) -> Result<(), ApiError> {
    if data.get("errcode").and_then(Value::as_i64) == Some(0) {
        Ok(())
    } else {
        Err(provider_error(format!("DingTalk {operation} failed")))
    }
}

fn required_string(data: &Value, key: &str, message: &str) -> Result<String, ApiError> {
    let value = optional_string(data, key);
    if value.is_empty() { Err(provider_error(message)) } else { Ok(value) }
}

fn optional_string(data: &Value, key: &str) -> String {
    data.get(key).and_then(Value::as_str).map(str::trim).unwrap_or_default().to_string()
}

fn provider_error(message: impl Into<String>) -> ApiError {
    ApiError::internal(message)
}

#[cfg(test)]
#[path = "dingtalk_tests.rs"]
mod dingtalk_tests;
