//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use super::shared::{
    close_input_context_menu, focus_input_editor, sync_global_input_editor_if_needed,
};
use super::{ChatMessage, ClipboardPastePayload};
use crate::app::components::chat_panel::tool_text_support::selected_chat_text_for_target;
use crate::app::components::text_editor_context_menu::TextContextAction;
use crate::app::message::chat::input::clipboard::read_clipboard_for_input;
use crate::app::message::project::helpers::append_local_attachments;
use crate::app::{App, Message};
use iced::{Task, widget::text_editor};

/// 模块内可见函数，执行 handle_open_input_context_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_open_input_context_menu(app: &mut App, x: f32, y: f32) -> Task<Message> {
    app.input_context_menu.open(x, y);
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
    apply_input_menu(app, desktop_chat_input::input_panel::input_menu::Action::Copy)
}

/// 模块内可见函数，执行 handle_cut_input_selection 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_cut_input_selection(app: &mut App) -> Task<Message> {
    apply_input_menu(app, desktop_chat_input::input_panel::input_menu::Action::Cut)
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
        ClipboardPastePayload::LocalizedError { source, arguments } => {
            app.show_localized_toast(crate::app::state::ToastKind::Error, source, arguments)
        }
        ClipboardPastePayload::Error(error) => {
            app.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "粘贴失败：%s",
                vec![error.into()],
            )
        }
    }
}

/// 模块内可见函数，执行 handle_select_all_input 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_select_all_input(app: &mut App) -> Task<Message> {
    apply_input_menu(app, desktop_chat_input::input_panel::input_menu::Action::SelectAll)
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
    // Preserve target publication before selection resolution.
    app.chat_context_menu.target = Some(target);
    app.chat_context_menu.position = Some((x, y));
    let text = selected_chat_text_for_target(app, target).unwrap_or(text);
    app.chat_context_menu.open(target, x, y, text);
    Task::none()
}

/// 模块内可见函数，执行 handle_close_message_context_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_close_message_context_menu(app: &mut App) -> Task<Message> {
    app.chat_context_menu.close();
    Task::none()
}

/// 模块内可见函数，执行 handle_copy_context_menu_text 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_copy_context_menu_text(app: &mut App) -> Task<Message> {
    handle_text_context_action(app, TextContextAction::Copy)
}

/// 模块内可见函数，执行 handle_append_context_menu_text 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_append_context_menu_text(app: &mut App) -> Task<Message> {
    handle_text_context_action(app, TextContextAction::Append)
}

/// 模块内可见函数，执行 handle_search_context_menu_with_baidu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_search_context_menu_with_baidu(app: &mut App) -> Task<Message> {
    handle_text_context_action(app, TextContextAction::Baidu)
}

/// 模块内可见函数，执行 handle_search_context_menu_with_google 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_search_context_menu_with_google(app: &mut App) -> Task<Message> {
    handle_text_context_action(app, TextContextAction::Google)
}

/// 模块内可见函数，执行 handle_search_context_menu_with_bing 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_search_context_menu_with_bing(app: &mut App) -> Task<Message> {
    handle_text_context_action(app, TextContextAction::Bing)
}

fn handle_text_context_action(app: &mut App, action: TextContextAction) -> Task<Message> {
    action
        .message(app.chat_context_menu.take_text())
        .map(crate::app::components::text_editor_context_menu::text_context_message)
        .map_or_else(Task::none, Task::done)
}

pub(super) fn handle_open_fork_session_dialog(
    app: &mut App,
    anchor: Option<crate::app::message::chat::ForkSessionAnchor>,
) -> Task<Message> {
    if app.chat_fork_dialog.as_ref().is_some_and(|dialog| dialog.in_progress) {
        return Task::none();
    }
    app.chat_reset_menu_idx = None;
    app.chat_fork_dialog = anchor.filter(|anchor| crate::app::message::chat::fork_session_anchor_is_current(&anchor, app)).and_then(|anchor| {
        let index = app
            .chat_message_ids
            .iter()
            .position(|id| id.as_deref() == Some(anchor.user_message_id.as_str()))?;
        let prompt = crate::app::components::chat_panel::message_view::chat_message_action_text(
            app, index, false,
        )?;
        Some(crate::app::message::chat::ForkSessionDialog { anchor, prompt, in_progress: false })
    });
    if app.chat_fork_dialog.is_some() {
        Task::none()
    } else {
        app.show_localized_toast(
            crate::app::state::ToastKind::Warning,
            "无法确认此消息的轮次，请重新打开源会话后再试",
            vec![],
        )
    }
}

/// 模块内可见函数，执行 handle_close_fork_session_dialog 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_close_fork_session_dialog(app: &mut App) -> Task<Message> {
    desktop_session::message::chat::fork::close_fork_dialog(&mut app.chat_fork_dialog);
    Task::none()
}

/// 模块内可见函数，执行 handle_toggle_reset_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_toggle_reset_menu(app: &mut App, msg_idx: usize) -> Task<Message> {
    desktop_session::message::chat::fork::toggle_reset_menu(
        &mut app.chat_fork_dialog, &mut app.chat_reset_menu_idx, msg_idx,
    );
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

fn apply_input_menu(app: &mut App, action: desktop_chat_input::input_panel::input_menu::Action) -> Task<Message> {
    use desktop_chat_input::input_panel::input_menu::{self, Effect};
    let mut menu = std::mem::take(&mut app.input_context_menu);
    let outcome = input_menu::update(&mut menu, &mut app.current_session_runtime_mut().input_editor, action);
    app.input_context_menu = menu;
    if outcome.sync_editor { sync_global_input_editor_if_needed(app); }
    match outcome.effect {
        Effect::None => Task::none(),
        Effect::Copy(text) => Task::done(Message::CopyCode(text)),
        Effect::Focus => focus_input_editor(app),
    }
}
