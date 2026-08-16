//! 配置 API 处理器模块
//!
//! 本模块提供配置管理相关的 HTTP API 端点处理函数。
//! 支持配置的读取和更新操作，并对敏感字段进行自动掩码和水合处理。
//!
//! # 主要功能
//!
//! - **配置读取**：获取当前运行时配置，敏感字段（如密钥、令牌）自动掩码
//! - **配置更新**：接收新配置，自动水合敏感字段，验证后持久化保存
//!
//! # API 端点
//!
//! - `GET /api/config` - 获取当前配置（TOML 格式，敏感字段已掩码）
//! - `PUT /api/config` - 更新配置（保留现有敏感字段值）
//!
//! # 安全特性
//!
//! - 所有端点均需要身份验证
//! - 读取配置时自动掩码敏感字段，防止密钥泄露
//! - 更新配置时自动水合（保留）现有敏感字段值
//! - 配置保存前进行完整验证

use crate::app::agent::gateway::AppState;
use axum::{
    Json, Router,
    extract::{Extension, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use vw_api_types::id::TenantScope;

use crate::app::agent::config;
use crate::app::agent::config::schema::{save_config, validate_config};
use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::instance::InstanceQuery;
use crate::app::agent::gateway::instance::resolve_directory;
use crate::app::agent::gateway::instance::with_instance;
use crate::app::agent::provider;

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/config", get(config_get).patch(config_patch))
        .route("/config/providers", get(config_providers))
}

pub(crate) fn stateful_router() -> Router<AppState> {
    Router::new()
        .route("/config", get(config_get_stateful).patch(config_patch_stateful))
        .route("/config/providers", get(config_providers))
}

async fn config_get(
    Extension(tenant): Extension<TenantScope>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    if !has_explicit_directory(&query, &headers) {
        let cfg = config::get_global_for_tenant(&tenant)
            .await
            .map_err(|error| ApiError::bad_request(error.to_string()))?;
        return Ok(Json(
            serde_json::to_value(cfg).map_err(|error| ApiError::internal(error.to_string()))?,
        ));
    }

    let dir = resolve_directory(&query, &headers);
    let project_root = PathBuf::from(&dir);
    let cfg = with_instance(dir, move || {
        Box::pin(async move {
            config::get_project_for_tenant(&tenant, &project_root)
                .await
                .map_err(|error| ApiError::bad_request(error.to_string()))
        })
    })
    .await?;
    Ok(Json(serde_json::to_value(cfg).map_err(|e| ApiError::internal(e.to_string()))?))
}

async fn config_patch(
    Extension(tenant): Extension<TenantScope>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (next, _) = apply_config_patch(&tenant, &query, &headers, patch).await?;
    Ok(Json(serde_json::to_value(next).map_err(|e| ApiError::internal(e.to_string()))?))
}

async fn config_get_stateful(
    tenant: Extension<TenantScope>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    config_get(tenant, Query(query), headers).await
}

async fn config_patch_stateful(
    tenant: Extension<TenantScope>,
    State(state): State<AppState>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (next, updates_active_config) =
        apply_config_patch(&tenant, &query, &headers, patch).await?;
    let response = serde_json::to_value(&next).map_err(|e| ApiError::internal(e.to_string()))?;

    if updates_active_config {
        state.pairing.update_from_skeys(next.gateway.auth_enabled, &next.gateway.skeys);
        *state.config.lock() = next;
    }

    Ok(Json(response))
}

async fn apply_config_patch(
    tenant: &TenantScope,
    query: &InstanceQuery,
    headers: &HeaderMap,
    patch: Value,
) -> Result<(config::Config, bool), ApiError> {
    let updates_active_config = !has_explicit_directory(query, headers);
    if updates_active_config {
        config::update_global_for_tenant(tenant, patch)
            .await
            .map_err(|error| ApiError::bad_request(error.to_string()))?;
        let next = config::get_global_for_tenant(tenant)
            .await
            .map_err(|error| ApiError::bad_request(error.to_string()))?;
        return Ok((next, true));
    }

    let dir = resolve_directory(query, headers);
    let project_root = PathBuf::from(&dir);
    let tenant = tenant.clone();
    let next = with_instance(dir, move || {
        Box::pin(async move {
            config::update_project_for_tenant(&tenant, &project_root, patch)
                .await
                .map_err(|error| ApiError::bad_request(error.to_string()))
        })
    })
    .await?;
    Ok((next, false))
}

fn has_explicit_directory(query: &InstanceQuery, headers: &HeaderMap) -> bool {
    query.directory.as_deref().is_some_and(|value| !value.trim().is_empty())
        || headers
            .get("x-vibewindow-directory")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| !value.trim().is_empty())
}

#[derive(Debug, Serialize)]
struct ConfigProvidersResponse {
    providers: Vec<provider::provider::Info>,
    default: std::collections::HashMap<String, String>,
}

