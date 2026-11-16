//! 提供项目消息处理共享辅助函数，集中处理路径、时间和异步任务构造。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use crate::app::i18n::project::ProjectFeedback;
use crate::app::message::project::ProjectMessage;
use crate::app::{App, Message};
use iced::Task;
use vw_api_file::file::ListFilesRequest;
use vw_api_project::ProjectDto;

pub(crate) use desktop_workspace::project_message::attachments::is_supported_image_attachment;
pub(crate) fn collect_local_attachments(app: &App, picked: Vec<String>) -> (Vec<String>, Vec<ProjectFeedback>) {
    desktop_workspace::project_message::attachments::collect_local_attachments(
        &app.files, app.navigation_tabs.project_path.as_deref(), app.multimodal_settings.max_images,
        app.multimodal_settings.max_image_size_mb, picked,
    )
}

/// append_local_attachments 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn append_local_attachments(app: &mut App, picked: Vec<String>) {
    let (accepted, errors) = collect_local_attachments(app, picked);
    if !accepted.is_empty() {
        app.files.extend(accepted);
    }
    if !errors.is_empty() {
        let fallback =
            errors.iter().map(|error| error.fallback.as_str()).collect::<Vec<_>>().join("\n");
        let message = crate::app::i18n::format::Message::lines(
            errors
                .into_iter()
                .map(|error| crate::app::i18n::format::Message::new(error.source, error.arguments))
                .collect(),
        );
        app.push_notification(fallback);
        app.notifications.last_mut().expect("notification was inserted").localized_message =
            Some(message);
    }
}

/// persist_clipboard_image_attachment 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use desktop_workspace::project_message::attachments::persist_clipboard_image_attachment;

pub use desktop_workspace::project_message::data::FileTreeIndexSnapshot;

/// refresh_file_index 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(crate) fn refresh_file_index(app: &App) -> Task<Message> {
    let Some(root) = app.navigation_tabs.project_path.clone() else {
        return Task::none();
    };
    // 耗时或平台相关操作交给异步任务，避免阻塞界面消息循环。
    Task::perform(
        async move {
            let snapshot = refresh_file_index_with_fallback(&root).await;
            ProjectMessage::FileTreeSnapshotReady { project_path: root, snapshot }
        },
        Message::Project,
    )
}

pub(crate) async fn refresh_file_index_with_fallback(project_path: &str) -> FileTreeIndexSnapshot {
    match refresh_gateway_file_index(project_path).await {
        Ok(snapshot) => snapshot,
        Err(error) => {
            if gateway_http::is_gateway_unavailable_error(&error) {
                tracing::debug!(
                    target: "vw_desktop",
                    project_path,
                    error = %error,
                    "failed to refresh file index from gateway, falling back to local scan"
                );
            } else {
                tracing::warn!(
                    target: "vw_desktop",
                    project_path,
                    error = %error,
                    "failed to refresh file index from gateway, falling back to local scan"
                );
            }
            let files = refresh_local_file_index(project_path).await;
            let tree = crate::app::components::file_tree::model::build_file_tree_model(
                project_path,
                &files,
            );
            FileTreeIndexSnapshot { files, tree }
        }
    }
}

async fn refresh_local_file_index(project_path: &str) -> Vec<String> {
    let path = project_path.to_string();
    crate::app::message::spawn_blocking_opt(move || Some(crate::app::refresh_file_index(&path)))
        .await
        .unwrap_or_default()
}

#[allow(dead_code)]
/// unique_name_in_dir 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) use desktop_workspace::project_message::paths::unique_name_in_dir;

#[allow(dead_code)]
/// copy_recursively 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) use desktop_workspace::project_message::paths::copy_recursively;

/// find_session_project_path 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn find_session_project_path(app: &App, session_id: &str) -> Option<String> {
    app.project_sessions
        .iter()
        .find_map(|(path, sessions)| {
            sessions.iter().any(|s| s.id == session_id).then(|| path.clone())
        })
        .or_else(|| app.navigation_tabs.project_path.clone())
}

