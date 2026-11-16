//! 处理聊天流式会话事件。
//! 本模块把网关轮询和流式增量落到会话状态，避免 UI 层理解传输细节。

use super::ChatMessage;
use crate::app::message::after;
use desktop_session::session_gateway;
use crate::app::{App, Message};
use iced::Task;
use std::time::Duration;

fn panel(app: &mut App) -> desktop_session::message::chat::stream::todos::TodoPanel<'_> {
    desktop_session::message::chat::stream::todos::TodoPanel {
        active_session_id: app.session_state.active_session_id.as_deref(),
        chat_todo_session_id: &mut app.todo_state.chat_todo_session_id,
        chat_todo_items: &mut app.todo_state.chat_todo_items,
        chat_todo_expanded: &mut app.todo_state.chat_todo_expanded,
        chat_todo_anim: &mut app.todo_state.chat_todo_anim,
    }
}
fn animation_task(tick: bool) -> Task<Message> {
    if tick { after(Duration::from_millis(16), Message::Chat(ChatMessage::TodoAnimTick)) }
    else { Task::none() }
}
fn apply_loaded_session_todos(app: &mut App, session_id: String, res: Result<Vec<todo_types::Todo>, String>) -> Task<Message> {
    animation_task(desktop_session::message::chat::stream::todos::apply_loaded_session_todos(&mut panel(app), session_id, res))
}

fn todo_list_directory(app: &App) -> Option<String> {
    app.session_state.active_session_id
        .as_ref()
        .and_then(|id| app.session_state.sessions.iter().find(|session| &session.id == id))
        .map(|session| session.directory.clone())
        .filter(|directory| !directory.trim().is_empty())
        .or_else(|| app.navigation_tabs.project_path.clone())
}

/// 模块内可见函数，执行 load_input_panel_todos_task 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn load_input_panel_todos_task(
    session_id: String,
    directory: Option<String>,
) -> Task<Message> {
    Task::perform(
        async move {
            let result =
                session_gateway::gateway_session_todo_list_async(crate::app::config::gateway_client_endpoint(), &session_id, directory.as_deref())
                    .await;
            (session_id, result)
        },
        |(session_id, res)| Message::Chat(ChatMessage::InputPanelTodosLoaded(session_id, res)),
    )
}

/// 模块内可见函数，执行 handle_load_input_panel_todos 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_load_input_panel_todos(app: &mut App) -> Task<Message> {
    let Some(session_id) = app.session_state.active_session_id.clone() else {
        app.todo_state.chat_todo_session_id = None;
        app.todo_state.chat_todo_items.clear();
        return Task::none();
    };

    load_input_panel_todos_task(session_id, todo_list_directory(app))
}

/// 模块内可见函数，执行 handle_todo_poll_tick 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_todo_poll_tick(app: &mut App) -> Task<Message> {
    let Some(session_id) = app.session_state.active_session_id.clone() else {
        app.todo_state.chat_todo_session_id = None;
        app.todo_state.chat_todo_items.clear();
        return Task::none();
    };

    load_input_panel_todos_task(session_id, todo_list_directory(app))
}

/// 模块内可见函数，执行 handle_input_panel_todos_loaded 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_input_panel_todos_loaded(
    app: &mut App,
    session_id: String,
    res: Result<Vec<todo_types::Todo>, String>,
) -> Task<Message> {
    apply_loaded_session_todos(app, session_id, res)
}
#[cfg(test)]
#[path = "todos_tests.rs"]
mod todos_tests;
