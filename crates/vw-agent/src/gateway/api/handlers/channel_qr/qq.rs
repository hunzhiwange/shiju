use base64::Engine;
use ring::aead::{AES_256_GCM, Aad, LessSafeKey, Nonce, UnboundKey};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{ChannelQrStatus, ProviderFetch, ProviderPoll};
use crate::app::agent::gateway::ApiError;

const PORTAL_BASE: &str = "https://q.qq.com";
const CREATE_PATH: &str = "/lite/create_bind_task";
const POLL_PATH: &str = "/lite/poll_bind_result";
const FRONTEND_PATH: &str = "/qqbot/openclaw/connect.html";

#[derive(Deserialize)]
struct PortalResponse {
    retcode: i64,
    #[serde(default)]
    data: Value,
}

#[derive(Deserialize, Serialize)]
struct PollState {
    task_id: String,
    key: String,
}

pub(super) async fn fetch() -> Result<ProviderFetch, ApiError> {
    fetch_from(&reqwest::Client::new(), PORTAL_BASE).await
}

async fn fetch_from(client: &reqwest::Client, base: &str) -> Result<ProviderFetch, ApiError> {
    let key: [u8; 32] = rand::random();
    let encoded_key = base64::engine::general_purpose::STANDARD.encode(key);
    let response = client
        .post(format!("{base}{CREATE_PATH}"))
        .json(&serde_json::json!({ "key": encoded_key }))
        .send()
        .await
        .map_err(|_| ApiError::internal("QQ bind task request failed"))?;
    if !response.status().is_success() {
        return Err(ApiError::internal("QQ bind task request failed"));
    }
    let response: PortalResponse = response
        .json()
        .await
        .map_err(|_| ApiError::internal("QQ bind task returned an invalid response"))?;
    if response.retcode != 0 {
        return Err(ApiError::internal("QQ bind task failed"));
    }
    let task_id = response.data.get("task_id").and_then(Value::as_str).unwrap_or_default().trim();
    if task_id.is_empty() {
        return Err(ApiError::internal("QQ bind task returned no task id"));
    }

    let poll_state =
        serde_json::to_string(&PollState { task_id: task_id.to_string(), key: encoded_key })
            .map_err(|_| ApiError::internal("QQ bind task state could not be stored"))?;
    let scan_url = format!(
        "{PORTAL_BASE}{FRONTEND_PATH}?task_id={}&_wv=2&source=VibeWindow",
        urlencoding::encode(task_id)
    );
    Ok(ProviderFetch { scan_url, poll_state })
}

pub(super) async fn poll(state: &str) -> Result<ProviderPoll, ApiError> {
    poll_from(&reqwest::Client::new(), PORTAL_BASE, state).await
}

async fn poll_from(
    client: &reqwest::Client,
    base: &str,
    state: &str,
) -> Result<ProviderPoll, ApiError> {
    let state: PollState = serde_json::from_str(state)
        .map_err(|_| ApiError::bad_request("Invalid QQ authorization state"))?;
    let key = base64::engine::general_purpose::STANDARD
        .decode(state.key)
        .map_err(|_| ApiError::bad_request("Invalid QQ authorization state"))?;
    let key: [u8; 32] =
        key.try_into().map_err(|_| ApiError::bad_request("Invalid QQ authorization state"))?;
    let response = client
        .post(format!("{base}{POLL_PATH}"))
        .json(&serde_json::json!({ "task_id": state.task_id }))
        .send()
        .await
        .map_err(|_| ApiError::internal("QQ bind status request failed"))?;
    if !response.status().is_success() {
        return Err(ApiError::internal("QQ bind status request failed"));
    }
    let response: Value = response
        .json()
        .await
        .map_err(|_| ApiError::internal("QQ bind status returned an invalid response"))?;
    Ok(map_poll_response(response, &key))
}

fn map_poll_response(response: Value, key: &[u8; 32]) -> ProviderPoll {
    if response.get("retcode").and_then(Value::as_i64) != Some(0) {
        return failed();
    }
    let data = response.get("data").and_then(Value::as_object);
    let status = data.and_then(|data| data.get("status")).and_then(Value::as_i64);
    match status {
        Some(2) => authorized(data, key),
        Some(3) => ProviderPoll { status: ChannelQrStatus::Expired, credentials: None },
        Some(1) => ProviderPoll { status: ChannelQrStatus::Scanned, credentials: None },
        _ => ProviderPoll { status: ChannelQrStatus::Pending, credentials: None },
    }
}

fn authorized(data: Option<&Map<String, Value>>, key: &[u8; 32]) -> ProviderPoll {
    let Some(data) = data else {
        return failed();
    };
    let app_id = match data.get("bot_appid") {
        Some(Value::String(value)) => value.trim().to_string(),
        Some(Value::Number(value)) => value.to_string(),
        _ => String::new(),
    };
    let encrypted = data.get("bot_encrypt_secret").and_then(Value::as_str).unwrap_or_default();
    if app_id.is_empty() || encrypted.is_empty() {
        return failed();
    }
    let Ok(app_secret) = decrypt_secret(encrypted, key) else {
        return failed();
    };
    if app_secret.is_empty() {
        return failed();
    }

    let mut credentials = Map::new();
    credentials.insert("app_id".to_string(), Value::String(app_id));
    credentials.insert("app_secret".to_string(), Value::String(app_secret));
    if let Some(user_openid) = data.get("user_openid").and_then(Value::as_str) {
        credentials.insert("user_openid".to_string(), Value::String(user_openid.to_string()));
    }
    ProviderPoll { status: ChannelQrStatus::Authorized, credentials: Some(credentials) }
}

fn decrypt_secret(encrypted: &str, key: &[u8; 32]) -> Result<String, ()> {
    let raw = base64::engine::general_purpose::STANDARD.decode(encrypted).map_err(|_| ())?;
    if raw.len() < 28 {
        return Err(());
    }
    let (nonce, ciphertext) = raw.split_at(12);
    let nonce: [u8; 12] = nonce.try_into().map_err(|_| ())?;
    let key = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, key).map_err(|_| ())?);
    let mut ciphertext = ciphertext.to_vec();
    let plaintext = key
        .open_in_place(Nonce::assume_unique_for_key(nonce), Aad::empty(), &mut ciphertext)
        .map_err(|_| ())?;
    std::str::from_utf8(plaintext).map(str::to_string).map_err(|_| ())
}

fn failed() -> ProviderPoll {
    ProviderPoll { status: ChannelQrStatus::Failed, credentials: None }
}

#[cfg(test)]
#[path = "qq_tests.rs"]
mod qq_tests;
