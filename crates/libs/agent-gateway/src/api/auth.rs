//! 网关 REST 处理器的 skey 提取与认证边界。
//!
//! 仅接受去除首尾空白后的 Bearer 值；未启用配对认证时处理器直接放行。
//!
//! Gateway skey authentication for REST API handlers.

use gateway_types::AppState;
use axum::{
    http::{HeaderMap, StatusCode},
    response::Json,
};

pub use gateway_types::auth::extract_auth_skey;
#[cfg(test)]
fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    extract_auth_skey(headers)
}

/// 按网关配对状态验证请求，失败时返回可直接转成响应的 401 错误。
pub fn require_auth<S, C, H>(
    state: &AppState<S, C, H>,
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