/// now_ms 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn now_ms() -> u64 {
    crate::app::time::now_ms()
}

fn gateway_client() -> Result<vw_gateway_client::GatewayClient, String> {
    crate::app::gateway_client()
}

pub(crate) use desktop_workspace::project_message::gateway::GatewayFileContext;

/// resolve_gateway_file_context 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn resolve_gateway_file_context(
    project_path: &str,
) -> Result<GatewayFileContext, String> {
    desktop_workspace::project_message::gateway::resolve_gateway_file_context(&gateway_client()?, &crate::app::session_gateway::active_tenant_scope(), project_path).await
}

async fn refresh_gateway_file_index(project_path: &str) -> Result<FileTreeIndexSnapshot, String> {
    let client = gateway_client()?;
    let context = resolve_gateway_file_context(project_path).await?;
    let response = gateway_files::GatewayFilesClient::new(client.transport().clone())
        .file_list(&ListFilesRequest {
            project_id: context.project_id,
            worktree_id: context.worktree_id,
            path: None,
            depth: None,
        })
        .await?;
    Ok(FileTreeIndexSnapshot::from_dto(&context.base_directory, &response.root))
}









use desktop_workspace::project_message::data::map_gateway_project;

pub(crate) fn map_loaded_project_info(
    project_path: String,
    project: ProjectDto,
) -> super::LoadedProjectInfo {
    let current_branch = project.current_branch.clone().filter(|value| !value.trim().is_empty());
    super::LoadedProjectInfo { project_path, info: map_gateway_project(project), current_branch }
}

#[allow(dead_code)]
/// load_gateway_recent_projects 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn load_gateway_recent_projects()
-> Result<Vec<crate::app::RecentProjectMeta>, String> {
    let projects = desktop_workspace::project_message::gateway::load_recent_projects(
        &gateway_client()?, &crate::app::session_gateway::active_tenant_scope(),
    ).await?;

    Ok(projects
        .into_iter()
        .filter(|project| crate::app::projects::is_visible_recent_project_path(&project.directory))
        .map(|project| crate::app::RecentProjectMeta {
            path: project.directory,
            name: project.name,
            task_board_settings: None,
            session_auto_refresh: crate::app::state::default_recent_project_session_auto_refresh(),
            session_refresh_interval_seconds:
                crate::app::state::default_recent_project_session_refresh_interval_seconds(),
            icon: None,
            icon_color: None,
            worktree_start_command: None,
        })
        .collect())
}

/// resolve_gateway_project 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(crate) async fn resolve_gateway_project(project_path: &str) -> Result<ProjectDto, String> {
    desktop_workspace::project_message::gateway::resolve_gateway_project(&gateway_client()?, &crate::app::session_gateway::active_tenant_scope(), project_path).await
}



pub(crate) async fn list_gateway_project_sessions(
    client: &vw_gateway_client::GatewayClient,
    project_path: &str,
) -> Result<Vec<session_info::Info>, String> {
    desktop_workspace::project_message::gateway::list_gateway_project_sessions(client, project_path).await
}

pub(crate) async fn gateway_session_project_path(
    client: &vw_gateway_client::GatewayClient,
    info: &session_info::Info,
) -> Result<String, String> {
    desktop_workspace::project_message::gateway::gateway_session_project_path(client, info).await
}

/// load_project_info_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(crate) fn load_project_info_task(project_path: String) -> Task<Message> {
    Task::perform(
        async move {
            resolve_gateway_project(&project_path)
                .await
                .map(|project| map_loaded_project_info(project_path, project))
        },
        |res| Message::Project(ProjectMessage::ProjectInfoLoaded(res)),
    )
}

/// load_project_worktree_picker_options 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn load_project_worktree_picker_options(
    project_path: &str,
) -> Result<Vec<(String, String)>, String> {
    desktop_workspace::project_message::gateway::load_project_worktree_picker_options(&gateway_client()?, &crate::app::session_gateway::active_tenant_scope(), project_path).await
}

