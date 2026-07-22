//! 计划模式工具结果视图。
//!
//! 本模块负责把计划模式工具的结构化输出压缩成可读摘要、元信息和详情区域。

use iced::widget::{Space, button, column, container, mouse_area, row, text};
/// 重新导出 use iced::{Alignment, Element, Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Alignment, Element, Length, Theme};
/// 重新导出 use serde_json::{Map, Value}，让上层模块通过稳定路径访问。
use serde_json::{Map, Value};

/// 重新导出 use crate::app::assets::Icon，让上层模块通过稳定路径访问。
use crate::app::assets::Icon;
/// 重新导出 use crate::app::components::chat_panel::utils::{，让上层模块通过稳定路径访问。
use crate::app::components::chat_panel::utils::{
    chat_secondary_muted_text_color, eye_icon_button_style, eye_icon_svg_style, icon_svg,
    simplified_block_style, truncate_chars,
};
/// 重新导出 use crate::app::{App, Message, message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message, message};

/// 重新导出 use super::tool_meta::tool_header_title，让上层模块通过稳定路径访问。
use super::tool_meta::tool_header_title;
/// 重新导出 use super::tool_parse::{，让上层模块通过稳定路径访问。
use super::tool_parse::{
    tool_error_text, tool_output_text, tool_result_data, tool_status, tool_summary_text,
};
/// 重新导出 use super::{，让上层模块通过稳定路径访问。
use super::{
    ToolPermissionState, canonical_tool_name, tool_permission_error_text, tool_permission_state,
    tool_permission_title,
};

/// 处理 is plan mode tool 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `true` 表示当前输入满足该辅助函数描述的条件。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn is_plan_mode_tool(tool_name: &str) -> bool {
    matches!(tool_name, "enter_plan_mode" | "exit_plan_mode" | "verify_plan_execution")
}

/// 处理 string field 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `None` 表示输入缺少必要字段、当前状态不匹配，或该视图片段不需要展示。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn string_field<'a>(data: Option<&'a Map<String, Value>>, key: &str) -> Option<&'a str> {
    data.and_then(|items| items.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// 处理 bool field 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `true` 表示当前输入满足该辅助函数描述的条件。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn bool_field(data: Option<&Map<String, Value>>, key: &str) -> bool {
    data.and_then(|items| items.get(key)).and_then(Value::as_bool).unwrap_or(false)
}

/// 处理 u64 field 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn u64_field(data: Option<&Map<String, Value>>, key: &str) -> u64 {
    data.and_then(|items| items.get(key)).and_then(Value::as_u64).unwrap_or(0)
}

/// 处理 string list field 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回集合保持输入顺序或界面展示顺序，空集合表示没有可展示项。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn string_list_field(data: Option<&Map<String, Value>>, key: &str) -> Vec<String> {
    data.and_then(|items| items.get(key))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// 生成 derived summary，用于工具卡片或状态行的简短说明。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回字符串已经按界面展示或比较需求做过必要整理。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn derived_summary(tool_name: &str, data: Option<&Map<String, Value>>) -> String {
    match tool_name {
        "enter_plan_mode" => {
            if bool_field(data, "already_active") {
                "Plan mode remains active".to_string()
            } else if bool_field(data, "active") {
                "Plan mode enabled".to_string()
            } else {
                // String 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                String::new()
            }
        }
        "exit_plan_mode" => {
            if bool_field(data, "exited") {
                "Plan mode disabled".to_string()
            } else if bool_field(data, "active") {
                "Plan mode remains active".to_string()
            } else {
                // String 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                String::new()
            }
        }
        "verify_plan_execution" => {
            if bool_field(data, "ready") {
                format!("Ready to execute {} todo(s)", u64_field(data, "pending_count"))
            } else {
                let blockers = string_list_field(data, "blockers");
                if blockers.is_empty() {
                    // String 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    String::new()
                } else {
                    format!("Blocked by {} issue(s)", blockers.len())
                }
            }
        }
        _ => String::new(),
    }
}

/// 处理 metadata text 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回字符串已经按界面展示或比较需求做过必要整理。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn metadata_text(tool_name: &str, data: Option<&Map<String, Value>>) -> String {
    let mut parts = Vec::new();
    if let Some(goal) = string_field(data, "goal") {
        parts.push(format!("Goal: {}", truncate_chars(goal, 64)));
    }
    if let Some(note) = string_field(data, "note") {
        parts.push(format!("Note: {}", truncate_chars(note, 64)));
    }
    if tool_name == "verify_plan_execution" {
        let todo_count = u64_field(data, "todo_count");
        let pending_count = u64_field(data, "pending_count");
        let in_progress_count = u64_field(data, "in_progress_count");
        parts.push(format!(
            "Todo: {} · Pending: {} · In progress: {}",
            todo_count, pending_count, in_progress_count
        ));
    }
    parts.join(" · ")
}

pub(super) fn plan_mode_base_title(tool_name: &str) -> Option<&'static str> {
    match tool_name {
        "enter_plan_mode" => Some("进入规划模式"),
        "exit_plan_mode" => Some("退出规划模式"),
        "verify_plan_execution" => Some("校验计划执行"),
        _ => None,
    }
}

pub(super) fn plan_mode_title(
    base_title: &str,
    permission_state: Option<ToolPermissionState>,
    is_running: bool,
    is_error: bool,
) -> String {
    if let Some(permission_state) = permission_state {
        tool_permission_title(base_title, permission_state)
    } else if is_running {
        format!("{} 运行中", base_title)
    } else if is_error {
        format!("{} 失败", base_title)
    } else {
        base_title.to_string()
    }
}

