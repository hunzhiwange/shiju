//! 处理聊天会话相关应用消息。
//! 本模块协调用户输入、会话缓存和网关持久化。

use super::ChatMessage;
use crate::app::ui::chat;
use crate::app::{App, Message};
use iced::Task;
use desktop_session::message::chat::context::{ContextInput, context_input_text};

/// 公开函数，执行 update 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn update(app: &mut App, message: ChatMessage) -> Task<Message> {
    match message {
        ChatMessage::InsertPosition(path, line, col) => {
            let text = context_input_text(app.navigation_tabs.project_path.as_deref(), ContextInput::Position {
                path: &path, line, column: col,
            }).expect("position always produces context text");
            let runtime = app.current_session_runtime_mut();
            chat::append_line(&mut runtime.input_editor, &text);
            if app.session_state.active_session_id.is_none() {
                let runtime = app.current_session_runtime();
                app.session_state.input_editor = runtime.input_editor;
            }
            app.preview_state.show_preview_context_menu = false;
            Task::none()
        }
        ChatMessage::InsertActiveMatch => Task::none(),
        ChatMessage::InsertSelectionRange => Task::none(),
        ChatMessage::InsertSelectionPositions => {
            if let Some((path, start_line, start_col, end_line, end_col)) =
                app.preview_state.preview_context_target.clone()
            {
                let s = context_input_text(app.navigation_tabs.project_path.as_deref(), ContextInput::Selection {
                    path: &path, start_line, start_column: start_col, end_line, end_column: end_col,
                }).expect("selection always produces context text");
                let runtime = app.current_session_runtime_mut();
                chat::append_line(&mut runtime.input_editor, &s);
                if app.session_state.active_session_id.is_none() {
                    let runtime = app.current_session_runtime();
                    app.session_state.input_editor = runtime.input_editor;
                }
            }
            app.preview_state.show_preview_context_menu = false;
            Task::none()
        }
        ChatMessage::InsertSelected => {
            if let Some(path) = app.navigation_tabs.active_preview_path.clone()
                && let Some(tab) = app.navigation_tabs.preview_tabs.iter_mut().find(|t| t.path == path)
            {
                let copy_task =
                    tab.editor.inner.update(&iced_code_editor::Message::Copy).map(|e| {
                        Message::Preview(crate::app::message::preview::PreviewMessage::EditorEvent(
                            e,
                        ))
                    });

                app.preview_state.show_preview_context_menu = false;

                return copy_task
                    .chain(iced::clipboard::read().map(|opt| {
                        Message::Chat(ChatMessage::AppendText(opt.unwrap_or_default()))
                    }));
            }
            app.preview_state.show_preview_context_menu = false;
            Task::none()
        }
        ChatMessage::AppendText(text) => {
            if let Some(text) = context_input_text(None, ContextInput::Append(&text)) {
                let runtime = app.current_session_runtime_mut();
                chat::append_line(&mut runtime.input_editor, &text);
                if app.session_state.active_session_id.is_none() {
                    let runtime = app.current_session_runtime();
                    app.session_state.input_editor = runtime.input_editor;
                }
            }
            Task::none()
        }
        _ => Task::none(),
    }
}
#[cfg(test)]
#[path = "context_tests.rs"]
mod context_tests;
