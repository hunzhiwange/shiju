//! 集成 API 处理器
//!
//! 本模块提供与外部系统集成相关的 HTTP API 端点处理器。
//! 主要功能包括：
//! - 列出所有可用的集成及其状态
//! - 获取集成配置设置
//! - 更新集成凭据信息
//!
//! 这些处理器遵循统一的认证和错误处理模式，确保 API 的安全性和一致性。

use super::super::types::IntegrationCredentialsUpdateBody;
use crate::app::agent::config::schema::save_config;
use crate::app::agent::gateway::AppState;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json as JsonResponse},
};

/// 处理获取所有集成列表的 API 请求
///
/// 该端点返回系统中所有可用集成的列表，包括每个集成的名称、描述、类别和当前状态。
/// 需要通过认证才能访问。
///
/// # 参数
///
/// - `State(state)` - 应用共享状态，包含配置和其他全局资源
/// - `headers` - HTTP 请求头，用于认证验证
///
/// # 返回
///
/// 返回 JSON 格式的集成列表，结构如下：
/// ```json
/// {
///     "integrations": [
///         {
///             "name": "集成名称",
///             "description": "集成描述",
///             "category": "集成类别",
///             "status": "集成状态"
///         }
///     ]
/// }
/// ```
///
/// # 认证
///
/// 需要有效的认证令牌，否则返回 401 未授权错误。
pub async fn handle_api_integrations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // 验证请求的认证信息，失败则直接返回错误响应
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    // 获取当前配置的克隆，避免长时间持有锁
    let config = state.config.lock().clone();
    // 从集成注册表中获取所有可用的集成条目
    let entries = agent_integrations::registry::all_integrations();

    // 遍历每个集成条目，调用其状态函数获取当前状态，并构建 JSON 对象
    let integrations: Vec<serde_json::Value> = entries
        .iter()
        .map(|entry| {
            // 使用集成的状态检查函数获取当前配置下的集成状态
            let status = (entry.status_fn)(&config);
            serde_json::json!({
                "name": entry.name,
                "description": entry.description,
                "category": entry.category,
                "status": status,
            })
        })
        .collect();

    // 返回包含所有集成信息的 JSON 响应
    JsonResponse(serde_json::json!({"integrations": integrations})).into_response()
}

/// 处理获取集成设置的 API 请求
///
/// 该端点返回当前系统中所有集成的配置设置信息。
/// 用于前端展示和编辑集成配置的表单。
///
/// # 参数
///
/// - `State(state)` - 应用共享状态，包含配置和其他全局资源
/// - `headers` - HTTP 请求头，用于认证验证
///
/// # 返回
///
/// 返回 JSON 格式的集成设置数据，具体结构由 `build_integration_settings_payload` 函数定义。
///
/// # 认证
///
/// 需要有效的认证令牌，否则返回 401 未授权错误。
pub async fn handle_api_integrations_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // 验证请求的认证信息，失败则直接返回错误响应
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    // 获取当前配置的克隆，避免长时间持有锁
    let config = state.config.lock().clone();
    // 构建集成设置的数据载荷
    let payload = super::super::integrations::build_integration_settings_payload(&config);
    // 返回设置数据的 JSON 响应
    JsonResponse(payload).into_response()
}

