//! 工具相关 API 处理器。
//!
//! 本模块提供网关工具列表和系统诊断接口。

#![cfg(not(target_arch = "wasm32"))]

use agent_config::runtime::Config;
use gateway_types::AppState;
use axum::{Json, extract::State, http::HeaderMap, response::IntoResponse};
use vw_api_tools::ListToolSpecsResponse;

/// 处理网关工具列表 API 请求。
///
/// 返回当前网关已注册的工具规格，供客户端枚举可调用工具。
pub async fn handle_api_tools<S, C, Hooks, H: ToolsHost<S, C, Hooks>>(
    State(state): State<AppState<S, C, Hooks>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = H::require_auth(&state, &headers) {
        return e.into_response();
    }

    let tools = state.tools_registry.iter().map(|tool| tool.to_dto()).collect();

    Json(ListToolSpecsResponse { items: tools }).into_response()
}

/// 处理系统诊断 API 请求。
pub async fn handle_api_doctor<S, C, Hooks, H: ToolsHost<S, C, Hooks>>(
    State(state): State<AppState<S, C, Hooks>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = H::require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.lock().clone();
    let results = H::diagnose(&config);

    let ok_count = results.iter().filter(|r| H::severity(r) == Severity::Ok).count();
    let warn_count = results.iter().filter(|r| H::severity(r) == Severity::Warn).count();
    let error_count = results.iter().filter(|r| H::severity(r) == Severity::Error).count();

    Json(serde_json::json!({
        "results": results,
        "summary": {
            "ok": ok_count,
            "warnings": warn_count,
            "errors": error_count,
        }
    }))
    .into_response()
}

/// Severity projected from the host's existing diagnostic result.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Ok,
    Warn,
    Error,
}

/// Existing authentication and diagnostic operations supplied by the host.
pub trait ToolsHost<S, C, Hooks> {
    type Diagnostic: serde::Serialize;

    fn require_auth(
        state: &AppState<S, C, Hooks>,
        headers: &HeaderMap,
    ) -> Result<(), (axum::http::StatusCode, Json<serde_json::Value>)>;

    fn diagnose(config: &Config) -> Vec<Self::Diagnostic>;
    fn severity(result: &Self::Diagnostic) -> Severity;
}
