//! 处理 Git 相关弹窗的打开、关闭和输入状态变化。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use super::GitMessage;
use super::shared::dismiss_preview_transient_ui;
use crate::app::{App, Message};
use iced::Task;

fn open_copy_modal_with_text(app: &mut App, text: String) {
    let theme = app.effective_editor_theme();
    desktop_git_ui::message_git::modal::open_copy_modal_with_text(
        &mut app.show_git_copy_modal, &mut app.git_copy_modal_use_color,
        &mut app.git_copy_modal_editor, &mut app.git_copy_modal_code_editor,
        text, theme, app.current_font_size, app.current_line_height,
        crate::app::i18n::editor::language(app.desktop_locale),
    );
}

/// update 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn update(app: &mut App, message: GitMessage) -> Task<Message> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        GitMessage::OpenCustomDiffModal => {
            desktop_git_ui::message_git::modal::open_custom_diff_modal(&mut app.show_git_custom_diff_modal, &mut app.git_custom_diff_hide_inputs, &mut app.git_custom_diff_title);
            Task::none()
        }
        GitMessage::CloseCustomDiffModal => {
            desktop_git_ui::message_git::modal::close_custom_diff_modal(&mut app.show_git_custom_diff_modal, &mut app.git_custom_diff_hide_inputs);
            Task::none()
        }
        GitMessage::CustomDiffTitleChanged(title) => {
            desktop_git_ui::message_git::modal::custom_diff_title_changed(&mut app.git_custom_diff_title, title);
            Task::none()
        }
        GitMessage::CustomDiffBeforeEditorAction(action) => {
            desktop_git_ui::message_git::modal::custom_diff_before_editor_action(&mut app.git_custom_diff_before_editor, action);
            Task::none()
        }
        GitMessage::CustomDiffAfterEditorAction(action) => {
            desktop_git_ui::message_git::modal::custom_diff_after_editor_action(&mut app.git_custom_diff_after_editor, action);
            Task::none()
        }
        GitMessage::CustomDiffSwap => {
            desktop_git_ui::message_git::modal::custom_diff_swap(&mut app.git_custom_diff_before_editor, &mut app.git_custom_diff_after_editor);
            Task::none()
        }
        GitMessage::OpenCustomDiffResult { title, before, after } => {
            dismiss_preview_transient_ui(app);
            desktop_git_ui::message_git::modal::open_custom_diff_result(
                &mut app.show_git_custom_diff_modal, &mut app.git_custom_diff_hide_inputs,
                &mut app.git_custom_diff_title, &mut app.git_custom_diff_before_editor,
                &mut app.git_custom_diff_after_editor, title, before, after,
            );
            app.active_preview_path = None;
            app.show_diff = true;
            Task::none()
        }
        GitMessage::OpenChatTextDiff { title, file: _, before, after } => {
            // 与 OpenCustomDiffResult 共用自定义 Diff 弹窗，避免在面板中再插一块独立消息框。
            dismiss_preview_transient_ui(app);
            app.chat_text_diff = None;
            desktop_git_ui::message_git::modal::open_custom_diff_result(
                &mut app.show_git_custom_diff_modal, &mut app.git_custom_diff_hide_inputs,
                &mut app.git_custom_diff_title, &mut app.git_custom_diff_before_editor,
                &mut app.git_custom_diff_after_editor, title, before, after,
            );
            app.active_preview_path = None;
            app.show_diff = true;
            app.file_manager.file_manager_show_changes = true;
            Task::none()
        }
        GitMessage::CloseChatTextDiff => {
            desktop_git_ui::message_git::modal::close_chat_text_diff(&mut app.chat_text_diff, &mut app.show_git_custom_diff_modal, &mut app.git_custom_diff_hide_inputs);
            Task::none()
        }
        GitMessage::OpenDiffCopyMode(file) => {
            let patch = desktop_git_ui::message_git::modal::diff_patch(app.project_path.as_deref(), file);
            open_copy_modal_with_text(app, patch);
            Task::none()
        }
        GitMessage::OpenCopyModalWithText(text) => {
            open_copy_modal_with_text(app, text);
            Task::none()
        }
        GitMessage::OpenCopyModalFromPath(path) => {
            let content = std::fs::read_to_string(path).unwrap_or_default();
            open_copy_modal_with_text(app, content);
            Task::none()
        }
        GitMessage::CloseCopyModal => {
            desktop_git_ui::message_git::modal::close_copy_modal(&mut app.show_git_copy_modal);
            Task::none()
        }
        GitMessage::ToggleCopyModalColored(enabled) => {
            let theme = app.effective_editor_theme();
            desktop_git_ui::message_git::modal::toggle_copy_modal_colored(
                &mut app.git_copy_modal_use_color, &mut app.git_copy_modal_editor,
                &mut app.git_copy_modal_code_editor, enabled, theme,
                app.current_font_size, app.current_line_height,
                crate::app::i18n::editor::language(app.desktop_locale),
            );
            Task::none()
        }
        GitMessage::CopyModalEditorAction(action) => {
            desktop_git_ui::message_git::modal::copy_modal_editor_action(&mut app.git_copy_modal_editor, action);
            Task::none()
        }
        GitMessage::CopyModalCodeEditorEvent(event) => app
            .git_copy_modal_code_editor
            .update(&event)
            .map(|next| Message::Git(GitMessage::CopyModalCodeEditorEvent(next))),
        GitMessage::CopyModalCopyCurrent => {
            let content = desktop_git_ui::message_git::modal::current_copy_content(app.git_copy_modal_use_color, &app.git_copy_modal_editor, &app.git_copy_modal_code_editor);
            Task::done(Message::CopyCode(content))
        }
        GitMessage::InsertCopyModalToChatCurrent => {
            let content = desktop_git_ui::message_git::modal::current_copy_content(app.git_copy_modal_use_color, &app.git_copy_modal_editor, &app.git_copy_modal_code_editor);
            app.show_git_copy_modal = false;
            Task::done(Message::Chat(crate::app::message::ChatMessage::AppendText(content)))
        }
        GitMessage::InsertCopyModalToChat(text) => {
            app.show_git_copy_modal = false;
            Task::done(Message::Chat(crate::app::message::ChatMessage::AppendText(text)))
        }
        _ => unreachable!("unexpected modal git message"),
    }
}
