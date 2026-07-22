//! 处理聊天流式会话事件。
//! 本模块把网关轮询和流式增量落到会话状态，避免 UI 层理解传输细节。

mod permissions;
mod questions;
mod steps;
mod stream_events;
mod todos;

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
use vw_shared::session::session_utils::is_default_title;

#[cfg(all(test, not(target_arch = "wasm32")))]
type SaveSessionHook =
    Box<dyn FnMut(&models::ChatSession, Option<&str>) -> Option<std::path::PathBuf>>;

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
    hook: impl FnMut(&models::ChatSession, Option<&str>) -> Option<std::path::PathBuf> + 'static,
    f: impl FnOnce() -> R,
) -> R {
    SAVE_SESSION_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(hook));
    });
    let _guard = SaveSessionHookGuard;
    f()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
fn call_save_session_hook(
    session: &models::ChatSession,
    directory: Option<&str>,
) -> Option<Option<std::path::PathBuf>> {
    SAVE_SESSION_HOOK.with(|hook| hook.borrow_mut().as_mut().map(|hook| hook(session, directory)))
}

#[cfg(not(target_arch = "wasm32"))]
fn gateway_save_session_for_stream(
    session: &models::ChatSession,
    directory: Option<&str>,
) -> Option<std::path::PathBuf> {
    #[cfg(test)]
    if let Some(result) = call_save_session_hook(session, directory) {
        return result;
    }

    session_gateway::gateway_save_session(session, directory)
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
        |result| {
            if let Err(error) = result {
                tracing::warn!(target: "vw_desktop", error = %error, "failed to save session");
            }
            Message::Chat(ChatMessage::SessionSaveAck)
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
    Task::perform(
        async move {
            let result = crate::app::message::spawn_blocking_opt(move || {
                Some(gateway_save_session_for_stream(&session, directory.as_deref()))
            })
            .await
            .flatten();
            if result.is_none() {
                tracing::warn!(target: "vw_desktop", "failed to save session");
            }
        },
        |_| Message::Chat(ChatMessage::SessionSaveAck),
    )
}

/// 模块内可见函数，执行 session_directory_for_save 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn session_directory_for_save(app: &App, session_id: &str) -> Option<String> {
    app.known_session_directory(session_id)
        .filter(|directory| !directory.trim().is_empty())
        .or_else(|| app.project_path.clone().filter(|directory| !directory.trim().is_empty()))
}

/// 模块内可见函数，执行 now_ms 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn now_ms() -> u64 {
    crate::app::time::now_ms()
}

/// 模块内可见函数，执行 load_session_or_default 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn load_session_or_default(app: &App, id: String) -> models::ChatSession {
    let (title, created_ms, updated_ms) = app
        .sessions
        .iter()
        .find(|s| s.id == id)
        .map(|s| (s.title.clone(), s.time.created, s.time.updated))
        .unwrap_or_else(|| ("新会话".to_string(), now_ms(), 0));
    let is_active_session = app.active_session_id.as_deref() == Some(id.as_str());
    let (messages, message_ids, calls, steps) = if is_active_session {
        (
            app.chat.clone(),
            app.chat_message_ids.clone(),
            app.active_session_view_state.calls.clone(),
            app.active_session_view_state.steps.clone(),
        )
    } else if let Some(cached_chat) = app.cached_chat_messages(&id) {
        let messages = cached_chat.iter().cloned().collect::<Vec<_>>();
        let message_ids = app
            .cached_chat_message_ids(&id)
            .filter(|ids| ids.len() == messages.len())
            .unwrap_or_else(|| vec![None; messages.len()]);
        (messages, message_ids, Vec::new(), Vec::new())
    } else {
        (Vec::new(), Vec::new(), Vec::new(), Vec::new())
    };
    models::ChatSession { id, title, messages, message_ids, calls, steps, created_ms, updated_ms }
}

/// 模块内可见函数，执行 start_next 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn start_next(app: &mut App, session_id: &str) -> Task<Message> {
    let runtime = app.get_session_runtime(session_id);
    let Some(item) = runtime.queue.first().cloned() else {
        return Task::none();
    };
    let runtime = app.get_session_runtime_mut(session_id);
    runtime.queue.remove(0);
    super::session::start(app, item, true)
}

