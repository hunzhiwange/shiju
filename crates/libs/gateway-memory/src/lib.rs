#![cfg(not(target_arch = "wasm32"))]
//! 记忆查询、写入与删除的网关处理器，并统一认证和错误映射。
//!
//! 列表接口有查询词时走语义召回，否则按可选分类枚举；所有入口先执行网关认证。
//!
//! Memory API handlers.

use gateway_types::AppState;
use memory_types::memory::MemoryCategory;
use serde::Deserialize;

/// 宿主保留原有配对认证策略。
pub trait MemoryAuth {
    fn require_auth<S, C, H>(state: &AppState<S, C, H>, headers: &axum::http::HeaderMap)
        -> Result<(), (axum::http::StatusCode, axum::Json<serde_json::Value>)>;
}

/// 内存查询请求参数
///
/// 用于查询存储在内存系统中的数据，支持按关键词和类别进行过滤。
///
/// # 示例
///
/// ```json
/// {
///     "query": "用户偏好设置",
///     "category": "preferences"
/// }
/// ```
#[derive(Deserialize)]
pub struct MemoryQuery {
    /// 查询关键词或短语
    ///
    /// 可选参数，用于模糊匹配内存中的内容。
    /// 如果不提供，则可能返回所有或默认数量的记录。
    pub query: Option<String>,

    /// 数据类别过滤器
    ///
    /// 可选参数，用于限定查询的数据类别。
    /// 例如："preferences"、"history"、"cache" 等。
    pub category: Option<String>,
}

/// 内存存储请求体
///
/// 用于向内存系统存储新的数据条目，包含键、内容和可选的类别标签。
///
/// # 示例
///
/// ```json
/// {
///     "key": "user_theme",
///     "content": "dark",
///     "category": "preferences"
/// }
/// ```
#[derive(Deserialize)]
pub struct MemoryStoreBody {
    /// 数据的唯一标识键
    ///
    /// 必填参数，用于后续查询或更新此数据条目。
    /// 建议使用有意义且唯一的命名，如 "user_settings.theme"。
    pub key: String,

    /// 要存储的实际内容
    ///
    /// 必填参数，支持任意文本格式的内容。
    /// 对于结构化数据，建议使用 JSON 字符串格式。
    pub content: String,

    /// 数据类别标签
    ///
    /// 可选参数，用于对存储的数据进行分类管理。
    /// 便于后续按类别批量查询或清理数据。
    pub category: Option<String>,
}

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json as JsonResponse},
};

/// 查询或按分类列出记忆条目。
pub async fn handle_api_memory_list<A: MemoryAuth, S, C, H>(
    State(state): State<AppState<S, C, H>>,
    headers: HeaderMap,
    Query(params): Query<MemoryQuery>,
) -> impl IntoResponse {
    if let Err(e) = A::require_auth(&state, &headers) {
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
            "core" => MemoryCategory::Core,
            "daily" => MemoryCategory::Daily,
            "conversation" => MemoryCategory::Conversation,
            other => MemoryCategory::Custom(other.to_string()),
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
pub async fn handle_api_memory_store<A: MemoryAuth, S, C, H>(
    State(state): State<AppState<S, C, H>>,
    headers: HeaderMap,
    JsonResponse(body): JsonResponse<MemoryStoreBody>,
) -> impl IntoResponse {
    if let Err(e) = A::require_auth(&state, &headers) {
        return e.into_response();
    }

    let category = body
        .category
        .as_deref()
        .map(|cat| match cat {
            "core" => MemoryCategory::Core,
            "daily" => MemoryCategory::Daily,
            "conversation" => MemoryCategory::Conversation,
            other => MemoryCategory::Custom(other.to_string()),
        })
        .unwrap_or(MemoryCategory::Core);

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
pub async fn handle_api_memory_delete<A: MemoryAuth, S, C, H>(
    State(state): State<AppState<S, C, H>>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = A::require_auth(&state, &headers) {
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
