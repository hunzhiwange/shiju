//! MCP 检查接口，认证后枚举工作区服务器及其公开工具。
//!
//! 检查使用运行时工作区配置，连接或协议错误映射为请求错误而不泄露凭据。
//!
//! MCP inspection API handlers.

use gateway_types::AppState;
use super::host::McpHost;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

/// 认证后检查已配置 MCP 服务器并列出连接成功服务器公开的工具。
pub async fn handle_api_mcp_inspect<H: McpHost, S: Clone, C: Clone, K: Clone>(
    State(state): State<AppState<S, C, K>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let workspace_dir = state.config.lock().workspace_dir.clone();
    match H::inspect(&workspace_dir)
        .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Failed to inspect MCP servers: {error}")
            })),
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
pub struct AppToolCall {
    pub session: String,
    pub server: String,
    pub binding: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// This endpoint accepts only a previously issued session binding, never an MCP URL.
pub async fn handle_api_mcp_app_call<H: McpHost, S: Clone, C: Clone, K: Clone>(
    State(state): State<AppState<S, C, K>>,
    headers: HeaderMap,
    Json(body): Json<AppToolCall>,
) -> impl IntoResponse {
    if let Err(error) = require_auth(&state, &headers) {
        return error.into_response();
    }
    let tenant = H::current_tenant();
    let result = tokio::task::spawn_blocking(move || H::call(tenant, &body.session, &body.server, &body.binding, &body.name, body.arguments))
    .await;
    match result {
        Ok(Ok(result)) => Json(result).into_response(),
        _ => (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"MCP app call denied"})))
            .into_response(),
    }
}

fn extract_auth_skey(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .and_then(|auth| auth.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|skey| !skey.is_empty())
}

fn require_auth<S: Clone, C: Clone, K: Clone>(
    state: &AppState<S, C, K>,
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
