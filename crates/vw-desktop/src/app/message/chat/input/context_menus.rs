//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use super::shared::{
    close_input_context_menu, focus_input_editor, sync_global_input_editor_if_needed,
};
use super::{ChatMessage, ClipboardPastePayload};
use crate::app::components::chat_panel::tool_text_support::selected_chat_text_for_target;
use crate::app::message::chat::input::clipboard::read_clipboard_for_input;
use crate::app::message::project::helpers::append_local_attachments;
use crate::app::{App, Message};
use iced::{Task, widget::text_editor};

/// 模块内可见函数，执行 handle_open_input_context_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_open_input_context_menu(app: &mut App, x: f32, y: f32) -> Task<Message> {
    app.input_context_menu_open = true;
    app.input_context_menu_pos = Some((x, y));
    Task::none()
}

/// 模块内可见函数，执行 handle_close_input_context_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_close_input_context_menu(app: &mut App) -> Task<Message> {
    close_input_context_menu(app);
    Task::none()
}

/// 模块内可见函数，执行 handle_copy_input_selection 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_copy_input_selection(app: &mut App) -> Task<Message> {
    let selected = app.current_session_runtime_mut().input_editor.selection().unwrap_or_default();
    close_input_context_menu(app);
    if selected.is_empty() { Task::none() } else { Task::done(Message::CopyCode(selected)) }
}

/// 模块内可见函数，执行 handle_cut_input_selection 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_cut_input_selection(app: &mut App) -> Task<Message> {
    let selected = app.current_session_runtime_mut().input_editor.selection().unwrap_or_default();
    close_input_context_menu(app);

    if selected.is_empty() {
        return Task::none();
    }

    {
        let runtime = app.current_session_runtime_mut();
        runtime.input_editor.perform(text_editor::Action::Edit(text_editor::Edit::Backspace));
    }

    sync_global_input_editor_if_needed(app);
    Task::done(Message::CopyCode(selected))
}

/// 模块内可见函数，执行 handle_paste_into_input 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_paste_into_input(app: &mut App) -> Task<Message> {
    close_input_context_menu(app);
    read_clipboard_for_input()
}

/// 模块内可见函数，执行 handle_clipboard_paste_resolved 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_clipboard_paste_resolved(
    app: &mut App,
    payload: ClipboardPastePayload,
) -> Task<Message> {
    match payload {
        ClipboardPastePayload::Text(content) => {
            Task::done(Message::Chat(ChatMessage::InputEditorAction(text_editor::Action::Edit(
                text_editor::Edit::Paste(std::sync::Arc::new(content)),
            ))))
        }
        ClipboardPastePayload::AttachmentPath(path) => {
            append_local_attachments(app, vec![path]);
            focus_input_editor(app)
        }
        ClipboardPastePayload::Empty => focus_input_editor(app),
        ClipboardPastePayload::Error(error) => {
            app.push_notification(error);
            focus_input_editor(app)
        }
    }
}

/// 模块内可见函数，执行 handle_select_all_input 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_select_all_input(app: &mut App) -> Task<Message> {
    close_input_context_menu(app);
    {
        let runtime = app.current_session_runtime_mut();
        runtime.input_editor.perform(text_editor::Action::SelectAll);
    }
    sync_global_input_editor_if_needed(app);
    focus_input_editor(app)
}

/// 模块内可见函数，执行 handle_open_message_context_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_open_message_context_menu(
    app: &mut App,
    target: u64,
    x: f32,
    y: f32,
    text: String,
) -> Task<Message> {
    app.chat_context_menu_target = Some(target);
    app.chat_context_menu_pos = Some((x, y));
    app.chat_context_menu_text = selected_chat_text_for_target(app, target).unwrap_or(text);
    Task::none()
}

/// 模块内可见函数，执行 handle_close_message_context_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_close_message_context_menu(app: &mut App) -> Task<Message> {
    app.chat_context_menu_target = None;
    app.chat_context_menu_pos = None;
    app.chat_context_menu_text.clear();
    Task::none()
}

/// 模块内可见函数，执行 handle_copy_context_menu_text 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_copy_context_menu_text(app: &mut App) -> Task<Message> {
    let text = app.chat_context_menu_text.clone();
    clear_message_context_menu(app);
    if text.is_empty() { Task::none() } else { Task::done(Message::CopyCode(text)) }
}

/// 模块内可见函数，执行 handle_append_context_menu_text 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_append_context_menu_text(app: &mut App) -> Task<Message> {
    let text = app.chat_context_menu_text.clone();
    clear_message_context_menu(app);
    if text.is_empty() {
        Task::none()
    } else {
        Task::done(Message::Chat(ChatMessage::AppendText(text)))
    }
}

/// 模块内可见函数，执行 handle_search_context_menu_with_baidu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_search_context_menu_with_baidu(app: &mut App) -> Task<Message> {
    handle_search_context_menu(app, "https://www.baidu.com/s?wd=")
}

/// 模块内可见函数，执行 handle_search_context_menu_with_google 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_search_context_menu_with_google(app: &mut App) -> Task<Message> {
    handle_search_context_menu(app, "https://www.google.com/search?q=")
}

/// 模块内可见函数，执行 handle_search_context_menu_with_bing 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_search_context_menu_with_bing(app: &mut App) -> Task<Message> {
    handle_search_context_menu(app, "https://www.bing.com/search?q=")
}

fn clear_message_context_menu(app: &mut App) {
    app.chat_context_menu_target = None;
    app.chat_context_menu_pos = None;
    app.chat_context_menu_text.clear();
}

fn handle_search_context_menu(app: &mut App, base_url: &str) -> Task<Message> {
    let query = app.chat_context_menu_text.trim().to_string();
    clear_message_context_menu(app);
    if query.is_empty() {
        Task::none()
    } else {
        Task::done(Message::View(crate::app::message::ViewMessage::OpenUrlExternal(format!(
            "{base_url}{}",
            urlencoding::encode(&query)
        ))))
    }
}

/// 模块内可见函数，执行 handle_open_fork_session_dialog 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_open_fork_session_dialog(app: &mut App, msg_idx: usize) -> Task<Message> {
    app.chat_fork_dialog_idx = Some(msg_idx);
    app.chat_reset_menu_idx = None;
    Task::none()
}

/// 模块内可见函数，执行 handle_close_fork_session_dialog 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_close_fork_session_dialog(app: &mut App) -> Task<Message> {
    app.chat_fork_dialog_idx = None;
    Task::none()
}

/// 模块内可见函数，执行 handle_toggle_reset_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_toggle_reset_menu(app: &mut App, msg_idx: usize) -> Task<Message> {
    app.chat_fork_dialog_idx = None;
    app.chat_reset_menu_idx =
        if app.chat_reset_menu_idx == Some(msg_idx) { None } else { Some(msg_idx) };
    Task::none()
}

/// 模块内可见函数，执行 handle_close_reset_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_close_reset_menu(app: &mut App) -> Task<Message> {
    app.chat_reset_menu_idx = None;
    Task::none()
}
#[cfg(test)]
#[path = "context_menus_tests.rs"]
mod context_menus_tests;
