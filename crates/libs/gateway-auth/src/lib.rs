//! 认证凭据管理 API。
//!
//! 该模块提供 provider 级认证信息的写入和删除接口，网关层只负责路由、
//! 反序列化和错误映射，实际凭据持久化由 `auth` 模块处理。

#![cfg(not(target_arch = "wasm32"))]

use axum::Json;
use axum::Router;
use axum::extract::Path;
use axum::routing::put;

use agent_auth as auth;
use gateway_types::ApiError;

/// 构建认证管理路由。
///
/// # 返回值
///
/// 返回包含 `PUT /auth/{provider_id}` 和 `DELETE /auth/{provider_id}` 的 Axum 路由。
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/auth/{provider_id}", put(auth_set).delete(auth_remove))
}

async fn auth_set(
    Path(provider_id): Path<String>,
    Json(info): Json<auth::Info>,
) -> Result<Json<bool>, ApiError> {
    auth::set(&provider_id, &info).map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(true))
}

async fn auth_remove(Path(provider_id): Path<String>) -> Result<Json<bool>, ApiError> {
    auth::remove(&provider_id).map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(true))
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod auth_tests;