/// create_gateway_session_worktree 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn create_gateway_session_worktree(
    project_path: String,
    requested_name: String,
) -> Result<String, String> {
    desktop_workspace::project_message::gateway::create_gateway_session_worktree(&gateway_client()?, &crate::app::session_gateway::active_tenant_scope(), project_path, requested_name).await
}



/// delete_gateway_worktree 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn delete_gateway_worktree(
    project_path: &str,
    directory: &str,
    force: bool,
) -> Result<(), String> {
    desktop_workspace::project_message::gateway::delete_gateway_worktree(&gateway_client()?, &crate::app::session_gateway::active_tenant_scope(), project_path, directory, force).await
}

/// reset_gateway_worktree 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn reset_gateway_worktree(
    project_path: &str,
    directory: &str,
) -> Result<(), String> {
    desktop_workspace::project_message::gateway::reset_gateway_worktree(&gateway_client()?, &crate::app::session_gateway::active_tenant_scope(), project_path, directory).await
}


/// relative_to_project 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) use desktop_workspace::project_message::paths::relative_to_project;

/// 复制外部本机路径，所有源内容读取和递归写入都由网关负责。
pub(crate) async fn gateway_import_local_paths(
    project_path: &str,
    target_directory: &str,
    source_paths: Vec<String>,
) -> Result<vw_api_file::file::LocalImportResponse, String> {
    desktop_workspace::project_message::gateway::gateway_import_local_paths(&gateway_client()?, &crate::app::session_gateway::active_tenant_scope(), project_path, target_directory, source_paths).await
}

/// gateway_move_path 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn gateway_move_path(
    project_path: &str,
    from_path: &str,
    to_path: &str,
) -> Result<(), String> {
    desktop_workspace::project_message::gateway::gateway_move_path(&gateway_client()?, &crate::app::session_gateway::active_tenant_scope(), project_path, from_path, to_path).await
}

/// gateway_copy_path 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn gateway_copy_path(
    project_path: &str,
    from_path: &str,
    to_path: &str,
) -> Result<(), String> {
    desktop_workspace::project_message::gateway::gateway_copy_path(&gateway_client()?, &crate::app::session_gateway::active_tenant_scope(), project_path, from_path, to_path).await
}

pub(super) async fn gateway_duplicate_path(project_path: &str, path: &str) -> Result<(), String> {
    desktop_workspace::project_message::gateway::gateway_duplicate_path(&gateway_client()?, &crate::app::session_gateway::active_tenant_scope(), project_path, path).await
}

/// gateway_delete_path 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn gateway_delete_path(
    project_path: &str,
    path: &str,
    recursive: bool,
) -> Result<(), String> {
    desktop_workspace::project_message::gateway::gateway_delete_path(&gateway_client()?, &crate::app::session_gateway::active_tenant_scope(), project_path, path, recursive).await
}

/// run_find_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn run_find_task(
    tab_id: String,
    title: String,
    scope_path: String,
    query: String,
    replace_text: String,
    case_sensitive: bool,
    whole_word: bool,
    use_regex: bool,
    files: Vec<String>,
) -> Task<Message> {
    let tab_id_for_msg = tab_id.clone();
    let title_for_msg = title.clone();
    let scope_for_msg = scope_path.clone();
    let query_for_msg = query.clone();
    let replace_for_msg = replace_text.clone();

    Task::perform(
        async move {
            crate::app::message::spawn_blocking_opt(move || {
                Some(desktop_workspace::project_message::search::search(
                    scope_path, query, case_sensitive, whole_word, use_regex, files,
                ))
            })
            .await
            .unwrap_or((
                Vec::new(),
                Some(crate::app::i18n::input::Feedback::new("搜索失败", vec![])),
                false,
            ))
        },
        move |(matches, error, limit_reached)| {
            Message::Project(ProjectMessage::FileTreeFindCompleted {
                tab_id: tab_id_for_msg.clone(),
                title: title_for_msg.clone(),
                scope_path: scope_for_msg.clone(),
                query: query_for_msg.clone(),
                replace_text: replace_for_msg.clone(),
                case_sensitive,
                whole_word,
                use_regex,
                matches,
                error: error.as_ref().map(|feedback| feedback.fallback.clone()),
                localized_error: error,
                limit_reached,
            })
        },
    )
}