/// 处理更新集成凭据的 API 请求（PUT 方法）
///
/// 该端点允许更新指定集成的凭据信息。支持乐观并发控制，
/// 通过版本号检查确保配置的一致性，防止并发更新冲突。
///
/// # 参数
///
/// - `State(state)` - 应用共享状态，包含配置和其他全局资源
/// - `headers` - HTTP 请求头，用于认证验证
/// - `Path(id)` - 路径参数，要更新的集成 ID
/// - `JsonResponse(body)` - 请求体，包含要更新的字段和可选的版本号
///
/// # 返回
///
/// 成功时返回：
/// ```json
/// {
///     "status": "ok",
///     "revision": "新版本号",
///     "unchanged": true  // 仅当配置未实际改变时存在
/// }
/// ```
///
/// 失败时可能返回：
/// - `404 NOT_FOUND` - 未知的集成 ID
/// - `400 BAD_REQUEST` - 不支持的字段或无效的配置更新
/// - `409 CONFLICT` - 配置版本过期，需要刷新后重试
/// - `500 INTERNAL_SERVER_ERROR` - 配置保存失败或其他内部错误
///
/// # 认证
///
/// 需要有效的认证令牌，否则返回 401 未授权错误。
///
/// # 并发控制
///
/// 使用乐观锁机制：客户端可以提供当前配置的版本号，
/// 服务器会检查该版本号是否与当前版本匹配。
/// 如果不匹配，说明配置已被其他客户端修改，
/// 需要客户端刷新配置后重新提交。
pub async fn handle_api_integration_credentials_put(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    JsonResponse(body): JsonResponse<IntegrationCredentialsUpdateBody>,
) -> impl IntoResponse {
    // 验证请求的认证信息，失败则直接返回错误响应
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    // 获取当前配置的克隆，用于后续处理
    let current = state.config.lock().clone();
    // 获取当前配置的版本号，用于乐观并发控制
    let current_revision = super::super::integrations::config_revision(&current);

    // 如果客户端提供了版本号，检查是否与当前版本匹配
    // 这是一种乐观锁机制，用于检测并发修改冲突
    if let Some(revision) = body.revision.as_deref() {
        if revision != current_revision {
            // 版本不匹配，说明配置已被其他客户端修改
            // 返回 409 冲突错误，提示客户端需要刷新配置
            return (
                StatusCode::CONFLICT,
                JsonResponse(serde_json::json!({
                    "error": "Integration settings are out of date. Refresh and retry.",
                    "revision": current_revision,
                })),
            )
                .into_response();
        }
    }

    // 尝试应用集成凭据更新
    let updated = match super::super::integrations::apply_integration_credentials_update(
        &current,
        &id,
        &body.fields,
    ) {
        Ok(config) => config,
        // 处理未知集成 ID 的情况
        Err(error) if error.starts_with("Unknown integration id:") => {
            return (StatusCode::NOT_FOUND, JsonResponse(serde_json::json!({ "error": error })))
                .into_response();
        }
        // 处理不支持的字段错误
        Err(error) if error.starts_with("Unsupported field") => {
            return (StatusCode::BAD_REQUEST, JsonResponse(serde_json::json!({ "error": error })))
                .into_response();
        }
        // 处理无效的配置更新
        Err(error) if error.starts_with("Invalid integration config update:") => {
            return (StatusCode::BAD_REQUEST, JsonResponse(serde_json::json!({ "error": error })))
                .into_response();
        }
        // 处理其他未预期的错误
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(serde_json::json!({ "error": error })),
            )
                .into_response();
        }
    };

    // 获取更新后配置的版本号
    let updated_revision = super::super::integrations::config_revision(&updated);

    // 检查配置是否实际发生了变化
    // 如果版本号相同，说明字段值没有实际改变
    if updated_revision == current_revision {
        return JsonResponse(serde_json::json!({
            "status": "ok",
            "revision": updated_revision,
            "unchanged": true,
        }))
        .into_response();
    }

    // 将更新后的配置保存到持久化存储
    if let Err(error) = save_config(&updated).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(serde_json::json!({"error": format!("Failed to save config: {error}")})),
        )
            .into_response();
    }

    // 更新内存中的配置，使其与持久化存储保持一致
    *state.config.lock() = updated;

    // 返回成功响应，包含新的配置版本号
    JsonResponse(serde_json::json!({
        "status": "ok",
        "revision": updated_revision,
    }))
    .into_response()
}

#[cfg(test)]
#[path = "integrations_tests.rs"]
mod integrations_tests;
