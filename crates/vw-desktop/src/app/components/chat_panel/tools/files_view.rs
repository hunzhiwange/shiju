//! 文件视图组件模块
//!
//! 本模块提供聊天面板中工具执行结果的文件列表视图渲染功能。
//! 主要用于展示各类文件操作工具（读取、写入、编辑、搜索等）的执行结果，
//! 并提供文件预览、变更查看等交互功能。
//!
//! # 主要功能
//!
//! - 解析工具执行的输出内容，提取文件路径信息
//! - 渲染文件列表视图，支持文件过滤和截断显示
//! - 为编辑类工具提供变更差异预览入口
//! - 支持搜索类工具（glob/grep/lsp）的结果展示
//! - 历史消息中的旧搜索别名（如 codesearch、glob_search、content_search）仍可渲染
//!
//! # 视图类型
//!
//! - **编辑类视图**：针对 write/apply_patch 工具，显示文件变更统计和差异预览按钮
//! - **搜索类视图**：针对 glob/grep/lsp 及其历史别名，提供文件过滤输入框
//! - **读取类视图**：针对 read 工具，显示读取的行范围信息

mod common_view;
mod list_ui;
mod parse;
mod write_view;

#[cfg(test)]
mod common_view_tests;
#[cfg(test)]
mod list_ui_tests;
#[cfg(test)]
mod parse_tests;
#[cfg(test)]
mod write_view_tests;

use iced::widget::{Space, button, column, container, mouse_area};
use iced::{Element, Length, Theme};

use crate::app::assets::Icon;
use crate::app::components::chat_panel::utils::{
    eye_icon_button_style, eye_icon_svg_style, icon_svg, simplified_block_style,
};
use crate::app::{App, Message, message};

use super::{canonical_tool_name, tool_permission_state, tool_permission_target_summary};
use super::tool_meta::{tool_activity_header, tool_verb, tool_visual_state};
use super::tool_parse::{tool_error_text, tool_input, tool_output_text, tool_status};

struct FilesViewContext<'a> {
    app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    visible: String,
    tool_name: String,
    error_text: Option<String>,
    input: String,
    output: String,
    verb: &'static str,
    is_error: bool,
    is_running: bool,
    is_edit_like: bool,
    read_range: Option<String>,
}

struct FileListState {
    items_for_display: Vec<(String, String)>,
    total_items: usize,
    display_count: usize,
    truncated_middle: bool,
    middle_omitted: usize,
    tail_omitted: usize,
    filter_query: String,
    is_empty_filtered: bool,
    max_items: usize,
    is_search: bool,
}

/// 构建工具文件列表视图
///
/// 根据工具执行的输出内容，解析并渲染相应的文件列表视图。
/// 支持多种工具类型的视图渲染，包括读取、写入、编辑和搜索类工具。
///
/// # 参数
///
/// * `app` - 应用状态引用，用于获取配置和状态信息
/// * `msg_idx` - 消息索引，用于标识所属消息
/// * `tool_idx` - 工具索引，用于标识消息中的具体工具调用
/// * `visible` - 工具执行的原始输出文本，格式为 "tool <工具名>\n<JSON数据>"
///
/// # 返回值
///
/// * `Some(Element)` - 成功解析并构建的视图元素
/// * `None` - 无法解析或不支持的工具类型
///
/// # 处理流程
///
/// 1. 解析工具名称和状态（running/error/正常）
/// 2. 从输出中提取文件路径列表
/// 3. 应用文件过滤器（针对搜索类工具）
/// 4. 处理文件列表截断（超过上限时显示部分）
/// 5. 根据工具类型构建相应的视图布局
pub fn tool_files_view<'a>(
    app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
) -> Option<Element<'a, Message>> {
    // 解析工具输出的第一行获取工具名称
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    if tool_name.is_empty() {
        return None;
    }
    // 解析 JSON 格式的工具输出数据
    let v = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    let input = tool_input(&v);

    if parse::should_skip_files_view(tool_name, input) {
        return None;
    }

    let raw_status = tool_status(&v);
    let is_error = matches!(raw_status, "error" | "failed" | "denied");
    let is_running = matches!(raw_status, "waiting" | "queued" | "running");
    let output = tool_output_text(&v).unwrap_or_default();
    let error_text = tool_error_text(&v);
    let (changes_by_path, items) = parse::parse_output_files(app, tool_name, input, &output, &v);
    let is_edit_like = parse::is_edit_like_tool(tool_name);

    if items.is_empty() && !is_edit_like && !parse::is_search_tool(tool_name) {
        return None;
    }
    let render_state = parse::build_file_list_state(
        items,
        parse::is_search_tool(tool_name),
        &app.tool_files_filter,
        100,
    );
    let view_ctx = FilesViewContext {
        app,
        msg_idx,
        tool_idx,
        visible: visible.to_string(),
        tool_name: tool_name.to_string(),
        error_text,
        input: input.to_string(),
        output: output.to_string(),
        verb: tool_verb(tool_name),
        is_error,
        is_running,
        is_edit_like,
        read_range: parse::parse_read_range(tool_name, input),
    };

    let list_column =
        list_ui::build_file_list_column(app, &render_state, &view_ctx, &changes_by_path);

    if matches!(tool_name, "write" | "file_write" | "file_edit" | "notebook_edit") {
        let permission_state = tool_permission_state(tool_name, &v);
        let title = write_view::write_tool_title(
            view_ctx.verb,
            view_ctx.is_running,
            permission_state,
            view_ctx.is_error,
        );
        let summary = tool_permission_target_summary(tool_name, &v)
            .or_else(|| write_view::write_tool_summary(&render_state))
            .unwrap_or_default();
        let key = ((msg_idx as u64) << 32) | tool_idx as u64;
        let detail_btn = button(
            icon_svg(Icon::ChevronRight)
                .width(Length::Fixed(10.0))
                .height(Length::Fixed(10.0))
                .style(eye_icon_svg_style),
        )
        .padding([2, 4])
        .style(|theme: &Theme, status| eye_icon_button_style(theme, status))
        .on_press(Message::Chat(message::ChatMessage::OpenToolDetail(
            msg_idx,
            tool_idx,
            visible.to_string(),
        )));
        let detail_slot: Element<'a, Message> = if app.chat_tool_hovered_idx == Some(key) {
            detail_btn.into()
        } else {
            Space::new().width(Length::Fixed(22.0)).into()
        };
        let head = tool_activity_header(
            tool_name,
            title,
            summary,
            tool_visual_state(raw_status),
            detail_slot,
        );
        let body = write_view::build_write_tool_view(&view_ctx, &render_state, list_column);
        return Some(
            mouse_area(
                container(column![head, body].spacing(8))
                    .padding([2, 6])
                    .width(Length::Fill)
                    .style(simplified_block_style),
            )
            .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
            .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave))
            .into(),
        );
    }

    Some(common_view::build_common_tool_view(&view_ctx, &render_state, list_column))
}