/// 后台读取 UI Store，并将完整历史和首屏分块合并为一个冷打开结果。
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)] // Kept for explicit compatibility/import flows; normal opens use bounded pages.
pub(crate) fn load_prepared_session_open_from_ui_store_task(
    project_path: String,
    session_id: String,
    open_generation: u64,
    show_reasoning_summary: bool,
) -> Task<Message> {
    let target_session_id = session_id.clone();
    let target_project_path = project_path.clone();
    Task::perform(
        async move {
            let client = crate::app::gateway_client()?;
            let mut loaded = gateway_session::GatewaySessionClient::new(client.transport().clone())
                .session_ui_get_without_raw(&session_id, Some(&project_path))
                .await?
                .filter(|loaded| !loaded.session.messages.is_empty())
                .ok_or_else(|| "session UI store has no message history".to_string())?;
            let step_llm_raw = std::mem::take(&mut loaded.step_llm_raw);
            let llm_raw_step_indices = std::mem::take(&mut loaded.llm_raw_step_indices);
            let agent_binding = loaded.agent_binding;
            let mut session = loaded.session;

            crate::app::session::backfill_message_models_from_steps(
                &mut session.messages,
                &session.steps,
            );

            crate::app::message::spawn_blocking_opt(move || {
                let mut prepared = crate::app::session::prepare_session_open_from_ui_store(
                    session,
                    show_reasoning_summary,
                );
                prepared.agent_binding = agent_binding;
                prepared.step_llm_raw = step_llm_raw;
                prepared.llm_raw_step_indices = llm_raw_step_indices;
                Some(prepared)
            })
            .await
            .ok_or_else(|| "session UI preparation task did not complete".to_string())
        },
        move |result| {
            Message::Project(ProjectMessage::SessionOpenPrepared {
                target_session_id,
                project_path: target_project_path,
                open_generation,
                source: super::PreparedSessionOpenSource::UiStore,
                result,
            })
        },
    )
}

/// 后台读取远端历史，并在同一后台边界内完成协议转换和首屏 UI 准备。
pub(crate) fn load_prepared_session_open_from_gateway_task(
    project_path: String,
    session_id: String,
    open_generation: u64,
    show_reasoning_summary: bool,
) -> Task<Message> {
    let target_session_id = session_id.clone();
    let target_project_path = project_path.clone();
    Task::perform(
        async move {
            tracing::info!(
                target: "vw_desktop",
                session_id = %session_id,
                project_path = %project_path,
                "loading prepared session history from gateway"
            );
            let client = crate::app::gateway_client()?;
            prepare_gateway_session_open(
                &client,
                &session_id,
                &project_path,
                show_reasoning_summary,
            )
            .await
        },
        move |result| {
            Message::Project(ProjectMessage::SessionOpenPrepared {
                target_session_id,
                project_path: target_project_path,
                open_generation,
                source: super::PreparedSessionOpenSource::RemoteHistory,
                result,
            })
        },
    )
}

pub(crate) async fn prepare_gateway_session_open(
    client: &vw_gateway_client::GatewayClient,
    session_id: &str,
    directory: &str,
    show_reasoning_summary: bool,
) -> Result<crate::app::session::PreparedSessionOpen, String> {
    let (summary, page) = crate::app::session_gateway::gateway_bounded_session_open_with_client(
        client, session_id, directory,
    )
    .await?;
    if summary.id != session_id {
        return Err("Gateway returned history for a different session".into());
    }
    crate::app::message::spawn_blocking_opt(move || {
        Some(crate::app::session::prepare_bounded_session_open(
            summary,
            page,
            show_reasoning_summary,
        ))
    })
    .await
    .ok_or_else(|| "session history preparation task did not complete".to_string())
}

