//! 处理聊天流式会话事件。
//! 本模块把网关轮询和流式增量落到会话状态，避免 UI 层理解传输细节。

pub(super) use desktop_session::message::chat::stream::permissions::{split_auto_approved_permission_requests, sync_permission_requests, advance_permission_requests_after_reply};
use super::ChatMessage;
use crate::app::session_gateway;
use crate::app::{App, Message};
use iced::Task;
use {gateway_interaction::PendingPermissionReplyDto, gateway_interaction::PendingPermissionRequestDto};

fn permission_directory(app: &App) -> Option<String> {
    app.active_session_id
        .as_deref()
        .and_then(|session_id| app.known_session_directory(session_id))
        .filter(|directory| !directory.trim().is_empty())
        .or_else(|| app.project_path.clone().filter(|directory| !directory.trim().is_empty()))
}

fn clear_permission_modal(app: &mut App) {
    desktop_session::message::chat::stream::permissions::clear_permission_modal(&mut permission_state(app));
}

fn current_full_access_session_id(app: &App) -> Option<&str> {
    app.current_session_runtime_ref()
        .filter(|runtime| runtime.full_access_enabled)
        .and_then(|_| app.active_session_id.as_deref())
}

fn submit_permission_replies(
    request_ids: Vec<String>,
    reply: PendingPermissionReplyDto,
    directory: Option<String>,
) -> Task<Message> {
    if request_ids.is_empty() {
        return Task::none();
    }

    Task::perform(
        async move {
            for request_id in request_ids {
                session_gateway::gateway_permission_reply_async(
                    &request_id,
                    reply,
                    directory.as_deref(),
                )
                .await?;
            }
            Ok(())
        },
        |res| Message::Chat(ChatMessage::PermissionReplySubmitted(res)),
    )
}

fn apply_current_permission_request(app: &mut App, request_id: Option<String>) {
    desktop_session::message::chat::stream::permissions::apply_current_permission_request(&mut permission_state(app), request_id);
}

fn apply_permission_request(app: &mut App, requests: Vec<PendingPermissionRequestDto>) {
    desktop_session::message::chat::stream::permissions::apply_permission_request(&mut permission_state(app), requests);
}

/// 模块内可见函数，执行 handle_permission_poll_tick 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_permission_poll_tick(app: &App) -> Task<Message> {
    let directory = permission_directory(app);
    Task::perform(session_gateway::gateway_permission_list_owned_async(directory), |res| {
        Message::Chat(ChatMessage::PermissionListLoaded(res))
    })
}

