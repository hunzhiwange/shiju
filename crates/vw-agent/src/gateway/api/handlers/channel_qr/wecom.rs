use std::time::Duration;

use reqwest::Url;
use serde_json::{Map, Value};

use super::{ChannelQrStatus, ProviderFetch, ProviderPoll};
use crate::app::agent::gateway::ApiError;

const WECOM_AUTH_ORIGIN: &str = "https://work.weixin.qq.com";
const WECOM_SOURCE: &str = "vibewindow";

pub(super) async fn fetch() -> Result<ProviderFetch, ApiError> {
    fetch_at(WECOM_AUTH_ORIGIN).await
}

pub(super) async fn poll(state: &str) -> Result<ProviderPoll, ApiError> {
    poll_at(WECOM_AUTH_ORIGIN, state).await
}

async fn fetch_at(base_url: &str) -> Result<ProviderFetch, ApiError> {
    let state = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().timestamp_millis().to_string();
    let mut url = Url::parse(&format!("{base_url}/ai/qc/gen"))
        .map_err(|_| provider_error("WeCom authorization endpoint is invalid"))?;
    url.query_pairs_mut()
        .append_pair("source", WECOM_SOURCE)
        .append_pair("state", &state)
        .append_pair("timestamp", &timestamp);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|_| provider_error("WeCom authorization client is unavailable"))?;
    let html = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| provider_error("WeCom authorization page request failed"))?
        .text()
        .await
        .map_err(|_| provider_error("WeCom authorization page was invalid"))?;
    parse_fetch_html(&html)
}

fn parse_fetch_html(html: &str) -> Result<ProviderFetch, ApiError> {
    let marker = "window.settings";
    let settings = html
        .find(marker)
        .and_then(|start| {
            html[start + marker.len()..].find('=').map(|offset| start + marker.len() + offset + 1)
        })
        .and_then(|start| {
            serde_json::Deserializer::from_str(html[start..].trim_start())
                .into_iter::<Value>()
                .next()
        })
        .transpose()
        .map_err(|_| provider_error("WeCom authorization page settings were invalid"))?
        .and_then(|value| value.as_object().cloned())
        .ok_or_else(|| provider_error("WeCom authorization page settings were invalid"))?;

    let poll_state = required_string(&settings, "scode")?;
    let scan_url = required_string(&settings, "auth_url")?;
    if !is_trusted_wecom_url(&scan_url) {
        return Err(provider_error("WeCom returned an untrusted authorization URL"));
    }
    Ok(ProviderFetch { scan_url, poll_state })
}

async fn poll_at(base_url: &str, state: &str) -> Result<ProviderPoll, ApiError> {
    if state.trim().is_empty() {
        return Err(ApiError::bad_request("WeCom authorization state must not be empty"));
    }
    let mut url = Url::parse(&format!("{base_url}/ai/qc/query_result"))
        .map_err(|_| provider_error("WeCom status endpoint is invalid"))?;
    url.query_pairs_mut().append_pair("scode", state);
    let response: Value = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| provider_error("WeCom authorization client is unavailable"))?
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| provider_error("WeCom status request failed"))?
        .json()
        .await
        .map_err(|_| provider_error("WeCom status response was invalid"))?;
    Ok(map_poll_response(&response))
}

fn map_poll_response(response: &Value) -> ProviderPoll {
    let data = response.get("data").and_then(Value::as_object);
    let status = data
        .and_then(|data| data.get("status"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if status == "confirmed" {
        let bot_info = data.and_then(|data| data.get("bot_info")).and_then(Value::as_object);
        let bot_id = bot_info.and_then(|info| info.get("botid")).and_then(Value::as_str);
        let secret = bot_info.and_then(|info| info.get("secret")).and_then(Value::as_str);
        if let (Some(bot_id), Some(secret)) = (non_empty(bot_id), non_empty(secret)) {
            return ProviderPoll {
                status: ChannelQrStatus::Authorized,
                credentials: Some(Map::from_iter([
                    ("bot_id".to_string(), Value::String(bot_id.to_string())),
                    ("secret".to_string(), Value::String(secret.to_string())),
                ])),
            };
        }
        return failed();
    }

    let status = match status.as_str() {
        "waiting" | "" => ChannelQrStatus::Pending,
        "scanned" => ChannelQrStatus::Scanned,
        "expired" => ChannelQrStatus::Expired,
        _ => ChannelQrStatus::Failed,
    };
    ProviderPoll { status, credentials: None }
}

fn required_string(settings: &Map<String, Value>, key: &str) -> Result<String, ApiError> {
    non_empty(settings.get(key).and_then(Value::as_str))
        .map(str::to_string)
        .ok_or_else(|| provider_error("WeCom authorization page settings were invalid"))
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn is_trusted_wecom_url(value: &str) -> bool {
    Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str() == Some("work.weixin.qq.com")
            && url.port().is_none()
    })
}

fn failed() -> ProviderPoll {
    ProviderPoll { status: ChannelQrStatus::Failed, credentials: None }
}

fn provider_error(message: impl Into<String>) -> ApiError {
    ApiError::internal(message)
}

#[cfg(test)]
#[path = "wecom_tests.rs"]
mod wecom_tests;