pub(crate) fn load_session_history_page_task(
    project_path: String,
    session_id: String,
    open_generation: u64,
    requested_cursor: String,
) -> Task<Message> {
    let response_session_id = session_id.clone();
    let response_cursor = requested_cursor.clone();
    Task::perform(
        async move {
            let client = crate::app::gateway_client()?;
            let page =
                crate::app::session_gateway::gateway_session_message_page_before_with_client(
                    &client,
                    &session_id,
                    &project_path,
                    requested_cursor,
                )
                .await?;
            Ok(crate::app::session::prepare_timeline_page(&session_id, page))
        },
        move |result| {
            Message::Project(ProjectMessage::SessionHistoryPageLoaded {
                session_id: response_session_id,
                open_generation,
                requested_cursor: response_cursor,
                result,
            })
        },
    )
}

/// prepare_session_ui_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
struct ChatUiChunkSnapshot {
    chunk_start_idx: usize,
    messages: crate::app::session::SharedChatMessages,
}

fn chat_ui_chunk_snapshot(
    chat: &crate::app::session::SharedChatMessages,
    chunk_start_idx: usize,
) -> Option<ChatUiChunkSnapshot> {
    let (chunk_start_idx, chunk_end_idx) =
        crate::app::session::chat_ui_chunk_bounds(chat.len(), chunk_start_idx);
    (chunk_start_idx < chunk_end_idx).then(|| ChatUiChunkSnapshot {
        chunk_start_idx,
        messages: std::sync::Arc::from(&chat[chunk_start_idx..chunk_end_idx]),
    })
}

pub(crate) fn prepare_session_ui_task(
    session_id: String,
    chat: crate::app::session::SharedChatMessages,
    chunk_start_idx: usize,
    is_base: bool,
    show_reasoning_summary: bool,
) -> Task<Message> {
    let Some(snapshot) = chat_ui_chunk_snapshot(&chat, chunk_start_idx) else {
        return Task::done(Message::None);
    };
    Task::perform(
        async move {
            crate::app::message::spawn_blocking_opt(move || {
                Some((
                    session_id,
                    crate::app::session::prepare_chat_ui_chunk_phase(
                        &snapshot.messages,
                        snapshot.chunk_start_idx,
                        is_base,
                        show_reasoning_summary,
                    ),
                ))
            })
            .await
        },
        |res| match res {
            Some((session_id, phase)) => Message::Project(ProjectMessage::SessionUiPrepared {
                session_id,
                stream_generation: None,
                phase,
            }),
            None => Message::None,
        },
    )
}

pub(crate) fn prepare_stream_message_ui_task(
    session_id: String,
    message: crate::app::models::ChatMessage,
    message_idx: usize,
    stream_generation: u64,
    show_reasoning_summary: bool,
) -> Task<Message> {
    Task::perform(
        async move {
            crate::app::message::spawn_blocking_opt(move || {
                Some((
                    session_id,
                    stream_generation,
                    crate::app::session::prepare_chat_ui_message_phase(
                        &message,
                        message_idx,
                        show_reasoning_summary,
                    ),
                ))
            })
            .await
        },
        |res| match res {
            Some((session_id, stream_generation, phase)) => {
                Message::Project(ProjectMessage::SessionUiPrepared {
                    session_id,
                    stream_generation: Some(stream_generation),
                    phase,
                })
            }
            None => Message::None,
        },
    )
}

/// prepare_session_ui_chunks_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn prepare_session_ui_chunks_task(
    session_id: String,
    chat: crate::app::session::SharedChatMessages,
    chunk_starts: Vec<usize>,
    base_chunk_start: Option<usize>,
    show_reasoning_summary: bool,
) -> Task<Message> {
    if chunk_starts.is_empty() {
        return Task::none();
    }

    Task::batch(chunk_starts.into_iter().map(|chunk_start_idx| {
        prepare_session_ui_task(
            session_id.clone(),
            chat.clone(),
            chunk_start_idx,
            base_chunk_start == Some(chunk_start_idx),
            show_reasoning_summary,
        )
    }))
}

#[cfg(test)]
#[path = "helpers_tests.rs"]
mod helpers_tests;
