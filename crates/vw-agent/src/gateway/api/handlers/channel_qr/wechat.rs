use std::time::Duration;

use serde_json::{Map, Value};
use vw_config_types::channels::DEFAULT_WECHAT_BASE_URL;

use super::{ChannelQrStatus, ProviderFetch, ProviderPoll};
use crate::app::agent::gateway::ApiError;

const WECHAT_SCAN_URL: &str = "https://liteapp.weixin.qq.com/q/7GiQu1";

pub(super) async fn fetch() -> Result<ProviderFetch, ApiError> {
    fetch_at(DEFAULT_WECHAT_BASE_URL).await
}

pub(super) async fn poll(state: &str) -> Result<ProviderPoll, ApiError> {
    poll_at(DEFAULT_WECHAT_BASE_URL, state).await
}

async fn fetch_at(base_url: &str) -> Result<ProviderFetch, ApiError> {
    let response = get_json(
        base_url,
        "ilink/bot/get_bot_qrcode",
        &[("bot_type", "3")],
        "QR code",
        Duration::from_secs(15),
        false,
    )
    .await?;
    let poll_state = required_string(&response, "qrcode", "WeChat returned empty QR code data")?;
    let scan_url =
        format!("{WECHAT_SCAN_URL}?qrcode={}&bot_type=3", urlencoding::encode(&poll_state));
    Ok(ProviderFetch { scan_url, poll_state })
}

async fn poll_at(base_url: &str, state: &str) -> Result<ProviderPoll, ApiError> {
    if state.trim().is_empty() {
        return Err(ApiError::bad_request("WeChat authorization state must not be empty"));
    }
    let response = get_json(
        base_url,
        "ilink/bot/get_qrcode_status",
        &[("qrcode", state)],
        "status",
        Duration::from_secs(45),
        true,
    )
    .await?;
    Ok(map_poll_response(&response))
}

async fn get_json(
    base_url: &str,
    path: &str,
    query: &[(&str, &str)],
    operation: &str,
    timeout: Duration,
    include_client_version: bool,
) -> Result<Value, ApiError> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|_| provider_error("WeChat authorization client is unavailable"))?;
    let mut request = client.get(format!("{}/{path}", base_url.trim_end_matches('/'))).query(query);
    if include_client_version {
        request = request.header("iLink-App-ClientVersion", "1");
    }
    request
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| provider_error(format!("WeChat {operation} request failed")))?
        .json()
        .await
        .map_err(|_| provider_error(format!("WeChat {operation} response was invalid")))
}

fn map_poll_response(response: &Value) -> ProviderPoll {
    let status =
        response.get("status").and_then(Value::as_str).unwrap_or("waiting").to_ascii_lowercase();
    if status == "confirmed" {
        let bot_token = non_empty(response.get("bot_token").and_then(Value::as_str));
        let base_url = non_empty(response.get("baseurl").and_then(Value::as_str));
        if let (Some(bot_token), Some(base_url)) = (bot_token, base_url)
            && base_url.trim_end_matches('/') == DEFAULT_WECHAT_BASE_URL
        {
            return ProviderPoll {
                status: ChannelQrStatus::Authorized,
                credentials: Some(Map::from_iter([
                    ("bot_token".to_string(), Value::String(bot_token.to_string())),
                    ("base_url".to_string(), Value::String(DEFAULT_WECHAT_BASE_URL.to_string())),
                ])),
            };
        }
        return failed();
    }

    let status = match status.as_str() {
        "wait" | "waiting" | "" => ChannelQrStatus::Pending,
        "scaned" | "scanned" => ChannelQrStatus::Scanned,
        "expired" => ChannelQrStatus::Expired,
        _ => ChannelQrStatus::Failed,
    };
    ProviderPoll { status, credentials: None }
}

fn required_string(response: &Value, key: &str, message: &str) -> Result<String, ApiError> {
    non_empty(response.get(key).and_then(Value::as_str))
        .map(str::to_string)
        .ok_or_else(|| provider_error(message))
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn failed() -> ProviderPoll {
    ProviderPoll { status: ChannelQrStatus::Failed, credentials: None }
}

fn provider_error(message: impl Into<String>) -> ApiError {
    ApiError::internal(message)
}

#[cfg(test)]
#[path = "wechat_tests.rs"]
mod wechat_tests;
