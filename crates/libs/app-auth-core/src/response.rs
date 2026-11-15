//! Bounded upstream authentication responses and token validation.

use reqwest::Response;
use serde::de::DeserializeOwned;
use std::time::{SystemTime, UNIX_EPOCH};
use vw_api_app_auth::AuthErrorCode;
use vw_api_transport::TimestampMs;

const MAX_RESPONSE_BYTES: usize = 16 * 1024;

pub async fn read_json<T: DeserializeOwned>(response: Response) -> Result<T, AuthErrorCode> {
    read_json_limited(response, MAX_RESPONSE_BYTES).await
}

pub async fn read_json_limited<T: DeserializeOwned>(
    response: Response,
    max_bytes: usize,
) -> Result<T, AuthErrorCode> {
    let body = read_body_limited(response, max_bytes).await?;
    serde_json::from_slice(&body).map_err(|_| AuthErrorCode::UpstreamFailure)
}

pub async fn read_body_limited(
    mut response: Response,
    max_bytes: usize,
) -> Result<Vec<u8>, AuthErrorCode> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| AuthErrorCode::UpstreamFailure)? {
        if chunk.len() > max_bytes - body.len() {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pub fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RESPONSE_BYTES
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

pub fn utc_now() -> Result<TimestampMs, AuthErrorCode> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
        .map(TimestampMs)
        .ok_or(AuthErrorCode::ExecutionHostUnavailable)
}
