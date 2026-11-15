//! 处理聊天流式会话事件。
//! 本模块把网关轮询和流式增量落到会话状态，避免 UI 层理解传输细节。

use super::ChatMessage;
use crate::app::message::after;
use crate::app::session_gateway;
use crate::app::{App, Message};
use iced::Task;
use std::time::Duration;

fn maybe_auto_collapse_todo_panel(app: &mut App) -> Task<Message> {
    if !app.chat_todo_expanded && app.chat_todo_anim <= 0.001 {
        return Task::none();
    }

    let Some(ref session) = app.active_session_id else {
        app.chat_todo_session_id = None;
        app.chat_todo_items.clear();
        return Task::none();
    };

    if app.chat_todo_session_id.as_deref() != Some(session.as_str()) {
        return Task::none();
    }

    let todos = &app.chat_todo_items;
    if todos.is_empty() {
        return Task::none();
    }
    if todos.iter().any(|t| t.status != "completed") {
        return Task::none();
    }

    app.chat_todo_expanded = false;
    if app.chat_todo_anim.abs() < 0.001 {
        app.chat_todo_anim = 0.0;
        Task::none()
    } else {
        after(Duration::from_millis(16), Message::Chat(ChatMessage::TodoAnimTick))
    }
}

fn apply_loaded_session_todos(
    app: &mut App,
    session_id: String,
    res: Result<Vec<vw_shared::todo::Todo>, String>,
) -> Task<Message> {
    if app.active_session_id.as_deref() != Some(session_id.as_str()) {
        return Task::none();
    }

    let session_changed = app.chat_todo_session_id.as_deref() != Some(session_id.as_str());

    match res {
        Ok(todos) => {
            app.chat_todo_session_id = Some(session_id);
            app.chat_todo_items = todos;
            if session_changed
                && !app.chat_todo_items.is_empty()
                && app.chat_todo_items.iter().any(|todo| todo.status != "completed")
            {
                app.chat_todo_expanded = true;
                app.chat_todo_anim = 1.0;
            }
        }
        Err(err) => {
            tracing::warn!(target: "vw_desktop", error = %err, session_id = %session_id, "failed to load todo list via gateway");
            app.chat_todo_session_id = Some(session_id);
            app.chat_todo_items.clear();
        }
    }

    maybe_auto_collapse_todo_panel(app)
}

fn todo_list_directory(app: &App) -> Option<String> {
    app.active_session_id
        .as_ref()
        .and_then(|id| app.sessions.iter().find(|session| &session.id == id))
        .map(|session| session.directory.clone())
        .filter(|directory| !directory.trim().is_empty())
        .or_else(|| app.project_path.clone())
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
                session_gateway::gateway_session_todo_list_async(&session_id, directory.as_deref())
                    .await;
            (session_id, result)
        },
        |(session_id, res)| Message::Chat(ChatMessage::InputPanelTodosLoaded(session_id, res)),
    )
}

/// 模块内可见函数，执行 handle_load_input_panel_todos 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_load_input_panel_todos(app: &mut App) -> Task<Message> {
    let Some(session_id) = app.active_session_id.clone() else {
        app.chat_todo_session_id = None;
        app.chat_todo_items.clear();
        return Task::none();
    };

    load_input_panel_todos_task(session_id, todo_list_directory(app))
}

/// 模块内可见函数，执行 handle_todo_poll_tick 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_todo_poll_tick(app: &mut App) -> Task<Message> {
    let Some(session_id) = app.active_session_id.clone() else {
        app.chat_todo_session_id = None;
        app.chat_todo_items.clear();
        return Task::none();
    };

    load_input_panel_todos_task(session_id, todo_list_directory(app))
}

/// 模块内可见函数，执行 handle_input_panel_todos_loaded 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_input_panel_todos_loaded(
    app: &mut App,
    session_id: String,
    res: Result<Vec<vw_shared::todo::Todo>, String>,
) -> Task<Message> {
    apply_loaded_session_todos(app, session_id, res)
}
#[cfg(test)]
#[path = "todos_tests.rs"]
mod todos_tests;
