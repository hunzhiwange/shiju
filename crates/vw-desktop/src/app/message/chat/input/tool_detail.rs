//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use super::ChatMessage;
use crate::app::components::chat_panel::tools::{
    format_mcp_detail_content, is_mcp_tool_raw, tool_error_text, tool_header_label,
    tool_output_text, tool_status,
};
use crate::app::components::text_editor_context_menu::{
    focus_editor_task, paste_action, paste_task, selection_copy_task, selection_cut_task,
    selection_delete_task,
};
use crate::app::state::ToolDetailDialog;
use crate::app::{App, Message};
use iced::{Task, mouse, widget::text_editor};

/// 模块内可见函数，执行 handle_open_tool_detail 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_open_tool_detail(
    app: &mut App,
    msg_idx: usize,
    tool_idx: usize,
    raw: String,
) -> Task<Message> {
    if let Some((title, content)) = tool_detail_from_raw(&raw) {
        let editor = text_editor::Content::with_text(&content);
        app.tool_detail_dialog = Some(ToolDetailDialog {
            msg_idx,
            tool_idx,
            title,
            content,
            editor,
            editor_id: iced::widget::Id::unique(),
            context_menu_open: false,
            context_menu_pos: None,
            scroll_top_line: 0.0,
            scroll_remainder: 0.0,
            viewport_height: 0.0,
        });
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_tool_detail_editor_action(
    app: &mut App,
    action: text_editor::Action,
) -> Task<Message> {
    if let text_editor::Action::Scroll { lines } = &action {
        apply_tool_detail_scroll_lines(app, *lines);
    }

    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        dialog.editor.perform(action);
    }

    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_open_context_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_tool_detail_open_context_menu(app: &mut App, x: f32, y: f32) -> Task<Message> {
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        dialog.context_menu_open = true;
        dialog.context_menu_pos = Some((x, y));
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_close_context_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_tool_detail_close_context_menu(app: &mut App) -> Task<Message> {
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        return focus_editor_task(&dialog.editor_id);
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_context_menu_copy 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_tool_detail_context_menu_copy(app: &mut App) -> Task<Message> {
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        let (_outcome, task) = selection_copy_task(&dialog.editor, &dialog.editor_id);
        return task;
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_context_menu_cut 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_tool_detail_context_menu_cut(app: &mut App) -> Task<Message> {
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        let (_outcome, task) = selection_cut_task(&mut dialog.editor, &dialog.editor_id);
        return task;
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_context_menu_paste 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_tool_detail_context_menu_paste(app: &mut App) -> Task<Message> {
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        return paste_task(&dialog.editor_id, |content| {
            Message::Chat(ChatMessage::ToolDetailEditorAction(paste_action(content)))
        });
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_context_menu_delete 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_tool_detail_context_menu_delete(app: &mut App) -> Task<Message> {
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        let (_outcome, task) = selection_delete_task(&mut dialog.editor, &dialog.editor_id);
        return task;
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_editor_wheel_scrolled 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_tool_detail_editor_wheel_scrolled(
    app: &mut App,
    delta: mouse::ScrollDelta,
    viewport_height: f32,
) -> Task<Message> {
    let line_height = app.current_line_height.max(1.0);
    let delta_lines = match delta {
        mouse::ScrollDelta::Lines { y, .. } => -y * 1.25,
        mouse::ScrollDelta::Pixels { y, .. } => -y / line_height,
    };

    let mut whole_lines = 0;

    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        dialog.viewport_height = viewport_height.max(0.0);
        dialog.scroll_remainder += delta_lines;
        whole_lines = if dialog.scroll_remainder >= 0.0 {
            dialog.scroll_remainder.floor() as i32
        } else {
            dialog.scroll_remainder.ceil() as i32
        };

        if whole_lines != 0 {
            dialog.scroll_remainder -= whole_lines as f32;
        }
    }

    if whole_lines != 0 {
        apply_tool_detail_scroll_lines(app, whole_lines);
        if let Some(dialog) = app.tool_detail_dialog.as_mut() {
            dialog.editor.perform(text_editor::Action::Scroll { lines: whole_lines });
        }
    }

    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_scrollbar_changed 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_tool_detail_scrollbar_changed(
    app: &mut App,
    top_line: f32,
    viewport_height: f32,
) -> Task<Message> {
    let max_scroll = tool_detail_max_scroll_top_line(app);
    let mut delta = 0;

    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        dialog.viewport_height = viewport_height.max(0.0);
        let target_top_line = top_line.round().clamp(0.0, max_scroll);
        let current_top_line = dialog.scroll_top_line.round();
        delta = (target_top_line - current_top_line) as i32;
    }

    if delta != 0 {
        apply_tool_detail_scroll_lines(app, delta);
        if let Some(dialog) = app.tool_detail_dialog.as_mut() {
            dialog.editor.perform(text_editor::Action::Scroll { lines: delta });
        }
    }

    Task::none()
}

/// 模块内可见函数，执行 handle_close_tool_detail 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_close_tool_detail(app: &mut App) -> Task<Message> {
    app.tool_detail_dialog = None;
    Task::none()
}

fn tool_detail_from_raw(raw: &str) -> Option<(String, String)> {
    let (first, rest) = raw.split_once('\n')?;
    let raw_tool_name = first.trim().strip_prefix("tool ")?.trim();
    let is_mcp = is_mcp_tool_raw(raw);
    let tool_name = if is_mcp { "MCP" } else { raw_tool_name };
    let fallback_title = tool_header_label(tool_name).to_string();
    let value = serde_json::from_str::<serde_json::Value>(rest.trim()).ok();
    let title = value
        .as_ref()
        .map(tool_status)
        .map(|status| match status {
            "error" | "denied" => format!("{fallback_title} 失败"),
            "running" => format!("{fallback_title} 运行中"),
            _ => fallback_title.clone(),
        })
        .unwrap_or(fallback_title);
    let content = value
        .as_ref()
        .and_then(tool_output_text)
        .or_else(|| value.as_ref().and_then(tool_error_text))
        .unwrap_or_else(|| rest.trim().to_string());
    let content = if is_mcp { format_mcp_detail_content(&content) } else { content };
    Some((title, content))
}

fn close_tool_detail_context_menu(dialog: &mut ToolDetailDialog) {
    dialog.context_menu_open = false;
    dialog.context_menu_pos = None;
}

fn tool_detail_max_scroll_top_line(app: &App) -> f32 {
    let Some(dialog) = app.tool_detail_dialog.as_ref() else {
        return 0.0;
    };

    let line_height = app.current_line_height.max(1.0);
    let viewport_height = dialog.viewport_height.max(0.0);
    let total_lines = dialog.editor.line_count().max(1) as f32;
    let visible_lines = (viewport_height / line_height).floor().max(1.0);

    (total_lines - visible_lines).max(0.0)
}

fn apply_tool_detail_scroll_lines(app: &mut App, delta: i32) {
    let max_scroll = tool_detail_max_scroll_top_line(app);
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        dialog.scroll_top_line = (dialog.scroll_top_line + delta as f32).clamp(0.0, max_scroll);
    }
}
#[cfg(test)]
#[path = "tool_detail_tests.rs"]
mod tool_detail_tests;