/// 处理 fallback output lines 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回集合保持输入顺序或界面展示顺序，空集合表示没有可展示项。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
fn fallback_output_lines(value: &Value) -> Vec<String> {
    let output = tool_output_text(value).unwrap_or_default();
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let lines = trimmed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if lines.is_empty() { vec![trimmed.to_string()] } else { lines }
}

/// 处理 body lines 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回集合保持输入顺序或界面展示顺序，空集合表示没有可展示项。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn body_lines(tool_name: &str, value: &Value, is_error: bool) -> Vec<String> {
    if is_error {
        return tool_permission_error_text(tool_name, value)
            .or_else(|| tool_error_text(value))
            .map(|text| {
                text.lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    }

    let data = tool_result_data(value).and_then(Value::as_object);
    match tool_name {
        "enter_plan_mode" => {
            let mut lines = Vec::new();
            if let Some(message) = string_field(data, "message") {
                lines.push(message.to_string());
            }
            let instructions = string_list_field(data, "instructions");
            if !instructions.is_empty() {
                lines.push("In plan mode, you should:".to_string());
                lines.extend(
                    instructions
                        .iter()
                        .enumerate()
                        .map(|(index, line)| format!("{}. {}", index + 1, line)),
                );
                lines.push(
                    "Remember: DO NOT write or edit any files yet. This is a read-only exploration and planning phase."
                        .to_string(),
                );
                return lines;
            }
            if !lines.is_empty() {
                return lines;
            }
        }
        "exit_plan_mode" => {
            if bool_field(data, "exited") {
                return vec!["Plan mode exited. Execution can continue.".to_string()];
            }
            let blockers = string_list_field(data, "blockers");
            if !blockers.is_empty() {
                let mut lines = vec!["Plan mode exit is currently blocked:".to_string()];
                lines.extend(blockers.into_iter().map(|line| format!("- {line}")));
                return lines;
            }
        }
        "verify_plan_execution" => {
            if bool_field(data, "ready") {
                return vec![format!(
                    "Ready to execute.\nTodo: {} · Pending: {} · In progress: {}",
                    u64_field(data, "todo_count"),
                    u64_field(data, "pending_count"),
                    u64_field(data, "in_progress_count")
                )];
            }
            let blockers = string_list_field(data, "blockers");
            if !blockers.is_empty() {
                let mut lines = vec!["Plan execution is currently blocked:".to_string()];
                lines.extend(blockers.into_iter().map(|line| format!("- {line}")));
                return lines;
            }
        }
        _ => {}
    }

    fallback_output_lines(value)
}

/// 处理 tool plan mode view 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn tool_plan_mode_view<'a>(
    app: &'a App,
    // msg_idx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    msg_idx: usize,
    // tool_idx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tool_idx: usize,
    // visible 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    visible: &str,
) -> Option<Element<'a, Message>> {
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    if !is_plan_mode_tool(tool_name) {
        return None;
    }

    let value = serde_json::from_str::<Value>(rest.trim()).ok()?;
    let status = tool_status(&value);
    let is_error = matches!(status, "error" | "denied");
    let is_running = status == "running";
    let data = tool_result_data(&value).and_then(Value::as_object);
    let permission_state = tool_permission_state(tool_name, &value);

    let base_title = plan_mode_base_title(tool_name)?;
    let title = plan_mode_title(base_title, permission_state, is_running, is_error);

    let summary = tool_summary_text(&value)
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| derived_summary(tool_name, data));
    let metadata = metadata_text(tool_name, data);
    let lines = body_lines(tool_name, &value, is_error);

    if summary.trim().is_empty() && metadata.is_empty() && lines.is_empty() && !is_running {
        return None;
    }

    let key = ((msg_idx as u64) << 32) | (tool_idx as u64);
    let is_hovered = app.chat_tool_hovered_idx == Some(key);
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
    let detail_slot: Element<'a, Message> = if is_hovered {
        detail_btn.into()
    } else {
        // Space 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Space::new().width(Length::Fixed(22.0)).into()
    };

    let head_row = row![
        row![
            tool_header_title(tool_name, title, is_error),
            text(summary).size(13).style(|theme: &Theme| iced::widget::text::Style {
                // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                color: Some(chat_secondary_muted_text_color(theme)),
            })
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        container(Space::new()).width(Length::Fill),
        detail_slot,
    ]
    .align_y(Alignment::Center);

    let mut body = column![];
    if !metadata.is_empty() {
        body =
            body.push(text(metadata).size(12).style(|theme: &Theme| iced::widget::text::Style {
                // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                color: Some(chat_secondary_muted_text_color(theme)),
            }));
    }

    if is_running {
        body = body.push(text(format!("{}中…", base_title)).size(14).style(|theme: &Theme| {
            // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            iced::widget::text::Style {
                // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.85)),
            }
        }));
    } else {
        for (index, line) in lines.iter().enumerate() {
            let is_primary = index == 0 && !line.starts_with('-') && !line.starts_with("1.");
            body = body.push(text(line.clone()).size(if is_primary { 14 } else { 13 }).style(
                move |theme: &Theme| {
                    let color = if is_error {
                        theme.extended_palette().danger.base.color.scale_alpha(0.95)
                    } else if is_primary {
                        theme.extended_palette().secondary.base.text.scale_alpha(0.96)
                    } else {
                        chat_secondary_muted_text_color(theme)
                    };
                    iced::widget::text::Style { color: Some(color) }
                },
            ));
        }
    }

    let card = mouse_area(
        container(column![head_row, body.spacing(6)].spacing(10))
            .padding([8, 10])
            .width(Length::Fill)
            .style(simplified_block_style),
    )
    .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
    .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave));

    Some(card.into())
}