async fn config_providers(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<ConfigProvidersResponse>, ApiError> {
    let dir = resolve_directory(&query, &headers);
    let result = with_instance(dir, move || {
        Box::pin(async move {
            let providers = provider::provider::list().await;
            let mut out = Vec::new();
            let mut defaults = std::collections::HashMap::new();
            for p in providers.values() {
                out.push(p.clone());
                let models =
                    provider::provider::sort(p.models.values().cloned().collect::<Vec<_>>());
                if let Some(m) = models.first() {
                    defaults.insert(p.id.clone(), m.id.clone());
                }
            }
            Ok(ConfigProvidersResponse { providers: out, default: defaults })
        })
    })
    .await?;
    Ok(Json(result))
}

/// 处理配置获取请求 (GET /api/config)
///
/// 返回当前运行时配置的 TOML 表示。敏感字段（如 API 密钥、令牌等）
/// 将自动被掩码替换，以防止在日志或响应中泄露敏感信息。
///
/// # 参数
///
/// - `State(state)` - 应用共享状态，包含当前配置的引用
/// - `headers` - HTTP 请求头，用于身份验证
///
/// # 返回值
///
/// 成功时返回 JSON 响应，包含：
/// - `format`: 字符串 "toml"，表示配置格式
/// - `content`: TOML 格式的配置内容（敏感字段已掩码）
///
/// # 错误
///
/// - `401 Unauthorized` - 身份验证失败
/// - `500 Internal Server Error` - 配置序列化失败
///
/// # 示例响应
///
/// ```json
/// {
///   "format": "toml",
///   "content": "[agent]\nname = \"my-agent\"\napi_key = \"***MASKED***\"\n"
/// }
/// ```
pub async fn handle_api_config_get(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // 验证请求身份，失败则直接返回认证错误响应
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    // 获取当前配置的克隆副本，避免长时间持有锁
    let mut config = state.config.lock().clone();
    config.gateway.skeys = match config::load_gateway_skeys_for_tenant(&tenant).await {
        Ok(skeys) => skeys,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    serde_json::json!({"error": format!("Failed to load gateway skeys: {error}")}),
                ),
            )
                .into_response();
        }
    };

    // 对敏感字段进行掩码处理，防止密钥泄露
    let masked_config = super::super::secrets::mask_sensitive_fields(&config);

    // 将配置序列化为 TOML 格式字符串
    let toml_str = match toml::to_string_pretty(&masked_config) {
        Ok(s) => s,
        Err(e) => {
            // 序列化失败，返回内部服务器错误
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to serialize config: {e}")})),
            )
                .into_response();
        }
    };

    // 返回 TOML 格式的配置内容
    Json(serde_json::json!({
        "format": "toml",
        "content": toml_str,
    }))
    .into_response()
}

/// 处理配置更新请求 (PUT /api/config)
///
/// 接收 TOML 格式的新配置，进行验证后保存。对于请求中未包含或使用占位符
/// 的敏感字段，将自动从当前配置中水合（继承）其值，确保敏感信息不会丢失。
///
/// # 参数
///
/// - `State(state)` - 应用共享状态，包含当前配置的引用
/// - `headers` - HTTP 请求头，用于身份验证
/// - `body` - 请求体，TOML 格式的新配置内容
///
/// # 返回值
///
/// 成功时返回 JSON 响应：
/// ```json
/// { "status": "ok" }
/// ```
///
/// # 错误
///
/// - `401 Unauthorized` - 身份验证失败
/// - `400 Bad Request` - TOML 格式无效或配置验证失败
/// - `500 Internal Server Error` - 配置保存失败
///
/// # 处理流程
///
/// 1. 验证请求身份
/// 2. 解析 TOML 格式的请求体
/// 3. 规范化配置结构（处理仪表板特定的配置格式）
/// 4. 从当前配置水合敏感字段
/// 5. 验证新配置的完整性和正确性
/// 6. 持久化保存配置到文件
/// 7. 更新内存中的运行时配置
///
/// # 安全特性
///
/// - 敏感字段自动水合：如果新配置中缺少或使用占位符表示敏感字段，
///   将自动从当前配置中继承，避免因配置更新导致密钥丢失
pub async fn handle_api_config_put(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    // 验证请求身份，失败则直接返回认证错误响应
    if let Err(e) = super::super::auth::require_auth(&state, &headers) {
        return e.into_response();
    }

    // 解析 TOML 格式的请求体为通用 TOML 值
    let incoming_toml: toml::Value = match toml::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            // TOML 解析失败，返回请求格式错误
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid TOML: {e}")})),
            )
                .into_response();
        }
    };

    // 将 TOML 值转换为强类型的 Config 结构
    let incoming: crate::app::agent::config::Config = match incoming_toml.try_into() {
        Ok(c) => c,
        Err(e) => {
            // 转换失败，说明配置结构与预期不符
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid TOML: {e}")})),
            )
                .into_response();
        }
    };

    // 获取当前配置，用于水合敏感字段
    let current_config = state.config.lock().clone();

    // 水合敏感字段：从当前配置中继承新配置中缺失或占位的敏感值
    let mut new_config = super::super::secrets::hydrate_config_for_save(incoming, &current_config);
    let runtime_auth_enabled = new_config.gateway.auth_enabled;

    // 验证新配置的完整性和正确性
    if let Err(e) = validate_config(&new_config) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Invalid config: {e}")})),
        )
            .into_response();
    }

    if let Err(e) =
        config::replace_gateway_skeys_for_tenant(&tenant, &new_config.gateway.skeys).await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to save gateway skeys: {e}")})),
        )
            .into_response();
    }
    new_config.gateway.skeys = match config::load_gateway_skeys_for_tenant(&tenant).await {
        Ok(skeys) => skeys,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to load gateway skeys: {e}")})),
            )
                .into_response();
        }
    };
    let runtime_skeys = new_config.gateway.skeys.clone();

    // 持久化保存配置到文件系统
    if let Err(e) = save_config(&new_config).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to save config: {e}")})),
        )
            .into_response();
    }

    // 更新内存中的运行时配置，使新配置立即生效
    *state.config.lock() = new_config;
    state.pairing.update_from_skeys(runtime_auth_enabled, &runtime_skeys);

    // 返回成功响应
    Json(serde_json::json!({"status": "ok"})).into_response()
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
