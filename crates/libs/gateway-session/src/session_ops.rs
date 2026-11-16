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
use vw_api_session::session::{
    GatewaySessionDiffQuery, GatewaySessionForkBody, GatewaySessionForkTarget,
    GatewaySessionPatchBody, GatewaySessionResetBody, GatewaySessionSummarizeBody,
    GatewaySessionTitleGenerateBody, GatewaySessionTitleGenerateResponse,
    GatewaySessionTodoPutBody,
};

use super::shared::{
    UiSessionCreateBody, UiSessionListQuery, has_explicit_directory, lifecycle_api_error,
    session_api_error,
};
use gateway_types::ApiError;
use gateway_instance::host::InstanceQuery;
use agent_project::{self as project, instance};
use snapshot_types as snapshot;
use session_port::{CreateSessionInput, SessionScope};
use crate::session_host::{SessionHost, PreparedTurnFork};
use session_info as session;
use session_engine::message;

pub fn task_session_summary_needs_refresh(session: &session::Info) -> bool {
    if !session.id.starts_with("task-board-") {
        return false;
    }

    session.summary.as_ref().is_none_or(|summary| {
        summary.files == 0 && summary.additions == 0 && summary.deletions == 0
    })
}

async fn refresh_task_session_summaries(
    sessions: &mut [session::Info],
    refresh: impl AsyncFn(&str) -> Result<session::Summary, String>,
) {
    for session in sessions.iter_mut().filter(|session| task_session_summary_needs_refresh(session))
    {
        if let Ok(summary) = refresh(&session.id).await
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
pub async fn ui_session_list<H: SessionHost>(
    Query(query): Query<UiSessionListQuery>,
    _headers: HeaderMap,
) -> Result<Json<Vec<session::Info>>, ApiError> {
    let tenant = H::current_tenant_scope();
    let UiSessionListQuery { directory, project_id, roots, start, search, limit } = query;
    let directory = directory.filter(|d| !d.trim().is_empty());
    let term = search.map(|s| s.to_ascii_lowercase());
    let limit = limit.unwrap_or(usize::MAX);

    let mut sessions = if let Some(project_id) = project_id {
        if project_id.as_ref().trim().is_empty() {
            return Err(ApiError::bad_request("projectID cannot be blank"));
        }
        H::with_tenant_scope(tenant, H::list_for_project(project_id.0))
            .await
    } else {
        H::with_tenant_scope(tenant, H::list_all()).await
    }
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
    refresh_task_session_summaries(&mut sessions, async |id| H::refresh_summary(id).await).await;
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
pub async fn ui_session_status<H: SessionHost>(
    Query(query): Query<InstanceQuery>,
    _headers: HeaderMap,
) -> Result<Json<HashMap<String, session_port::SessionStatus>>, ApiError> {
    let tenant = H::current_tenant_scope();
    let directory = query.directory.filter(|d| !d.trim().is_empty());

    let mut sessions = H::with_tenant_scope(tenant, H::list_all())
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    if let Some(directory) = directory.as_deref() {
        sessions.retain(|s| s.directory == directory);
    }

    let mut map = H::status_list();
    for session in sessions {
        map.entry(session.id).or_insert(session_port::SessionStatus::Idle);
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
pub async fn ui_session_get<H: SessionHost>(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<session::Info>, ApiError> {
    let tenant = H::current_tenant_scope();
    if has_explicit_directory(&query, &headers) {
        let dir = H::resolve_directory(&query, &headers);
        let scope = SessionScope::new(tenant, dir).map_err(lifecycle_api_error)?;
        let snapshot = H::load_session(scope, &session_id).await.map_err(lifecycle_api_error)?;
        return Ok(Json(snapshot.info));
    }

    let session = H::with_tenant_scope(tenant, H::get_any(&session_id))
        .await
        .map_err(session_api_error)?;
    Ok(Json(session))
}

/// 保存当前会话的权限选择。
pub async fn session_full_access_put<H: SessionHost>(
    path: Path<String>,
    query: Query<InstanceQuery>,
    headers: HeaderMap,
    Json(full_access): Json<bool>,
) -> Result<Json<bool>, ApiError> {
    let Json(session) = ui_session_get::<H>(path, query, headers).await?;
    H::resolve_full_access(
        &session.project_id,
        &session.id,
        Some(full_access),
        false,
    )
    .await
    .map(Json)
    .map_err(|error| ApiError::internal(error.to_string()))
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
pub async fn ui_session_create<H: SessionHost>(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<Option<UiSessionCreateBody>>,
) -> Result<Json<session::Info>, ApiError> {
    let tenant = H::current_tenant_scope();
    let dir = H::resolve_directory(&query, &headers);
    let body =
        body.unwrap_or(UiSessionCreateBody { session: Default::default(), permission: None });
    let UiSessionCreateBody { session, permission } = body;
    let vw_api_session::session::GatewaySessionCreateBody { parent_id, title } = session;

    let scope = SessionScope::new(tenant, dir).map_err(lifecycle_api_error)?;
    let title = title.filter(|title| !title.trim().is_empty());
    let named = title.is_some();
    let input = CreateSessionInput { scope, parent_id, title, permission };
    let created = if named {
        H::create_session(input).await
    } else {
        H::reserve_session(input).await
    }
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
pub async fn ui_session_patch<H: SessionHost>(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewaySessionPatchBody>,
) -> Result<Json<session::Info>, ApiError> {
    let tenant = H::current_tenant_scope();
    let title = body.title.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let archived = body.time.and_then(|time| time.archived);

    let update_fn = move |session: &mut session::Info| {
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
        let dir = H::resolve_directory(&query, &headers);
        H::with_tenant_instance(tenant, dir, move || {
            Box::pin(async move {
                H::update(&session_id, update_fn)
                    .await
                    .map_err(session_api_error)
            })
        })
        .await?
    } else {
        H::with_tenant_scope(
            tenant,
            H::update_any(&session_id, update_fn),
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
pub async fn ui_session_delete<H: SessionHost>(
    State(state): State<H::State>,
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<bool>, ApiError> {
    let tenant = H::current_tenant_scope();
    if has_explicit_directory(&query, &headers) {
        let dir = H::resolve_directory(&query, &headers);
        let session_id_for_remove = session_id.clone();
        H::with_tenant_instance(tenant, dir, move || {
            Box::pin(async move {
                H::remove(&session_id_for_remove)
                    .await
                    .map_err(session_api_error)?;
                Ok(())
            })
        })
        .await?;
    } else {
        H::with_tenant_scope(tenant, H::remove_any(&session_id))
            .await
            .map_err(session_api_error)?;
    }

    H::invalidate_engine(&state, &session_id).await;

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
pub async fn session_children<H: SessionHost>(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<session::Info>>, ApiError> {
    let tenant = H::current_tenant_scope();
    let dir = H::resolve_directory(&query, &headers);
    let children = H::with_tenant_instance(tenant, dir, move || {
        Box::pin(async move {
            H::children(&session_id)
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
pub async fn session_todo_get<H: SessionHost>(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<vw_api_session::session::GatewaySessionTodoItem>>, ApiError> {
    let tenant = H::current_tenant_scope();
    let dir = H::resolve_directory(&query, &headers);
    let todos = H::with_tenant_instance(tenant, dir, move || {
        Box::pin(async move { Ok(H::todo_get(&session_id).await) })
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
pub async fn session_todo_put<H: SessionHost>(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewaySessionTodoPutBody>,
) -> Result<Json<bool>, ApiError> {
    let tenant = H::current_tenant_scope();
    let dir = H::resolve_directory(&query, &headers);
    H::with_tenant_instance(tenant, dir, move || {
        Box::pin(async move {
            H::todo_update(session_id, body.todos)
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
pub async fn session_fork<H: SessionHost>(
    State(state): State<H::State>,
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<Option<GatewaySessionForkBody>>,
) -> Result<Json<session::Info>, ApiError> {
    let tenant = H::current_tenant_scope();
    let dir = H::resolve_directory(&query, &headers);
    let body = body.unwrap_or_default();
    if let Some(boundary) = body.before_turn_id {
        if body.message_id.is_some() {
            return Err(ApiError::bad_request("beforeTurnId and messageID are mutually exclusive"));
        }
        if boundary.trim().is_empty() {
            return Err(ApiError::bad_request("beforeTurnId must be a persisted user message ID"));
        }
        let directory = has_explicit_directory(&query, &headers).then_some(dir);
        let target = body.target.unwrap_or(GatewaySessionForkTarget::SameDirectory);
        // A disconnected HTTP client must not cancel the operation between writes and cleanup.
        let result = tokio::spawn(H::with_tenant_scope(
            tenant.clone(),
            fork_before_turn::<H>(tenant, directory, session_id, boundary, target),
        ))
        .await
        .map_err(|error| ApiError::internal(error.to_string()))??;
        return Ok(Json(result));
    }
    if body.target.is_some() {
        return Err(ApiError::bad_request("a fork target requires beforeTurnId"));
    }
    let session_id_for_fork = session_id.clone();

    let result = H::with_tenant_instance(tenant, dir, move || {
        Box::pin(async move {
            H::fork(&session_id_for_fork, body.message_id.as_deref())
                .await
                .map_err(|e| ApiError::bad_request(e.to_string()))
        })
    })
    .await?;

    H::fork_engine(&state, &session_id, &result.id)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(result))
}

/// 将会话历史截断到指定消息。
///
/// # 参数
///
/// * `state` - 网关状态，用于失效查询引擎缓存。
/// * `session_id` - 目标会话 id。
/// * `body` - 重置目标消息。
///
/// # 返回值
///
/// 返回重置后的会话信息。
pub async fn session_reset<H: SessionHost>(
    State(state): State<H::State>,
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewaySessionResetBody>,
) -> Result<Json<session::Info>, ApiError> {
    let tenant = H::current_tenant_scope();
    let dir = H::resolve_directory(&query, &headers);
    let session_id_for_reset = session_id.clone();
    let result = H::with_tenant_instance(tenant, dir, move || {
        Box::pin(async move {
            H::cleanup_after_message(&session_id_for_reset, &body.message_id)
                .await
                .map_err(|e| ApiError::bad_request(e.to_string()))?;
            H::get(&session_id_for_reset)
                .await
                .map_err(|e| ApiError::bad_request(e.to_string()))
        })
    })
    .await?;

    H::invalidate_engine(&state, &session_id).await;
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
pub async fn session_diff<H: SessionHost>(
    Path(session_id): Path<String>,
    Query(query): Query<GatewaySessionDiffQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<snapshot::FileDiff>>, ApiError> {
    let tenant = H::current_tenant_scope();
    let dir = H::resolve_directory(&InstanceQuery { directory: query.directory }, &headers);
    let diffs = H::with_tenant_instance(tenant, dir, move || {
        Box::pin(async move {
            Ok(H::diff(session_id, query.message_id)
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
pub async fn session_summarize<H: SessionHost>(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewaySessionSummarizeBody>,
) -> Result<Json<bool>, ApiError> {
    let tenant = H::current_tenant_scope();
    let dir = H::resolve_directory(&query, &headers);
    H::with_tenant_instance(tenant, dir, move || {
        Box::pin(async move {
            let outcome =
                H::process_session(&session_id, &body.message_id, false)
                    .await
                    .map_err(|error| ApiError::bad_request(error.to_string()))?;
            match outcome {
                session_port::CompactionOutcome::Compacted { .. } => Ok(true),
                session_port::CompactionOutcome::NotNeeded { .. } => Ok(false),
                session_port::CompactionOutcome::Failed { error, .. } => {
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
/// * `body` - 旧客户端的兼容请求及可选模型信息。
///
/// # 返回值
///
/// 返回生成的标题。
pub async fn session_title_generate<H: SessionHost>(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewaySessionTitleGenerateBody>,
) -> Result<Json<GatewaySessionTitleGenerateResponse>, ApiError> {
    let tenant = H::current_tenant_scope();
    let directory = if has_explicit_directory(&query, &headers) {
        H::resolve_directory(&query, &headers)
    } else {
        H::get_any(&session_id).await.map_err(session_api_error)?.directory
    };
    let scope = SessionScope::new(tenant.clone(), directory).map_err(lifecycle_api_error)?;
    let directory = scope.directory().to_string_lossy().into_owned();
    H::with_tenant_instance(tenant.clone(), directory, move || {
        Box::pin(async move {
            let info = H::get(&session_id).await.map_err(session_api_error)?;
            let recorded_directory = std::fs::canonicalize(&info.directory)
                .map_err(|_| ApiError::not_found("session directory is unavailable"))?;
            if recorded_directory != scope.directory() {
                return Err(ApiError::not_found("session is outside the title directory"));
            }
            if info.is_first_response_pending() {
                return Err(ApiError::conflict("session has no committed first response"));
            }

            let committed = agent_database::run_isolated(move || {
                H::has_committed_first_response(&tenant, &info.project_id, &info.id)
                    .map(|committed| committed.then_some(info.title))
            })
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
            .map_err(|error| ApiError::internal(error.to_string()))?;
            if let Some(title) = committed {
                if title.is_empty() {
                    return Err(ApiError::conflict("the committed session has no generated title"));
                }
                return Ok(Json(GatewaySessionTitleGenerateResponse { title }));
            }

            // Legacy sessions must prove a successful native turn before invoking a model.
            let history = message::messages(&session_id, None)
                .await
                .map_err(|error| ApiError::internal(error.to_string()))?;
            let content = legacy_successful_title_source::<H>(&history)
                .ok_or_else(|| ApiError::conflict("session has no successful turn with text"))?;
            let title = H::generate_title(
                session_id,
                content,
                body.preferred_model,
                body.acp_agent,
            )
            .await
            .map_err(ApiError::bad_request)?;
            Ok(Json(GatewaySessionTitleGenerateResponse { title }))
        })
    })
    .await
}

pub fn legacy_successful_title_source<H: SessionHost>(history: &[message::WithParts]) -> Option<String> {
    use message::{Info, Part};

    let text = |entry: &message::WithParts| {
        entry.parts.iter().find_map(|part| match part {
            Part::Text(part) if part.synthetic != Some(true) && part.ignored != Some(true) => {
                let text = H::first_response_source(&part.text);
                (!text.is_empty()).then_some(text)
            }
            _ => None,
        })
    };
    history.iter().find_map(|entry| {
        let Info::Assistant(assistant) = &entry.info else { return None };
        if assistant.error.is_some()
            || assistant.time.completed.is_none()
            || !H::is_successful_finish_reason(assistant.finish.as_deref())
        {
            return None;
        }
        text(entry)?;
        history.iter().find_map(|user| match &user.info {
            Info::User(info) if info.id == assistant.parent_id => text(user),
            _ => None,
        })
    })
}

async fn fork_before_turn<H: SessionHost>(
    tenant: vw_api_transport::TenantScope,
    directory: Option<String>,
    session_id: String,
    boundary: String,
    target: GatewaySessionForkTarget,
) -> Result<session::Info, ApiError> {
    let directory = match directory {
        Some(directory) => directory,
        None => {
            H::get_any(&session_id).await.map_err(session_api_error)?.directory
        }
    };
    let (project, worktree) = project::from_directory(&tenant, &directory)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let context = instance::Context { directory, worktree, project };
    instance::provide_context(context.clone(), async move {
        let fork = H::prepare_fork(&session_id, &boundary)
            .await
            .map_err(|error| ApiError::bad_request(error.to_string()))?;
        match target {
            GatewaySessionForkTarget::SameDirectory => {
                #[cfg(not(target_arch = "wasm32"))]
                H::ensure_ready(fork.source_directory())
                    .map_err(|error| ApiError::conflict(error.to_string()))?;
                let context = instance::Context { directory: fork.source_directory().to_owned(), ..context };
                instance::provide_context(context, fork.persist())
                    .await
                    .map_err(|error| ApiError::internal(error.to_string()))
            }
            GatewaySessionForkTarget::Worktree { name } => {
                #[cfg(target_arch = "wasm32")]
                {
                    let _ = name;
                    Err(ApiError::bad_request("worktree targets are unavailable on WASM"))
                }
                #[cfg(not(target_arch = "wasm32"))]
                {

                    let checkout = H::create_checkout(name.as_deref())
                        .await
                        .map_err(|error| ApiError::bad_request(error.to_string()))?;
                    let result = async {
                        let (project, worktree) = project::from_directory(&tenant, &checkout.directory)
                            .await
                            .map_err(|error| ApiError::internal(error.to_string()))?;
                        let context = instance::Context { directory: worktree.clone(), worktree, project };
                        instance::provide_context(context, fork.persist())
                            .await
                            .map_err(|error| ApiError::internal(error.to_string()))
                    }
                    .await;
                    if result.is_err()
                        && let Err(error) = H::remove_checkout(checkout.directory).await
                    {
                        tracing::warn!(%error, "failed to remove worktree after session fork failure");
                    }
                    result
                }
            }
        }
    })
    .await
}

#[cfg(test)]
#[path = "host_migration_10_tests/summary.rs"]
mod host_migration_10_tests;