/// 模块内可见函数，执行 handle_permission_list_loaded 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_permission_list_loaded(
    app: &mut App,
    res: Result<Vec<PendingPermissionRequestDto>, String>,
) -> Task<Message> {
    match res {
        Ok(requests) => {
            let directory = permission_directory(app);
            let (requests, auto_approve_request_ids) = split_auto_approved_permission_requests(
                current_full_access_session_id(app),
                requests,
            );
            apply_permission_request(app, requests);
            if !auto_approve_request_ids.is_empty() {
                return submit_permission_replies(
                    auto_approve_request_ids,
                    PendingPermissionReplyDto::Once,
                    directory,
                );
            }
        }
        Err(err) => {
            tracing::warn!(target: "vw_desktop", error = %err, "failed to load permission list via gateway");
            clear_permission_modal(app);
        }
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_permission_select_request 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_permission_select_request(app: &mut App, request_id: String) -> Task<Message> {
    apply_current_permission_request(app, Some(request_id));
    Task::none()
}

/// 模块内可见函数，执行 handle_toggle_full_access_permission 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_toggle_full_access_permission(app: &mut App) -> Task<Message> {
    let session_id = app.active_session_id.clone();
    let directory = permission_directory(app);
    let (full_access_enabled, should_rebuild, saved_session) = {
        let runtime = app.current_session_runtime_mut();
        if runtime.full_access_saving || (runtime.draft.is_some() && runtime.request_lifecycle.is_requesting) {
            return Task::none();
        }
        let saved_session = session_id.clone().filter(|_| runtime.draft.is_none());
        runtime.full_access_enabled = !runtime.full_access_enabled;
        runtime.full_access_overridden = true;
        runtime.full_access_initialized = true;
        runtime.full_access_saving = saved_session.is_some();
        runtime.acp_options_state.options.access_status = None;
        (runtime.full_access_enabled, runtime.acp_agent.is_some(), saved_session)
    };
    if should_rebuild && let Some(session_id) = session_id {
        app.mark_session_acp_rebuild_required(&session_id);
    }
    let remember = if should_rebuild {
        Task::none()
    } else {
        app.last_llm_full_access = Some(full_access_enabled);
        if saved_session.is_some() {
            app.reset_draft_full_access();
        }
        app.save_desktop_preference("last_llm_full_access", serde_json::json!(full_access_enabled))
    };
    let save = saved_session.map_or_else(Task::none, |session_id| {
        let target = session_id.clone();
        let client = crate::app::gateway_client();
        let directory = directory.clone();
        Task::perform(
            async move {
                gateway_session::GatewaySessionClient::new(client?.transport().clone())
                    .session_set_full_access(&target, directory.as_deref(), full_access_enabled)
                    .await
            },
            move |result| {
                Message::Chat(ChatMessage::SessionFullAccessSaved {
                    session_id: session_id.clone(),
                    previous: !full_access_enabled,
                    result,
                })
            },
        )
    });
    if !full_access_enabled {
        return Task::batch([remember, save]);
    }

    let (remaining_requests, auto_approve_request_ids) = split_auto_approved_permission_requests(
        app.active_session_id.as_deref(),
        std::mem::take(&mut app.permission_modal_requests),
    );
    apply_permission_request(app, remaining_requests);
    let replies = submit_permission_replies(
        auto_approve_request_ids,
        PendingPermissionReplyDto::Once,
        directory,
    );
    Task::batch([remember, save, replies])
}

pub(super) fn handle_full_access_saved(
    app: &mut App,
    session_id: &str,
    previous: bool,
    result: Result<bool, String>,
) -> Task<Message> {
    let Some(runtime) = app.session_runtime_states.get_mut(session_id) else {
        return Task::none();
    };
    runtime.full_access_saving = false;
    runtime.full_access_save_failed = result.is_err();
    runtime.full_access_enabled = result.as_ref().copied().unwrap_or(previous);
    match result {
        Ok(_) => super::start_next(app, session_id),
        Err(error) => app.show_error_toast(format!("保存会话权限失败：{error}")),
    }
}

/// 模块内可见函数，执行 handle_permission_approve_once 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_permission_approve_once(app: &mut App) -> Task<Message> {
    submit_permission_reply(app, PendingPermissionReplyDto::Once)
}

/// 模块内可见函数，执行 handle_permission_approve_always 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_permission_approve_always(app: &mut App) -> Task<Message> {
    submit_permission_reply(app, PendingPermissionReplyDto::Always)
}

/// 模块内可见函数，执行 handle_permission_approve_all_always 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_permission_approve_all_always(app: &mut App) -> Task<Message> {
    let directory = permission_directory(app);
    let request_ids = std::mem::take(&mut app.permission_modal_requests)
        .into_iter()
        .map(|request| request.id)
        .collect::<Vec<_>>();
    clear_permission_modal(app);
    submit_permission_replies(request_ids, PendingPermissionReplyDto::Always, directory)
}

/// 模块内可见函数，执行 handle_permission_reject 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_permission_reject(app: &mut App) -> Task<Message> {
    submit_permission_reply(app, PendingPermissionReplyDto::Reject)
}

/// 模块内可见函数，执行 handle_permission_reply_submitted 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_permission_reply_submitted(
    app: &App,
    res: Result<(), String>,
) -> Task<Message> {
    if let Err(err) = res {
        tracing::warn!(target: "vw_desktop", error = %err, "failed to submit permission reply via gateway");
    }
    handle_permission_poll_tick(app)
}

fn submit_permission_reply(app: &mut App, reply: PendingPermissionReplyDto) -> Task<Message> {
    let Some(request_id) = app.permission_modal_request_id.clone() else {
        return Task::none();
    };
    let directory = permission_directory(app);
    let (remaining_requests, selected_request_id) = advance_permission_requests_after_reply(
        Some(request_id.as_str()),
        std::mem::take(&mut app.permission_modal_requests),
    );
    app.permission_modal_requests = remaining_requests;
    apply_current_permission_request(app, selected_request_id);
    submit_permission_replies(vec![request_id], reply, directory)
}
#[cfg(test)]
#[path = "permissions_tests.rs"]
mod permissions_tests;

fn permission_state(app: &mut App) -> desktop_session::message::chat::stream::permissions::PermissionState<'_> {
    desktop_session::message::chat::stream::permissions::PermissionState {
    permission_modal_request_id: &mut app.permission_modal_request_id,
    permission_modal_request: &mut app.permission_modal_request,
    permission_modal_requests: &mut app.permission_modal_requests,
    }
}
