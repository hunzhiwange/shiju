//! MCP 检查接口，认证后枚举工作区服务器及其公开工具。
//!
//! 检查使用运行时工作区配置，连接或协议错误映射为请求错误而不泄露凭据。
//!
//! MCP inspection API handlers.

use crate::app::agent::gateway::AppState;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

/// 认证后检查已配置 MCP 服务器并列出连接成功服务器公开的工具。
pub async fn handle_api_mcp_inspect(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    let workspace_dir = state.config.lock().workspace_dir.clone();
    match crate::app::agent::tools::mcp_inspect::inspect_mcp_servers_from_workspace(&workspace_dir)
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

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod mcp_tests;

#[derive(serde::Deserialize)]
pub struct AppToolCall {
    session: String,
    server: String,
    binding: String,
    name: String,
    arguments: serde_json::Value,
}

/// This endpoint accepts only a previously issued session binding, never an MCP URL.
pub async fn handle_api_mcp_app_call(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AppToolCall>,
) -> impl IntoResponse {
    if let Err(error) = super::super::auth::require_auth(&state, &headers) {
        return error.into_response();
    }
    let tenant = crate::session::ui_store::current_tenant_scope();
    let result = tokio::task::spawn_blocking(move || {
        crate::session::ui_store::with_tenant_scope_sync(tenant, || {
            crate::plugins::mcp_app_execution::call(
                &body.session,
                &body.server,
                &body.binding,
                &body.name,
                body.arguments,
            )
        })
    })
    .await;
    match result {
        Ok(Ok(result)) => Json(result).into_response(),
        _ => (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"MCP app call denied"})))
            .into_response(),
    }
}
