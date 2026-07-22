//! 会话 UI 快照相关的 Gateway HTTP 处理函数。
//!
//! 本模块负责把前端使用的会话视图模型读写到本地 `ui_store`，并按照实例目录
//! 或显式 scope 隔离不同工作区的会话列表、归档状态和当前会话范围。文件系统
//! 访问通过无 Tokio 上下文的数据库线程执行，兼容同步 PostgreSQL 客户端。

use axum::Json;
use axum::extract::Path;
use axum::extract::Query;
use axum::http::HeaderMap;
use serde_json::Value;
use vw_api_types::session::GatewaySessionScopeBody;

use super::shared::resolve_scope_from_query;
use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::instance::InstanceQuery;
use crate::app::agent::gateway::instance::resolve_directory;
use crate::session::ui_store;
use crate::session::ui_types as ui_models;

pub(super) async fn session_ui_get(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Option<ui_models::ChatSession>>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let scope = resolve_scope_from_query(&tenant, &query, &headers);
    let id = session_id.clone();
    // `ui_store` 是同步文件存储接口；放入阻塞线程池可以让网关继续处理其他请求。
    let session = crate::global_database::run_isolated(move || {
        ui_store::load_session_scoped_for_tenant(&tenant, &id, scope.as_deref())
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(session))
}

pub(super) async fn session_ui_save(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<ui_models::ChatSession>,
) -> Result<Json<bool>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    // URL 中的 session id 是路由授权和定位依据，保存前必须拒绝正文里伪造的 id。
    if body.id != session_id {
        return Err(ApiError::bad_request("body.id does not match session_id"));
    }

    let directory = resolve_directory(&query, &headers);
    let scope = ui_store::resolve_session_scope_id_for_tenant(&tenant, Some(&directory), None);
    tracing::info!(
        target: "vw_agent",
        session_id = %body.id,
        directory = %directory,
        scope = ?scope,
        message_count = body.messages.len(),
        step_count = body.steps.len(),
        "gateway saving session ui snapshot"
    );
    // 保存完整 UI 快照可能触发磁盘写入，继续隔离到阻塞线程池中执行。
    crate::global_database::run_isolated(move || {
        ui_store::save_session_scoped_for_tenant(&tenant, &body, scope.as_deref())
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(true))
}

pub(super) async fn session_ui_previews(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<ui_models::ChatSessionMeta>>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let scope = resolve_scope_from_query(&tenant, &query, &headers);
    // 预览列表按 scope 读取，防止不同工作区的历史会话在 UI 中互相串见。
    let previews = crate::global_database::run_isolated(move || {
        ui_store::load_sessions_scoped_for_tenant(&tenant, scope.as_deref())
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(previews))
}

pub(super) async fn session_ui_preview_meta(
    Path(session_id): Path<String>,
) -> Result<Json<Option<ui_models::ChatSessionMeta>>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let id = session_id.clone();
    let meta = crate::global_database::run_isolated(move || {
        ui_store::session_preview_meta_for_tenant(&tenant, &id)
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(meta))
}

pub(super) async fn session_archived_get(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<String>>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let scope = resolve_scope_from_query(&tenant, &query, &headers);
    // 归档集合在存储层使用集合语义，对外返回 Vec 便于 JSON 客户端消费。
    let ids = crate::global_database::run_isolated(move || {
        ui_store::load_archived_session_ids_scoped_for_tenant(&tenant, scope.as_deref())
            .into_iter()
            .collect::<Vec<String>>()
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(ids))
}

pub(super) async fn session_archived_put(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<Vec<String>>,
) -> Result<Json<bool>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let scope = resolve_scope_from_query(&tenant, &query, &headers);
    crate::global_database::run_isolated(move || {
        // 写入前去重，保持归档状态是集合而不是顺序敏感列表。
        let set: std::collections::HashSet<String> = body.into_iter().collect();
        ui_store::save_archived_session_ids_scoped_for_tenant(&tenant, &set, scope.as_deref());
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(true))
}

pub(super) async fn session_path_get(
    Path(session_id): Path<String>,
) -> Result<Json<Option<String>>, ApiError> {
    let id = session_id.clone();
    let path = crate::global_database::run_isolated(move || {
        ui_store::session_file_path(&id).map(|p| p.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(path))
}

pub(super) async fn session_step_llm_raw_get(
    Path((session_id, step_index)): Path<(String, u32)>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Option<Value>>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let scope = resolve_scope_from_query(&tenant, &query, &headers);
    let payload = crate::global_database::run_isolated(move || {
        ui_store::load_llm_raw_step_scoped(&session_id, step_index, scope.as_deref())
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?
    .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(payload))
}

pub(super) async fn session_scope_get() -> Result<Json<Option<String>>, ApiError> {
    let scope = crate::global_database::run_isolated(ui_store::current_session_scope)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(scope))
}

pub(super) async fn session_scope_put(
    Json(body): Json<GatewaySessionScopeBody>,
) -> Result<Json<bool>, ApiError> {
    crate::global_database::run_isolated(move || {
        ui_store::set_session_scope(body.scope.as_deref())
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(true))
}

pub(super) async fn session_ui_get_any(
    Path(session_id): Path<String>,
) -> Result<Json<Option<ui_models::ChatSession>>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let id = session_id.clone();
    let session = crate::global_database::run_isolated(move || {
        ui_store::load_session_any_for_tenant(&tenant, &id)
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(session))
}

#[cfg(test)]
#[path = "ui_ops_tests.rs"]
mod ui_ops_tests;
