//! 工具相关 API 处理器。
//!
//! 本模块提供网关工具列表和系统诊断接口。

use crate::app::agent::gateway::AppState;
use axum::{Json, extract::State, http::HeaderMap, response::IntoResponse};
use vw_api_types::tools::ListToolSpecsResponse;

/// 处理网关工具列表 API 请求。
///
/// 返回当前网关已注册的工具规格，供客户端枚举可调用工具。
pub async fn handle_api_tools(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    let tools = state.tools_registry.iter().map(|tool| tool.to_dto()).collect();

    Json(ListToolSpecsResponse { items: tools }).into_response()
}

/// 处理系统诊断 API 请求。
pub async fn handle_api_doctor(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.lock().clone();
    let results = crate::app::agent::doctor::diagnose(&config);

    let ok_count =
        results.iter().filter(|r| r.severity == crate::app::agent::doctor::Severity::Ok).count();
    let warn_count =
        results.iter().filter(|r| r.severity == crate::app::agent::doctor::Severity::Warn).count();
    let error_count =
        results.iter().filter(|r| r.severity == crate::app::agent::doctor::Severity::Error).count();

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

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tools_tests;
