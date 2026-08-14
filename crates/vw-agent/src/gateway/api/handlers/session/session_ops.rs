//! 会话生命周期、分支、重置和元数据 API。
//!
//! 本模块实现桌面网关的会话列表、状态、创建、更新、删除、fork、reset、diff、
//! summarize 和标题生成入口。涉及项目目录的操作都会进入实例上下文，跨实例查询则
//! 只在未显式指定目录的兼容路径上使用。

use std::collections::HashMap;

use axum::Json;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::HeaderMap;
use vw_api_types::session::{
    GatewaySessionDiffQuery, GatewaySessionForkBody, GatewaySessionPatchBody,
    GatewaySessionResetBody, GatewaySessionSummarizeBody, GatewaySessionTitleGenerateBody,
    GatewaySessionTitleGenerateResponse, GatewaySessionTodoPutBody,
};

use super::shared::{
    UiSessionCreateBody, UiSessionListQuery, has_explicit_directory, lifecycle_api_error,
    session_api_error, with_tenant_instance,
};
use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::AppState;
use crate::app::agent::gateway::chat::{
    fork_session_query_engine, invalidate_session_query_engine,
};
use crate::app::agent::gateway::instance::InstanceQuery;
use crate::app::agent::gateway::instance::resolve_directory;
use crate::app::agent::session as agent_session;
use crate::app::agent::snapshot;
use crate::session::runtime::{CreateSessionInput, SessionScope, create_session, load_session};
use crate::session::ui_store;

fn task_session_summary_needs_refresh(session: &agent_session::session::Info) -> bool {
    if !session.id.starts_with("task-board-") {
        return false;
    }

    session.summary.as_ref().is_none_or(|summary| {
        summary.files == 0 && summary.additions == 0 && summary.deletions == 0
    })
}

async fn refresh_task_session_summaries(sessions: &mut [agent_session::session::Info]) {
    for session in sessions.iter_mut().filter(|session| task_session_summary_needs_refresh(session))
    {
        if let Ok(summary) = agent_session::summary::refresh_session_diff_summary(&session.id).await
            && (summary.files > 0 || session.summary.is_none())
        {
            session.summary = Some(summary);
        }
    }
}

/// 列出 UI 会话。
///
/// # 参数
///
/// * `query` - 目录、根会话、更新时间、搜索和数量限制过滤条件。
///
/// # 返回值
///
/// 返回过滤后的会话信息列表。
pub(super) async fn ui_session_list(
    Query(query): Query<UiSessionListQuery>,
    _headers: HeaderMap,
) -> Result<Json<Vec<agent_session::session::Info>>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let UiSessionListQuery { directory, roots, start, search, limit } = query;
    let directory = directory.filter(|d| !d.trim().is_empty());
    let term = search.map(|s| s.to_ascii_lowercase());
    let limit = limit.unwrap_or(usize::MAX);

    let mut sessions = ui_store::with_tenant_scope(tenant, agent_session::session::list_all())
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    if let Some(directory) = directory.as_deref() {
        sessions.retain(|s| s.directory == directory);
    }

    if roots == Some(true) {
        sessions.retain(|s| s.parent_id.is_none());
    }

    if let Some(start) = start {
        sessions.retain(|s| s.time.updated >= start);
    }

    if let Some(term) = term.as_deref() {
        sessions.retain(|s| s.title.to_ascii_lowercase().contains(term));
    }

    sessions.truncate(limit);
    refresh_task_session_summaries(&mut sessions).await;
    Ok(Json(sessions))
}

/// 获取 UI 会话状态表。
///
/// # 参数
///
/// * `query` - 可选目录过滤。
///
/// # 返回值
///
/// 返回会话 id 到运行状态的映射；没有运行态记录的会话补为 `Idle`。
pub(super) async fn ui_session_status(
    Query(query): Query<InstanceQuery>,
    _headers: HeaderMap,
) -> Result<Json<HashMap<String, agent_session::status::Info>>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let directory = query.directory.filter(|d| !d.trim().is_empty());

    let mut sessions = ui_store::with_tenant_scope(tenant, agent_session::session::list_all())
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    if let Some(directory) = directory.as_deref() {
        sessions.retain(|s| s.directory == directory);
    }

    let mut map = agent_session::status::list();
    for session in sessions {
        map.entry(session.id).or_insert(agent_session::status::Info::Idle);
    }

    Ok(Json(map))
}

