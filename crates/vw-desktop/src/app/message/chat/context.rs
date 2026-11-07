//! 处理聊天会话相关应用消息。
//! 本模块协调用户输入、会话缓存和网关持久化。

use super::ChatMessage;
use crate::app::ui::chat;
use crate::app::{App, Message};
use iced::Task;

fn mention_display_path(app: &App, path: &str) -> String {
    if let Some(project_root) = &app.project_path {
        std::path::Path::new(path)
            .strip_prefix(project_root)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or(path)
            .replace('\\', "/")
    } else {
        path.replace('\\', "/")
    }
}

/// 公开函数，执行 update 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn update(app: &mut App, message: ChatMessage) -> Task<Message> {
    match message {
        ChatMessage::InsertPosition(path, line, col) => {
            let display_path = mention_display_path(app, &path);
            let runtime = app.current_session_runtime_mut();
            chat::append_line(
                &mut runtime.input_editor,
                &chat::format_position(&display_path, line, col),
            );
            if app.active_session_id.is_none() {
                let runtime = app.current_session_runtime();
                app.input_editor = runtime.input_editor;
            }
            app.show_preview_context_menu = false;
            Task::none()
        }
        ChatMessage::InsertActiveMatch => Task::none(),
        ChatMessage::InsertSelectionRange => Task::none(),
        ChatMessage::InsertSelectionPositions => {
            if let Some((path, start_line, start_col, end_line, end_col)) =
                app.preview_context_target.clone()
            {
                let display_path = mention_display_path(app, &path);
                let s = chat::format_selection_positions(
                    &display_path,
                    start_line,
                    start_col,
                    end_line,
                    end_col,
                );
                let runtime = app.current_session_runtime_mut();
                chat::append_line(&mut runtime.input_editor, &s);
                if app.active_session_id.is_none() {
                    let runtime = app.current_session_runtime();
                    app.input_editor = runtime.input_editor;
                }
            }
            app.show_preview_context_menu = false;
            Task::none()
        }
        ChatMessage::InsertSelected => {
            if let Some(path) = app.active_preview_path.clone()
                && let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path == path)
            {
                let copy_task =
                    tab.editor.inner.update(&iced_code_editor::Message::Copy).map(|e| {
                        Message::Preview(crate::app::message::preview::PreviewMessage::EditorEvent(
                            e,
                        ))
                    });

                app.show_preview_context_menu = false;

                return copy_task
                    .chain(iced::clipboard::read().map(|opt| {
                        Message::Chat(ChatMessage::AppendText(opt.unwrap_or_default()))
                    }));
            }
            app.show_preview_context_menu = false;
            Task::none()
        }
        ChatMessage::AppendText(text) => {
            if !text.is_empty() {
                let runtime = app.current_session_runtime_mut();
                chat::append_line(&mut runtime.input_editor, &text);
                if app.active_session_id.is_none() {
                    let runtime = app.current_session_runtime();
                    app.input_editor = runtime.input_editor;
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
