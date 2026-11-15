//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。



use desktop_ui_controls::text_editor_context_menu::{
    focus_editor_task, paste_action, paste_task, selection_copy_task, selection_cut_task,
    selection_delete_task,
};

use iced::widget::Id;
use iced::{Task, mouse, widget::text_editor};

pub const TOOL_DETAIL_HISTORY_LIMIT: usize = 100;

/// 模块内可见函数，执行 handle_open_tool_detail 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn handle_open_tool_detail(
    app: &mut ToolDetailState,
    msg_idx: usize,
    tool_idx: usize,
    detail: Option<(String, String)>,
) -> Task<ToolDetailMessage> {
    if let Some((title, content)) =
        detail
    {
        let editor = text_editor::Content::with_text(&content);
        app.tool_detail_dialog = Some(ToolDetailDialog {
            msg_idx,
            tool_idx,
            title,
            content,
            editor,
            editor_id: iced::widget::Id::unique(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
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
pub fn handle_tool_detail_editor_action(
    app: &mut ToolDetailState,
    action: text_editor::Action,
) -> Task<ToolDetailMessage> {
    if let text_editor::Action::Scroll { lines } = &action {
        apply_tool_detail_scroll_lines(app, *lines);
    }

    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        let previous = action.is_edit().then(|| dialog.editor.text());
        dialog.editor.perform(action);
        if let Some(previous) = previous {
            record_tool_detail_edit(dialog, previous);
        }
    }

    Task::none()
}

/// 撤销当前工具详情弹窗中的一次文本编辑。
pub fn handle_tool_detail_undo(app: &mut ToolDetailState) -> Task<ToolDetailMessage> {
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        if let Some(previous) = dialog.undo_stack.pop() {
            dialog.redo_stack.push(dialog.editor.text());
            restore_tool_detail_text(dialog, &previous);
        }
        return focus_editor_task(&dialog.editor_id);
    }
    Task::none()
}

/// 重做当前工具详情弹窗中最近撤销的文本编辑。
pub fn handle_tool_detail_redo(app: &mut ToolDetailState) -> Task<ToolDetailMessage> {
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        if let Some(next) = dialog.redo_stack.pop() {
            dialog.undo_stack.push(dialog.editor.text());
            restore_tool_detail_text(dialog, &next);
        }
        return focus_editor_task(&dialog.editor_id);
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_open_context_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn handle_tool_detail_open_context_menu(app: &mut ToolDetailState, x: f32, y: f32) -> Task<ToolDetailMessage> {
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        dialog.context_menu_open = true;
        dialog.context_menu_pos = Some((x, y));
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_close_context_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn handle_tool_detail_close_context_menu(app: &mut ToolDetailState) -> Task<ToolDetailMessage> {
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        return focus_editor_task(&dialog.editor_id);
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_context_menu_copy 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn handle_tool_detail_context_menu_copy(app: &mut ToolDetailState) -> Task<ToolDetailMessage> {
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        let (_outcome, task) = selection_copy_task(&dialog.editor, &dialog.editor_id);
        return task;
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_context_menu_cut 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn handle_tool_detail_context_menu_cut(app: &mut ToolDetailState) -> Task<ToolDetailMessage> {
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        let previous = dialog.editor.text();
        let (_outcome, task) = selection_cut_task(&mut dialog.editor, &dialog.editor_id);
        record_tool_detail_edit(dialog, previous);
        return task;
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_context_menu_paste 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn handle_tool_detail_context_menu_paste(app: &mut ToolDetailState) -> Task<ToolDetailMessage> {
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        return paste_task(&dialog.editor_id, |content| {
            ToolDetailMessage::EditorAction(paste_action(content))
        });
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_context_menu_delete 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn handle_tool_detail_context_menu_delete(app: &mut ToolDetailState) -> Task<ToolDetailMessage> {
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        close_tool_detail_context_menu(dialog);
        let previous = dialog.editor.text();
        let (_outcome, task) = selection_delete_task(&mut dialog.editor, &dialog.editor_id);
        record_tool_detail_edit(dialog, previous);
        return task;
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_tool_detail_editor_wheel_scrolled 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn handle_tool_detail_editor_wheel_scrolled(
    app: &mut ToolDetailState,
    delta: mouse::ScrollDelta,
    viewport_height: f32,
) -> Task<ToolDetailMessage> {
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
pub fn handle_tool_detail_scrollbar_changed(
    app: &mut ToolDetailState,
    top_line: f32,
    viewport_height: f32,
) -> Task<ToolDetailMessage> {
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
pub fn handle_close_tool_detail(app: &mut ToolDetailState) -> Task<ToolDetailMessage> {
    app.tool_detail_dialog = None;
    Task::none()
}



fn record_tool_detail_edit(dialog: &mut ToolDetailDialog, previous: String) {
    if dialog.editor.text() == previous {
        return;
    }
    if dialog.undo_stack.len() >= TOOL_DETAIL_HISTORY_LIMIT {
        dialog.undo_stack.remove(0);
    }
    dialog.undo_stack.push(previous);
    dialog.redo_stack.clear();
}

fn restore_tool_detail_text(dialog: &mut ToolDetailDialog, content: &str) {
    dialog.editor = text_editor::Content::with_text(content);
    // 重建 Content 会重置编辑器内部视口，同步外部滚动条。
    dialog.scroll_top_line = 0.0;
    dialog.scroll_remainder = 0.0;
}

fn close_tool_detail_context_menu(dialog: &mut ToolDetailDialog) {
    dialog.context_menu_open = false;
    dialog.context_menu_pos = None;
}

fn tool_detail_max_scroll_top_line(app: &ToolDetailState) -> f32 {
    let Some(dialog) = app.tool_detail_dialog.as_ref() else {
        return 0.0;
    };

    let line_height = app.current_line_height.max(1.0);
    let viewport_height = dialog.viewport_height.max(0.0);
    let total_lines = dialog.editor.line_count().max(1) as f32;
    let visible_lines = (viewport_height / line_height).floor().max(1.0);

    (total_lines - visible_lines).max(0.0)
}

fn apply_tool_detail_scroll_lines(app: &mut ToolDetailState, delta: i32) {
    let max_scroll = tool_detail_max_scroll_top_line(app);
    if let Some(dialog) = app.tool_detail_dialog.as_mut() {
        dialog.scroll_top_line = (dialog.scroll_top_line + delta as f32).clamp(0.0, max_scroll);
    }
}


#[derive(Debug, Clone)]
pub struct ToolDetailDialog {
    pub msg_idx: usize,
    pub tool_idx: usize,
    pub title: String,
    pub content: String,
    pub editor: text_editor::Content,
    pub editor_id: Id,
    pub undo_stack: Vec<String>,
    pub redo_stack: Vec<String>,
    pub context_menu_open: bool,
    pub context_menu_pos: Option<(f32, f32)>,
    pub scroll_top_line: f32,
    pub scroll_remainder: f32,
    pub viewport_height: f32,
}

#[derive(Debug)]
pub enum ToolDetailMessage { EditorAction(text_editor::Action) }
pub struct ToolDetailState {
 pub tool_detail_dialog: Option<ToolDetailDialog>,
 pub current_line_height: f32,
}

use desktop_chat_tools::tools::tool_meta::tool_header_label;
use desktop_chat_tools::tools::tool_parse::{tool_error_text, tool_output_text, tool_status};
pub fn tool_detail_from_raw(i18n: desktop_i18n::I18n, raw: &str, is_mcp: bool, workflow_detail: impl FnOnce(desktop_i18n::I18n, &serde_json::Value) -> String) -> Option<(String, String)> {
    let (first, rest) = raw.split_once('\n')?;
    let raw_tool_name = first.trim().strip_prefix("tool ")?.trim();
    let tool_name = if is_mcp {
        "MCP"
    } else {
        desktop_chat_tools::tool_names::canonical_tool_name(raw_tool_name)
    };
    let fallback_title = i18n.__(&tool_header_label(tool_name)).to_string();
    let value = serde_json::from_str::<serde_json::Value>(rest.trim()).ok();
    let title = if tool_name == "workflow_node" {
        value
            .as_ref()
            .and_then(|payload| payload.get("metadata"))
            .and_then(|metadata| metadata.get("title"))
            .and_then(|title| title.as_str())
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(|title| {
                let status = value.as_ref().map(tool_status).unwrap_or("");
                match status {
                    "error" | "denied" | "failed" => i18n
                        .format("%s · 失败", &[title.into()])
                        .expect("valid tool detail title template"),
                    "running" => i18n
                        .format("%s · 运行中", &[title.into()])
                        .expect("valid tool detail title template"),
                    _ => title.to_string(),
                }
            })
            .unwrap_or_else(|| {
                value
                    .as_ref()
                    .map(tool_status)
                    .map(|status| match status {
                        "error" | "denied" | "failed" => i18n
                            .format("%s 失败", &[fallback_title.as_str().into()])
                            .expect("valid tool detail title template"),
                        "running" => i18n
                            .format("%s 运行中", &[fallback_title.as_str().into()])
                            .expect("valid tool detail title template"),
                        _ => fallback_title.clone(),
                    })
                    .unwrap_or(fallback_title)
            })
    } else {
        value
            .as_ref()
            .map(tool_status)
            .map(|status| match status {
                "error" | "denied" => i18n
                    .format("%s 失败", &[fallback_title.as_str().into()])
                    .expect("valid tool detail title template"),
                "running" => i18n
                    .format("%s 运行中", &[fallback_title.as_str().into()])
                    .expect("valid tool detail title template"),
                _ => fallback_title.clone(),
            })
            .unwrap_or(fallback_title)
    };
    let content = if tool_name == "workflow_node" {
        value
            .as_ref()
            .map(|value| workflow_detail(i18n, value))
            .filter(|content| !content.trim().is_empty())
            .unwrap_or_else(|| rest.trim().to_string())
    } else {
        let content = value
            .as_ref()
            .and_then(tool_output_text)
            .or_else(|| value.as_ref().and_then(tool_error_text))
            .unwrap_or_else(|| rest.trim().to_string());
        let trimmed = content.trim();
        if is_mcp || trimmed.starts_with(['{', '[']) {
            serde_json::from_str::<serde_json::Value>(trimmed)
                .ok()
                .and_then(|value| serde_json::to_string_pretty(&value).ok())
                .unwrap_or_else(|| if is_mcp { content.trim().to_string() } else { content })
        } else {
            content
        }
    };
    Some((title, content))
}

#[cfg(test)]
#[path = "tool_detail_tests.rs"]
mod tool_detail_tests;