/// 获取单个会话信息。
///
/// # 参数
///
/// * `session_id` - 目标会话 id。
/// * `query` - 可选实例目录。
/// * `headers` - 可携带实例目录 header。
///
/// # 返回值
///
/// 返回会话元信息。
///
/// # 错误处理
///
/// 显式目录请求只在该实例内查询；未显式目录时使用兼容的全局查询，找不到时返回 404。
pub(super) async fn ui_session_get(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<agent_session::session::Info>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    if has_explicit_directory(&query, &headers) {
        let dir = resolve_directory(&query, &headers);
        let scope = SessionScope::new(tenant, dir).map_err(lifecycle_api_error)?;
        let snapshot = load_session(scope, &session_id).await.map_err(lifecycle_api_error)?;
        return Ok(Json(snapshot.info));
    }

    let session = ui_store::with_tenant_scope(tenant, agent_session::session::get_any(&session_id))
        .await
        .map_err(session_api_error)?;
    Ok(Json(session))
}

/// 创建一个 UI 会话。
///
/// # 参数
///
/// * `query` - 目标实例目录。
/// * `headers` - 可携带实例目录 header。
/// * `body` - 可选创建参数和权限规则。
///
/// # 返回值
///
/// 返回新建会话信息。
pub(super) async fn ui_session_create(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<Option<UiSessionCreateBody>>,
) -> Result<Json<agent_session::session::Info>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let dir = resolve_directory(&query, &headers);
    let body =
        body.unwrap_or(UiSessionCreateBody { session: Default::default(), permission: None });
    let UiSessionCreateBody { session, permission } = body;
    let vw_api_types::session::GatewaySessionCreateBody { parent_id, title } = session;

    let scope = SessionScope::new(tenant, dir).map_err(lifecycle_api_error)?;
    let created = create_session(CreateSessionInput { scope, parent_id, title, permission })
        .await
        .map_err(lifecycle_api_error)?;
    Ok(Json(created.info))
}

/// 更新会话标题或归档时间。
///
/// # 参数
///
/// * `session_id` - 目标会话 id。
/// * `body` - patch 字段。
///
/// # 返回值
///
/// 返回更新后的会话信息。
pub(super) async fn ui_session_patch(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewaySessionPatchBody>,
) -> Result<Json<agent_session::session::Info>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let title = body.title.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let archived = body.time.and_then(|time| time.archived);

    let update_fn = move |session: &mut agent_session::session::Info| {
        if let Some(title) = title.clone() {
            session.title = title;
        }
        if let Some(archived) = archived {
            // `0` 作为清除归档时间的协议值，避免为 patch 语义再增加额外字段。
            if archived == 0 {
                session.time.archived = None;
            } else {
                session.time.archived = Some(archived);
            }
        }
    };

    let updated = if has_explicit_directory(&query, &headers) {
        let dir = resolve_directory(&query, &headers);
        with_tenant_instance(tenant, dir, move || {
            Box::pin(async move {
                agent_session::session::update(&session_id, update_fn)
                    .await
                    .map_err(session_api_error)
            })
        })
        .await?
    } else {
        ui_store::with_tenant_scope(
            tenant,
            agent_session::session::update_any(&session_id, update_fn),
        )
        .await
        .map_err(session_api_error)?
    };

    Ok(Json(updated))
}

