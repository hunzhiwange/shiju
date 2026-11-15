//! 处理聊天流式会话事件。
//! 本模块把网关轮询和流式增量落到会话状态，避免 UI 层理解传输细节。

// 任务 11 接入现有流消费路径前，调度器先作为独立状态机由行为测试验证。
#[allow(dead_code)]
pub(crate) mod delta_scheduler;
pub(super) mod draft;
mod permissions;
mod questions;
mod steps;
mod stream_events;
mod todos;

#[cfg(test)]
pub(crate) use stream_events::append_stream_delta_for_performance_probe;

#[cfg(test)]
#[path = "permission_tests.rs"]
mod permission_tests;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use super::ChatMessage;
use crate::app::session_gateway;
use crate::app::{App, Message, models};
use iced::Task;
#[cfg(all(test, not(target_arch = "wasm32")))]
use std::cell::RefCell;
#[cfg(all(test, not(target_arch = "wasm32")))]
use std::sync::Arc;
#[cfg(all(test, not(target_arch = "wasm32")))]
use std::sync::Mutex;

#[cfg(all(test, not(target_arch = "wasm32")))]
type SaveSessionHook = Arc<
    Mutex<Box<dyn FnMut(&models::ChatSession, Option<&str>) -> Option<std::path::PathBuf> + Send>>,
>;

#[cfg(all(test, not(target_arch = "wasm32")))]
thread_local! {
    static SAVE_SESSION_HOOK: RefCell<Option<SaveSessionHook>> = RefCell::new(None);
}

#[cfg(all(test, not(target_arch = "wasm32")))]
struct SaveSessionHookGuard;

#[cfg(all(test, not(target_arch = "wasm32")))]
impl Drop for SaveSessionHookGuard {
    fn drop(&mut self) {
        SAVE_SESSION_HOOK.with(|hook| {
            *hook.borrow_mut() = None;
        });
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(super) fn with_save_session_hook<R>(
    hook: impl FnMut(&models::ChatSession, Option<&str>) -> Option<std::path::PathBuf> + Send + 'static,
    f: impl FnOnce() -> R,
) -> R {
    SAVE_SESSION_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Arc::new(Mutex::new(Box::new(hook))));
    });
    let _guard = SaveSessionHookGuard;
    f()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