/// 公开函数，执行 update 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn update(app: &mut App, message: ChatMessage) -> Task<Message> {
    match message {
        ChatMessage::AgentStreamDelta(id, delta) => {
            stream_events::handle_agent_stream_delta(app, id, delta)
        }
        ChatMessage::AgentWorkflowNodeUpdate(id, raw_tool_block) => {
            stream_events::handle_agent_workflow_node_update(app, id, raw_tool_block)
        }
        ChatMessage::AgentStepStart(id, session_id, step_index, created_ms, model) => {
            steps::handle_agent_step_start(app, id, session_id, step_index, created_ms, model)
        }
        ChatMessage::AgentStepFinish(
            id,
            session_id,
            step_index,
            finished_ms,
            usage,
            finish_reason,
            model,
        ) => steps::handle_agent_step_finish(
            app,
            id,
            session_id,
            step_index,
            finished_ms,
            usage,
            finish_reason,
            model,
        ),
        ChatMessage::SessionSaveAck => Task::none(),
        ChatMessage::AgentStepCostLoaded(_id, session_id, step_index, resolved_model, cost) => {
            steps::handle_agent_step_cost_loaded(app, session_id, step_index, resolved_model, cost)
        }
        ChatMessage::AgentPostToolRound(id, session_id, step_index) => {
            stream_events::handle_agent_post_tool_round(app, id, session_id, step_index)
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
        ChatMessage::AgentStreamError(id, err) => {
            stream_events::handle_agent_stream_error(app, id, err)
        }
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
        ChatMessage::PermissionReplySubmitted(res) => {
            permissions::handle_permission_reply_submitted(app, res)
        }
        ChatMessage::SessionTitleGenerated(session_id, new_title) => {
            if new_title.is_empty() {
                return Task::none();
            }
            if let Some(s) = app.sessions.iter_mut().find(|s| s.id == session_id) {
                s.title = new_title.clone();
            }
            for sessions in app.project_sessions.values_mut() {
                if let Some(s) = sessions.iter_mut().find(|s| s.id == session_id) {
                    s.title = new_title.clone();
                }
            }
            app.refresh_task_pet_session_title(&session_id, &new_title);
            let session_id_clone = session_id.clone();
            Task::perform(
                async move {
                    if let Ok(client) = crate::app::gateway_client() {
                        let _ = client
                            .session_update::<vw_shared::session::info::Info>(
                                &session_id_clone,
                                None,
                                &vw_gateway_client::GatewaySessionPatchBody {
                                    title: Some(new_title),
                                    time: None,
                                },
                            )
                            .await;
                    }
                },
                |_| Message::None,
            )
        }
        _ => Task::none(),
    }
}

/// 模块内可见函数，执行 maybe_generate_session_title 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn maybe_generate_session_title(app: &App) -> Task<Message> {
    let Some(session_id) = app.active_session_id.clone() else {
        return Task::none();
    };

    let Some(current_session) = app.sessions.iter().find(|s| s.id == session_id) else {
        return Task::none();
    };

    let user_messages: Vec<&models::ChatMessage> =
        app.chat.iter().filter(|m| m.role == models::ChatRole::User).collect();

    if user_messages.len() != 1 {
        return Task::none();
    }

    let first_user_content = user_messages[0].content.clone();

    let title = current_session.title.trim();
    let first_user_trimmed = first_user_content.trim();
    let should_generate = is_default_title(title)
        || title.is_empty()
        || title == "新会话"
        || (!first_user_trimmed.is_empty() && title == first_user_trimmed);

    if !should_generate {
        return Task::none();
    }

    let preferred_model = app
        .current_session_runtime_ref()
        .map(|r| if r.auto_model { None } else { Some(r.model.clone()) })
        .unwrap_or_else(|| if app.auto_model { None } else { Some(app.model.clone()) });
    let title_acp_agent = app.current_session_runtime_ref().and_then(|runtime| {
        runtime
            .acp_agent
            .clone()
            .filter(|value| app.acp_agents.iter().any(|agent| agent == value))
            .or_else(|| {
                app.acp_agent
                    .clone()
                    .filter(|value| app.acp_agents.iter().any(|agent| agent == value))
            })
    });
    tracing::info!(
        target: "vw_desktop",
        session_id = %session_id,
        preferred_model = ?preferred_model,
        acp_test = title_acp_agent.is_some(),
        acp_agent = ?title_acp_agent,
        "starting session title generation task"
    );

    let session_id_for_result = session_id.clone();
    Task::perform(
        async move {
            let client = crate::app::config::gateway_client()?;
            client
                .session_title_generate(
                    &session_id,
                    &vw_gateway_client::GatewaySessionTitleGenerateBody {
                        content: first_user_content,
                        preferred_model,
                        acp_agent: title_acp_agent,
                    },
                )
                .await
                .map(|r| r.title)
        },
        move |result| match result {
            Ok(title) => {
                Message::Chat(ChatMessage::SessionTitleGenerated(session_id_for_result, title))
            }
            Err(_) => Message::None,
        },
    )
}