/// 删除一个会话并清理相关查询引擎缓存。
///
/// # 参数
///
/// * `state` - 网关状态，用于失效查询引擎。
/// * `session_id` - 目标会话 id。
///
/// # 返回值
///
/// 删除成功时返回 `true`。
pub(super) async fn ui_session_delete(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<bool>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    if has_explicit_directory(&query, &headers) {
        let dir = resolve_directory(&query, &headers);
        let session_id_for_remove = session_id.clone();
        with_tenant_instance(tenant, dir, move || {
            Box::pin(async move {
                agent_session::session::remove(&session_id_for_remove)
                    .await
                    .map_err(session_api_error)?;
                Ok(())
            })
        })
        .await?;
    } else {
        ui_store::with_tenant_scope(tenant, agent_session::session::remove_any(&session_id))
            .await
            .map_err(session_api_error)?;
    }

    invalidate_session_query_engine(&state, &session_id).await;

    Ok(Json(true))
}

/// 获取会话子分支。
///
/// # 参数
///
/// * `session_id` - 父会话 id。
///
/// # 返回值
///
/// 返回子会话列表。
pub(super) async fn session_children(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<agent_session::session::Info>>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let dir = resolve_directory(&query, &headers);
    let children = with_tenant_instance(tenant, dir, move || {
        Box::pin(async move {
            agent_session::session::children(&session_id)
                .await
                .map_err(|e| ApiError::bad_request(e.to_string()))
        })
    })
    .await?;
    Ok(Json(children))
}

/// 获取会话 TODO 列表。
///
/// # 参数
///
/// * `session_id` - 目标会话 id。
///
/// # 返回值
///
/// 返回该会话保存的 TODO 项。
pub(super) async fn session_todo_get(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<vw_api_types::session::GatewaySessionTodoItem>>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let dir = resolve_directory(&query, &headers);
    let todos = with_tenant_instance(tenant, dir, move || {
        Box::pin(async move { Ok(agent_session::todo::get(&session_id).await) })
    })
    .await?;
    Ok(Json(todos))
}

/// 覆盖保存会话 TODO 列表。
///
/// # 参数
///
/// * `session_id` - 目标会话 id。
/// * `body` - 完整 TODO 列表。
///
/// # 返回值
///
/// 保存成功时返回 `true`。
pub(super) async fn session_todo_put(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewaySessionTodoPutBody>,
) -> Result<Json<bool>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let dir = resolve_directory(&query, &headers);
    with_tenant_instance(tenant, dir, move || {
        Box::pin(async move {
            agent_session::todo::update(agent_session::todo::UpdateInput {
                session_id,
                todos: body.todos,
            })
            .await
            .map_err(|e| ApiError::bad_request(e.to_string()))?;
            Ok(())
        })
    })
    .await?;
    Ok(Json(true))
}

/// 从现有会话创建分支。
///
/// # 参数
///
/// * `state` - 网关状态，用于复制查询引擎上下文。
/// * `session_id` - 源会话 id。
/// * `body` - 可选截止消息 id。
///
/// # 返回值
///
/// 返回新分支会话信息。
pub(super) async fn session_fork(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<Option<GatewaySessionForkBody>>,
) -> Result<Json<agent_session::session::Info>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let dir = resolve_directory(&query, &headers);
    let body = body.unwrap_or_default();
    let session_id_for_fork = session_id.clone();

    let result = with_tenant_instance(tenant, dir, move || {
        Box::pin(async move {
            agent_session::session::fork(&session_id_for_fork, body.message_id.as_deref())
                .await
                .map_err(|e| ApiError::bad_request(e.to_string()))
        })
    })
    .await?;

    fork_session_query_engine(&state, &session_id, &result.id)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(result))
}