fn call_save_session_hook(
    session: &models::ChatSession,
    directory: Option<&str>,
) -> Option<Option<std::path::PathBuf>> {
    SAVE_SESSION_HOOK.with(|hook| {
        hook.borrow().as_ref().map(|hook| {
            let mut hook = hook.lock().expect("save session hook lock");
            hook(session, directory)
        })
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn gateway_save_session_for_stream(
    session: &models::ChatSession,
    directory: Option<&str>,
) -> Result<Option<std::path::PathBuf>, String> {
    #[cfg(test)]
    if let Some(result) = call_save_session_hook(session, directory) {
        return Ok(result);
    }

    session_gateway::gateway_save_session_result(session, directory)
}

#[cfg(not(target_arch = "wasm32"))]
use desktop_session::message::chat::stream::snapshot_save::{StreamSessionSaveResult, STREAM_SESSION_SAVE_COORDINATOR};

#[cfg(not(target_arch = "wasm32"))]
fn stream_session_save_message(completed: Option<StreamSessionSaveResult>) -> Message {
    use desktop_session::message::chat::stream::snapshot_save::{save_notification, SaveNotification};
    match save_notification(completed) {
        SaveNotification::Ack => Message::Chat(ChatMessage::SessionSaveAck),
        SaveNotification::None => Message::None,
        SaveNotification::Failed(error) => Message::Chat(ChatMessage::SessionSaveFailed(error)),
    }
}

/// 模块内可见函数，执行 save_session_task 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub(super) fn save_session_task(
    session: models::ChatSession,
    directory: Option<String>,
) -> Task<Message> {
    Task::perform(
        async move { session_gateway::gateway_save_session_async(&session, directory.as_deref()).await },
        |result| match result {
            Ok(_) => Message::Chat(ChatMessage::SessionSaveAck),
            Err(error) => {
                tracing::warn!(target: "vw_desktop", error = %error, "failed to save session");
                Message::Chat(ChatMessage::SessionSaveFailed(error))
            }
        },
    )
}

/// 模块内可见函数，执行 save_session_task 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn save_session_task(
    session: models::ChatSession,
    directory: Option<String>,
) -> Task<Message> {
    #[cfg(test)]
    let save_hook = SAVE_SESSION_HOOK.with(|hook| hook.borrow().clone());
    Task::perform(
        async move {
            crate::app::message::spawn_blocking_opt(move || {
                let result = STREAM_SESSION_SAVE_COORDINATOR.save(
                    session,
                    directory,
                    |session, directory| {
                        #[cfg(test)]
                        if let Some(hook) = save_hook.as_ref() {
                            let mut hook = hook.lock().expect("save session hook lock");
                            return Ok(hook(session, directory));
                        }
                        let result = gateway_save_session_for_stream(session, directory);
                        if let Err(error) = &result {
                            tracing::warn!(
                                target: "vw_desktop",
                                session_id = %session.id,
                                directory = ?directory,
                                error = %error,
                                "failed to save stream session"
                            );
                        }
                        result
                    },
                );
                Some(result)
            })
            .await
        },
        stream_session_save_message,
    )
}

/// 模块内可见函数，执行 session_directory_for_save 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn session_directory_for_save(app: &App, session_id: &str) -> Option<String> {
    desktop_session::message::chat::stream::projection::directory_for_save(
        app.known_session_directory(session_id), app.project_path.as_deref(),
    )
}

/// 模块内可见函数，执行 now_ms 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn now_ms() -> u64 {
    crate::app::time::now_ms()
}

/// 模块内可见函数，执行 load_session_or_default 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn load_session_or_default(app: &App, id: String) -> models::ChatSession {
    use desktop_session::message::chat::stream::projection::{load_snapshot, ActiveSnapshot};
    let active = (app.active_session_id.as_deref() == Some(id.as_str())).then(|| ActiveSnapshot {
        messages: &app.chat,
        message_ids: &app.chat_message_ids,
        calls: &app.active_session_view_state.calls,
        steps: &app.active_session_view_state.steps,
    });
    let cached = if active.is_none() {
        app.cached_chat_messages(&id).map(|messages| (
            messages.iter().cloned().collect(), app.cached_chat_message_ids(&id),
        ))
    } else { None };
    let info = app.sessions.iter().find(|session| session.id == id);
    let now = if info.is_none() { now_ms() } else { 0 };
    load_snapshot(id, info, active, cached, now)
}

/// 将新增消息按顺序加入流式保存队列，避免快照保存失败后漏掉用户消息。
pub(super) fn register_stream_session_messages(
    app: &mut App,
    session_id: &str,
    start: usize,
    end: usize,
    updated_ms: u64,
) -> Task<Message> {
    for index in start..end {
        stream_events::mark_agent_stream_session_dirty(app, session_id, index, updated_ms, None);
    }
    stream_events::schedule_agent_stream_session_save(app, session_id)
}

/// 启动队列中的下一个请求。
pub(crate) fn start_next(app: &mut App, session_id: &str) -> Task<Message> {
    if app.active_session_id.as_deref() != Some(session_id) {
        return Task::none();
    }
    let runtime = app.get_session_runtime(session_id);
    if runtime.request_lifecycle.is_requesting || runtime.full_access_saving || runtime.full_access_save_failed {
        return Task::none();
    }
    let Some(item) = runtime.request_lifecycle.queue.first().cloned() else {
        return Task::none();
    };
    let runtime = app.get_session_runtime_mut(session_id);
    runtime.request_lifecycle.queue.remove(0);
    super::session::start(app, item, true)
}

