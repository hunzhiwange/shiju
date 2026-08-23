//! 网关 REST 处理器的 skey 提取与认证边界。
//!
//! Gateway skey authentication for REST API handlers.

use super::super::AppState;
use axum::{
    http::{HeaderMap, StatusCode, header},
    response::Json,
};

pub fn extract_auth_skey(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .and_then(|auth| auth.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|skey| !skey.is_empty())
}

#[cfg(test)]
fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    extract_auth_skey(headers)
}

pub fn require_auth(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if !state.pairing.auth_enabled() {
        return Ok(());
    }

    let skey = extract_auth_skey(headers).unwrap_or("");
    if state.pairing.is_authenticated(skey) {
        Ok(())
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized — send a valid skey as Authorization: Bearer <skey>"
            })),
        ))
    }
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod auth_tests;