/// 重置会话到指定消息，并可选择回滚代码变更。
///
/// # 参数
///
/// * `state` - 网关状态，用于失效查询引擎缓存。
/// * `session_id` - 目标会话 id。
/// * `body` - 重置目标消息和是否回滚代码。
///
/// # 返回值
///
/// 返回重置后的会话信息。
pub(super) async fn session_reset(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewaySessionResetBody>,
) -> Result<Json<agent_session::session::Info>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let dir = resolve_directory(&query, &headers);
    let session_id_for_reset = session_id.clone();
    let result = with_tenant_instance(tenant, dir, move || {
        Box::pin(async move {
            let info = if body.revert_code {
                // 回滚代码后立即清理 revert 产物，保证 UI 看到的是已收敛的会话状态。
                let reverted = agent_session::revert::revert(agent_session::revert::RevertInput {
                    session_id: session_id_for_reset.clone(),
                    message_id: body.message_id.clone(),
                    part_id: None,
                })
                .await
                .map_err(|e| ApiError::bad_request(e.to_string()))?;
                agent_session::revert::cleanup(&reverted)
                    .await
                    .map_err(|e| ApiError::bad_request(e.to_string()))?;
                agent_session::session::get(&session_id_for_reset)
                    .await
                    .map_err(|e| ApiError::bad_request(e.to_string()))?
            } else {
                agent_session::revert::cleanup_after_message(
                    &session_id_for_reset,
                    &body.message_id,
                )
                .await
                .map_err(|e| ApiError::bad_request(e.to_string()))?;
                agent_session::session::get(&session_id_for_reset)
                    .await
                    .map_err(|e| ApiError::bad_request(e.to_string()))?
            };
            Ok(info)
        })
    })
    .await?;

    invalidate_session_query_engine(&state, &session_id).await;
    Ok(Json(result))
}

/// 获取会话相对指定消息的文件差异。
///
/// # 参数
///
/// * `session_id` - 目标会话 id。
/// * `query` - 目录与可选消息 id。
///
/// # 返回值
///
/// 返回文件 diff 列表。
pub(super) async fn session_diff(
    Path(session_id): Path<String>,
    Query(query): Query<GatewaySessionDiffQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<snapshot::FileDiff>>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let dir = resolve_directory(&InstanceQuery { directory: query.directory }, &headers);
    let diffs = with_tenant_instance(tenant, dir, move || {
        Box::pin(async move {
            Ok(agent_session::summary::diff(agent_session::summary::DiffInput {
                session_id,
                message_id: query.message_id,
            })
            .await)
        })
    })
    .await?;
    Ok(Json(diffs))
}

/// 对指定用户消息之前的会话历史执行手动压缩。
///
/// # 参数
///
/// * `session_id` - 目标会话 id。
/// * `body` - 目标消息 id。
///
/// # 返回值
///
/// 返回值表示是否真的替换了历史；无法生成摘要时返回可诊断错误，且保留原历史。
pub(super) async fn session_summarize(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewaySessionSummarizeBody>,
) -> Result<Json<bool>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let dir = resolve_directory(&query, &headers);
    with_tenant_instance(tenant, dir, move || {
        Box::pin(async move {
            let outcome =
                agent_session::compaction::process_session(&session_id, &body.message_id, false)
                    .await
                    .map_err(|error| ApiError::bad_request(error.to_string()))?;
            match outcome {
                agent_session::compaction::CompactionOutcome::Compacted { .. } => Ok(true),
                agent_session::compaction::CompactionOutcome::NotNeeded { .. } => Ok(false),
                agent_session::compaction::CompactionOutcome::Failed { error, .. } => {
                    Err(ApiError::bad_request(error))
                }
            }
        })
    })
    .await
    .map(Json)
}

/// 根据内容生成会话标题。
///
/// # 参数
///
/// * `session_id` - 目标会话 id。
/// * `body` - 待摘要内容及可选模型信息。
///
/// # 返回值
///
/// 返回生成的标题。
pub(super) async fn session_title_generate(
    Path(session_id): Path<String>,
    Json(body): Json<GatewaySessionTitleGenerateBody>,
) -> Result<Json<GatewaySessionTitleGenerateResponse>, ApiError> {
    let title = agent_session::title::generate_from_content(
        session_id,
        body.content,
        body.preferred_model,
        body.acp_agent,
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok(Json(GatewaySessionTitleGenerateResponse { title }))
}

#[cfg(test)]
#[path = "session_ops_tests.rs"]
mod session_ops_tests;