/// 公开函数，执行 update 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn update(app: &mut App, message: ChatMessage) -> Task<Message> {
    match message {
        ChatMessage::AgentStreamDelta(id, delta) => {
            stream_events::queue_agent_stream_delta(app, id, delta)
        }
        ChatMessage::McpAppPanelTick => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let mut failed = false;
                for runtime in app.session_runtime_states.values_mut() {
                    runtime.mcp_app_panels.retain(|panel| {
                        failed |= panel.failed();
                        !panel.finished()
                    });
                }
                if failed {
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "MCP 应用加载或通信失败，请重试",
                        vec![],
                    );
                }
            }
            Task::none()
        }
        ChatMessage::AgentToolLifecycle(id, event) => {
            let mut app_task = Task::none();
            #[cfg(not(target_arch = "wasm32"))]
            if event.kind == gateway_chat_stream::GatewayChatToolEventKind::Complete
                && event.status == gateway_chat_stream::GatewayChatToolStatus::Completed
                && let Some(session) = app.find_session_by_request_id(id)
                && let Some(data) = event
                    .payload
                    .get("result")
                    .and_then(|v| v.get("data"))
                    .or_else(|| event.payload.get("data"))
                && let Some(resource) = data.get("mcp_app")
            {
                let launch = crate::app::gateway_client().and_then(|client| {
                    let helper = crate::app::webview_runtime::webview_helper_candidates()
                        .into_iter()
                        .find(|path| path.is_file())
                        .ok_or_else(|| "MCP app helper unavailable".to_string())?;
                    crate::app::components::mcp_app_panel::McpAppPanel::launch(
                        &helper,
                        session.clone(),
                        resource,
                        event
                            .payload
                            .get("input")
                            .and_then(|value| {
                                value
                                    .as_str()
                                    .and_then(|text| serde_json::from_str(text).ok())
                                    .or_else(|| value.is_object().then(|| value.clone()))
                            })
                            .unwrap_or_else(|| serde_json::json!({})),
                        serde_json::json!({"content":data["content"],"structuredContent":data["structured_content"],"isError":data["is_error"]}),
                        client,
                    )
                    .map_err(|error| error.to_string())
                });
                match launch {
                    Ok(panel) => app.get_session_runtime_mut(&session).mcp_app_panels.push(panel),
                    Err(error) => {
                        app_task = app.show_localized_toast(
                            crate::app::state::ToastKind::Error,
                            "MCP 应用加载失败: %s",
                            vec![format!("{}", error).into()],
                        );
                    }
                }
            }
            let timing_task = steps::handle_agent_tool_lifecycle(app, id, &event);
            let delta_task = stream_events::queue_agent_tool_delta(app, id, event.legacy_delta());
            let refresh_task = session_tool_created_session(&event)
                .map(|(session_id, directory)| {
                    Task::perform(
                        async move {
                            let client = crate::app::gateway_client()?;
                            gateway_session::GatewaySessionClient::new(client.transport().clone())
                                .session_get::<session_info::Info>(
                                    &session_id,
                                    directory.as_deref(),
                                )
                                .await
                        },
                        |result| Message::Chat(ChatMessage::VisibleTaskSessionLoaded(result)),
                    )
                })
                .unwrap_or_else(Task::none);
            Task::batch([timing_task, delta_task, refresh_task, app_task])
        }
        ChatMessage::AgentModelRecovery(id, event) => {
            stream_events::handle_agent_model_recovery(app, id, event)
        }
        ChatMessage::AgentTodoContinuation(id) => {
            stream_events::handle_agent_todo_continuation(app, id)
        }
        ChatMessage::AgentCompactionStatus(id, completed) => {
            stream_events::handle_agent_compaction_status(app, id, completed)
        }
        ChatMessage::VisibleTaskSessionLoaded(result) => {
            match result {
                Ok(info) if info.is_visible_task() => {
                    crate::app::message::project::cache_external_session(app, info);
                }
                Ok(info) => tracing::warn!(
                    target: "vw_desktop",
                    session_id = %info.id,
                    "ignored SessionTool refresh for a non-visible session"
                ),
                Err(error) => tracing::warn!(
                    target: "vw_desktop",
                    error = %error,
                    "failed to refresh SessionTool-created session"
                ),
            }
            Task::none()
        }
        ChatMessage::AgentStreamDeltaFlush(session_id) => {
            stream_events::flush_agent_stream_delta(app, session_id)
        }
        ChatMessage::AgentStreamSessionSaveFlush(session_id) => {
            stream_events::flush_agent_stream_session_save(app, session_id)
        }
        ChatMessage::AgentStreamSessionDeltaSaved {
            session_id,
            generation,
            revision,
            updated_ms,
            result,
        } => stream_events::handle_agent_stream_session_delta_saved(
            app, session_id, generation, revision, updated_ms, result,
        ),
        ChatMessage::AgentContextLoaded(id, paths) => {
            stream_events::handle_agent_context_loaded(app, id, paths)
        }
        ChatMessage::AgentWorkflowNodeUpdate(id, raw_tool_block) => {
            stream_events::handle_agent_workflow_node_update(app, id, raw_tool_block)
        }
        ChatMessage::PlanRunSessionStarted {
            title_feedback,
            parent_request_id,
            parent_session_id,
            session_id,
            title,
            directory,
            query,
        } => {
            if let Some(feedback) = title_feedback {
                app.generated_session_titles.insert(session_id.clone(), feedback);
            }
            stream_events::handle_plan_run_session_started(
                app,
                parent_request_id,
                parent_session_id,
                session_id,
                title,
                directory,
                query,
            )
        }
        ChatMessage::PlanRunSessionDelta { session_id, delta } => {
            stream_events::queue_agent_session_delta(app, session_id, delta)
        }
        ChatMessage::PlanRunSessionWaiting(session_id) => {
            handle_plan_run_session_waiting(app, session_id)
        }
        ChatMessage::PlanRunSessionFinished(session_id) => {
            stream_events::handle_plan_run_session_finished(app, session_id)
        }
        ChatMessage::PlanRunSessionCancelled(session_id) => {
            stream_events::handle_plan_run_session_finished(app, session_id)
        }
        ChatMessage::AgentStepStart(id, session_id, step_index, created_ms, model) => {
            let delta_task = stream_events::flush_agent_stream_delta_for_request(app, id);
            Task::batch([
                delta_task,
                steps::handle_agent_step_start(app, id, session_id, step_index, created_ms, model),
            ])
        }
        ChatMessage::AgentStepFinish(
            id,
            session_id,
            step_index,
            finished_ms,
            usage,
            finish_reason,
            model,
        ) => {
            let delta_task = stream_events::flush_agent_stream_delta_for_request(app, id);
            Task::batch([
                delta_task,
                steps::handle_agent_step_finish(
                    app,
                    id,
                    session_id,
                    step_index,
                    finished_ms,
                    usage,
                    finish_reason,
                    model,
                ),
            ])
        }
        ChatMessage::SessionSaveAck => Task::none(),
        ChatMessage::SessionSaveFailed(error) => app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "会话保存失败：%s",
            vec![format!("{}", error).into()],
        ),
        ChatMessage::AgentStepCostLoaded(_id, session_id, step_index, resolved_model, cost) => {
            steps::handle_agent_step_cost_loaded(app, session_id, step_index, resolved_model, cost)
        }
        ChatMessage::AgentPostToolRound(id, session_id, step_index) => {
            stream_events::handle_agent_post_tool_round(app, id, session_id, step_index)
        }
        ChatMessage::SessionSummaryReloaded(result) => {
            match result {
                Ok(info) => {
                    draft::apply_published_title(app, &info);
                    desktop_session::message::chat::stream::projection::apply_summary(
                        app.sessions.iter_mut().chain(app.project_sessions.values_mut().flatten()),
                        &info,
                    );
                }
                Err(error) => tracing::warn!(%error, "failed to reload session summary"),
            }
            Task::none()
        }
        ChatMessage::AgentSessionDiffSummary(id, session_id, summary) => {
            let active_request = app
                .session_runtime_states
                .get(&session_id)
                .and_then(|runtime| runtime.request_lifecycle.active_agent_request.as_ref());
            desktop_session::message::chat::stream::projection::apply_diff_summary(
                app.sessions.iter_mut().chain(app.project_sessions.values_mut().flatten()),
                &session_id, &summary, id, active_request.map(|request| request.id),
            );
            Task::none()
        }
        ChatMessage::AgentStreamDone(id, usage, assistant_message_id, parent_message_id) => {
            stream_events::handle_agent_stream_done(
                app,
                id,
                usage,
                assistant_message_id,
                parent_message_id,
            )
        }
        ChatMessage::DraftResponseReloaded {
            id,
            usage,
            assistant_message_id,
            parent_message_id,
            publication,
        } => stream_events::complete_agent_stream(
            app,
            id,
            usage,
            assistant_message_id,
            parent_message_id,
            publication.map(|value| *value),
        ),
        ChatMessage::PublishedSessionTitleLoaded(info) => {
            draft::apply_published_title(app, &info);
            Task::none()
        }
        ChatMessage::AgentStreamCancelled(id) => handle_agent_stream_cancelled(app, id),
        ChatMessage::AgentStreamStopFinished(_, result) => match result {
            Ok(_) => Task::none(),
            Err(error) => app.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "停止后台请求失败：%s",
                vec![format!("{}", error).into()],
            ),
        },
        ChatMessage::AgentStreamError(id, err) => {
            if err.starts_with("gateway stream failed: 410 ")
                && let Some(session_id) = app.find_session_by_request_id(id)
                && let Some(runtime) = app.session_runtime_states.get(&session_id)
                && let (Some(draft), Some(request)) =
                    (&runtime.draft, &runtime.request_lifecycle.active_agent_request)
            {
                return draft::recover_expired_replay(
                    id,
                    err,
                    session_id,
                    draft.directory.clone(),
                    request.query.clone(),
                );
            }
            stream_events::handle_agent_stream_error(app, id, err)
        }
        ChatMessage::DraftReplayChecked { id, error, recovered } => match recovered {
            Some(recovered) => draft::complete_recovered(app, id, *recovered),
            None => stream_events::handle_agent_stream_error(app, id, error),
        },
        ChatMessage::QuestionPollTick => questions::handle_question_poll_tick(),
        ChatMessage::PermissionPollTick => permissions::handle_permission_poll_tick(app),
        ChatMessage::LoadInputPanelTodos => todos::handle_load_input_panel_todos(app),
        ChatMessage::TodoPollTick => todos::handle_todo_poll_tick(app),
        ChatMessage::QuestionListLoaded(res) => questions::handle_question_list_loaded(app, res),
        ChatMessage::PermissionListLoaded(res) => {
            permissions::handle_permission_list_loaded(app, res)
        }
        ChatMessage::InputPanelTodosLoaded(session_id, res) => {
            todos::handle_input_panel_todos_loaded(app, session_id, res)
        }
        ChatMessage::QuestionOptionToggled(q_idx, label) => {
            questions::handle_question_option_toggled(app, q_idx, label)
        }
        ChatMessage::QuestionCustomChanged(q_idx, value) => {
            questions::handle_question_custom_changed(app, q_idx, value)
        }
        ChatMessage::QuestionSubmit => questions::handle_question_submit(app),
        ChatMessage::QuestionReject => questions::handle_question_reject(app),
        ChatMessage::QuestionReplySubmitted(res) => questions::handle_question_reply_submitted(res),
        ChatMessage::QuestionRejected(res) => questions::handle_question_rejected(res),
        ChatMessage::PermissionApproveOnce => permissions::handle_permission_approve_once(app),
        ChatMessage::PermissionApproveAlways => permissions::handle_permission_approve_always(app),
        ChatMessage::PermissionApproveAllAlways => {
            permissions::handle_permission_approve_all_always(app)
        }
        ChatMessage::PermissionReject => permissions::handle_permission_reject(app),
        ChatMessage::PermissionSelectRequest(request_id) => {
            permissions::handle_permission_select_request(app, request_id)
        }
        ChatMessage::ToggleFullAccessPermission => {
            permissions::handle_toggle_full_access_permission(app)
        }
        ChatMessage::SessionFullAccessLoaded { session_id, full_access } => {
            if app.active_session_id.as_deref() == Some(session_id.as_str()) {
                app.restore_session_full_access(&session_id, full_access);
            }
            Task::none()
        }
        ChatMessage::SessionFullAccessSaved { session_id, previous, result } => {
            permissions::handle_full_access_saved(app, &session_id, previous, result)
        }
        ChatMessage::PermissionReplySubmitted(res) => {
            permissions::handle_permission_reply_submitted(app, res)
        }
        _ => Task::none(),
    }
}

