//! 记忆查询、写入与删除的网关处理器，并统一认证和错误映射。
//!
//! 列表接口有查询词时走语义召回，否则按可选分类枚举；所有入口先执行网关认证。
//!
//! Memory API handlers.

use super::super::types::{MemoryQuery, MemoryStoreBody};
use crate::app::agent::gateway::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json as JsonResponse},
};

/// 查询或按分类列出记忆条目。
pub async fn handle_api_memory_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<MemoryQuery>,
) -> impl IntoResponse {
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    if let Some(ref query) = params.query {
        match state.mem.recall(query, 50, None).await {
            Ok(entries) => JsonResponse(serde_json::json!({"entries": entries})).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(serde_json::json!({"error": format!("Memory recall failed: {e}")})),
            )
                .into_response(),
        }
    } else {
        let category = params.category.as_deref().map(|cat| match cat {
            "core" => crate::app::agent::memory::MemoryCategory::Core,
            "daily" => crate::app::agent::memory::MemoryCategory::Daily,
            "conversation" => crate::app::agent::memory::MemoryCategory::Conversation,
            other => crate::app::agent::memory::MemoryCategory::Custom(other.to_string()),
        });

        match state.mem.list(category.as_ref(), None).await {
            Ok(entries) => JsonResponse(serde_json::json!({"entries": entries})).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(serde_json::json!({"error": format!("Memory list failed: {e}")})),
            )
                .into_response(),
        }
    }
}

/// 写入一条记忆并返回持久化结果。
pub async fn handle_api_memory_store(
    State(state): State<AppState>,
    headers: HeaderMap,
    JsonResponse(body): JsonResponse<MemoryStoreBody>,
) -> impl IntoResponse {
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    let category = body
        .category
        .as_deref()
        .map(|cat| match cat {
            "core" => crate::app::agent::memory::MemoryCategory::Core,
            "daily" => crate::app::agent::memory::MemoryCategory::Daily,
            "conversation" => crate::app::agent::memory::MemoryCategory::Conversation,
            other => crate::app::agent::memory::MemoryCategory::Custom(other.to_string()),
        })
        .unwrap_or(crate::app::agent::memory::MemoryCategory::Core);

    match state.mem.store(&body.key, &body.content, category, None).await {
        Ok(()) => JsonResponse(serde_json::json!({"status": "ok"})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(serde_json::json!({"error": format!("Memory store failed: {e}")})),
        )
            .into_response(),
    }
}

/// 删除指定记忆键。
pub async fn handle_api_memory_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    match state.mem.forget(&key).await {
        Ok(deleted) => {
            JsonResponse(serde_json::json!({"status": "ok", "deleted": deleted})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(serde_json::json!({"error": format!("Memory forget failed: {e}")})),
        )
            .into_response(),
    }
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod memory_tests;