pub(crate) use desktop_session::message::chat::stream::tool_session::session_tool_created_session;

fn handle_plan_run_session_waiting(app: &mut App, session_id: String) -> Task<Message> {
    let session_id = session_id.trim();
    if session_id.is_empty() || !app.sessions.iter().any(|session| session.id == session_id) {
        return Task::none();
    }

    let flush_task = stream_events::flush_agent_stream_delta(app, session_id.to_string());
    app.get_session_runtime_mut(session_id).request_lifecycle.is_requesting = true;
    app.sync_task_pet_from_runtime();
    flush_task
}

fn handle_agent_stream_cancelled(app: &mut App, id: u64) -> Task<Message> {
    let Some(session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    stream_events::clear_model_recovery(app, &session_id);
    stream_events::settle_interrupted_compaction(
        app,
        &session_id,
        crate::app::components::chat_panel::message_view::COMPACTION_CANCELLED_TEXT,
    );
    let _ = stream_events::flush_agent_stream_delta(app, session_id.clone());
    let save_task = stream_events::force_save_agent_stream_session(app, &session_id);
    crate::app::state::clear_pending_guide_handoff(id);
    let is_active_session = app.active_session_id.as_ref() == Some(&session_id);
    {
        let runtime = app.get_session_runtime_mut(&session_id);
        runtime.request_lifecycle.is_requesting = false;
        runtime.request_lifecycle.submit_anim = 0;
        runtime.has_unseen_success = false;
        runtime.request_lifecycle.active_agent_request = None;
    }
    app.sync_task_pet_from_runtime();
    if is_active_session {
        app.show_send_mode_popover = false;
        app.sync_active_session_from_chat();
    }
    save_task.chain(start_next(app, &session_id))
}

/// 清除重试提示并显式停止对应的可恢复流，不能仅取消本地订阅。
pub(super) fn stop_agent_stream(app: &mut App, session_id: &str) -> Task<Message> {
    stream_events::clear_model_recovery(app, session_id);
    let Some(request) = app.get_session_runtime(session_id).request_lifecycle.active_agent_request.clone() else {
        return Task::none();
    };
    let request_id = crate::app::impls::desktop_stream_request_id(&request);
    let workflow = crate::app::impls::task_plan_run_active(
        request.workflow_mode_enabled,
        request.resume_history_only,
    );
    let id = request.id;
    Task::perform(
        async move {
            let client = crate::app::config::gateway_client()?;
            desktop_session::message::chat::stream::projection::stop_stream(&client, &request_id, workflow).await
        },
        move |result| Message::Chat(ChatMessage::AgentStreamStopFinished(id, result)),
    )
}
