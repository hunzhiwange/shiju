//! 渲染工作流节点执行进度。
//!
//! 聊天流只保留节点标题、状态与短摘要；完整节点输出统一交给工具详情弹窗。

use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::text::Wrapping;
use iced::widget::{button, container, mouse_area, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};
use serde_json::Value;

use crate::app::assets::Icon;
use crate::app::components::chat_panel::utils::{
    copy_tooltip_content, eye_icon_button_style, eye_icon_svg_style, icon_svg, truncate_chars,
};
use crate::app::{App, Message, message};

use super::tool_meta::{tool_activity_header, tool_visual_state};
use super::{canonical_tool_name, tool_error_text, tool_output_text, tool_status};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkflowNodeState {
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
}

const PLAN_RUN_CARD_MAX_WIDTH: f32 = 520.0;
const PLAN_RUN_TOOLTIP_MAX_WIDTH: f32 = 440.0;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlanRunCardDisplay {
    primary_text: String,
    summary_text: String,
    tooltip_text: Option<String>,
}

fn workflow_node_state(status: &str) -> WorkflowNodeState {
    match status {
        "running" => WorkflowNodeState::Running,
        "waiting" => WorkflowNodeState::Waiting,
        "error" | "failed" | "denied" => WorkflowNodeState::Failed,
        "cancelled" => WorkflowNodeState::Cancelled,
        _ => WorkflowNodeState::Completed,
    }
}

fn workflow_preview_message(value: &Value) -> Option<Message> {
    let metadata = value.get("metadata")?.as_object()?;
    let workflow_yaml = metadata.get("workflow_yaml")?.as_str()?.trim().to_string();
    if workflow_yaml.is_empty() {
        return None;
    }
    let focus_node_id = metadata
        .get("node_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    Some(Message::WorkflowTool(crate::apps::workflow::WorkflowMessage::OpenInlineYaml {
        workflow_yaml,
        focus_node_id,
    }))
}

fn icon_button<'a>(icon: Icon, label: &'static str, message: Message) -> Element<'a, Message> {
    let button = button(
        icon_svg(icon)
            .width(Length::Fixed(11.0))
            .height(Length::Fixed(11.0))
            .style(eye_icon_svg_style),
    )
    .padding([2, 4])
    .style(|theme: &Theme, status| eye_icon_button_style(theme, status))
    .on_press(message);

    Tooltip::new(button, copy_tooltip_content(label), TooltipPosition::Top).gap(6).into()
}

fn status_text(state: WorkflowNodeState) -> &'static str {
    match state {
        WorkflowNodeState::Running => "正在执行节点",
        WorkflowNodeState::Waiting => "正在等待后代智能体",
        WorkflowNodeState::Completed => "节点执行完成",
        WorkflowNodeState::Failed => "节点执行失败",
        WorkflowNodeState::Cancelled => "节点执行已取消",
    }
}

fn metadata_value<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.get("metadata").and_then(|metadata| metadata.get(key))
}

fn metadata_string(value: &Value, key: &str) -> String {
    metadata_value(value, key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
        .to_string()
}

fn plan_run_card_display(value: &Value) -> Option<PlanRunCardDisplay> {
    if metadata_value(value, "plan_run").and_then(Value::as_bool) != Some(true) {
        return None;
    }

    let task_path = metadata_string(value, "task_path");
    let (primary_text, tooltip_text) = match metadata_string(value, "event_type").as_str() {
        "planning_started" | "plan_created" => {
            let primary_text = if metadata_string(value, "event_type") == "planning_started" {
                "正在规划计划任务".to_string()
            } else {
                metadata_value(value, "subtask_count")
                    .and_then(Value::as_u64)
                    .map(|count| format!("计划任务已创建，共 {count} 个任务。"))
                    .unwrap_or_else(|| "计划任务已创建。".to_string())
            };
            let mut tooltip_lines = Vec::new();
            let task_id = metadata_string(value, "task_id");
            if !task_id.is_empty() {
                tooltip_lines.push(format!("UUID：{task_id}"));
            }
            if !task_path.is_empty() {
                tooltip_lines.push(format!("任务路径：{task_path}"));
            }
            (primary_text, (!tooltip_lines.is_empty()).then(|| tooltip_lines.join("\n")))
        }
        "subtask_started" | "subtask_finished" => {
            let title = metadata_string(value, "title");
            let primary_text = if title.is_empty() { "工作流节点".to_string() } else { title };
            let tooltip_text =
                (!task_path.is_empty()).then(|| format!("任务路径：{task_path}"));
            (primary_text, tooltip_text)
        }
        _ => return None,
    };

    Some(PlanRunCardDisplay {
        primary_text,
        summary_text: String::new(),
        tooltip_text,
    })
}

fn plan_run_tooltip_content(label: String) -> Element<'static, Message> {
    container(text(label).size(12).wrapping(Wrapping::WordOrGlyph))
        .padding([6, 10])
        .max_width(PLAN_RUN_TOOLTIP_MAX_WIDTH)
        .style(|_theme: &Theme| iced::widget::container::Style {
            text_color: Some(Color::from_rgb8(0xF3, 0xF4, 0xF6)),
            background: Some(Background::Color(Color::from_rgb8(0x0B, 0x0B, 0x0B))),
            border: Border {
                width: 1.0,
                color: Color::from_rgb8(0x14, 0x14, 0x14),
                radius: 6.0.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK.scale_alpha(0.25),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 6.0,
            },
            ..Default::default()
        })
        .into()
}

fn elapsed_label(value: &Value) -> Option<String> {
    let seconds = metadata_value(value, "elapsed_time").and_then(Value::as_f64)?;
    if seconds <= 0.0 {
        return None;
    }
    if seconds < 1.0 {
        return Some(format!("{:.0}ms", seconds * 1000.0));
    }
    Some(format!("{seconds:.2}s"))
}

fn token_value(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
                .or_else(|| {
                    value.as_f64().and_then(|number| {
                        if number.is_finite() && number >= 0.0 {
                            Some(number.round() as u64)
                        } else {
                            None
                        }
                    })
                })
        })
    })
}

fn output_value(value: &Value) -> Option<Value> {
    let raw_output = value.get("output").and_then(Value::as_str)?.trim();
    serde_json::from_str::<Value>(raw_output).ok()
}

fn usage_value(value: &Value) -> Option<Value> {
    metadata_value(value, "usage")
        .cloned()
        .or_else(|| value.get("usage").cloned())
        .or_else(|| output_value(value).and_then(|output| output.get("usage").cloned()))
}

fn usage_label(value: &Value) -> Option<String> {
    let usage = usage_value(value)?;
    let input = token_value(&usage, &["prompt_tokens", "input_tokens"]);
    let output = token_value(&usage, &["completion_tokens", "output_tokens"]);
    let reasoning = token_value(&usage, &["reasoning_tokens"]);
    let cached = token_value(&usage, &["cached_tokens", "cache_read_tokens"]);
    let total = token_value(&usage, &["total_tokens"]).or_else(|| {
        let total = input.unwrap_or(0) + output.unwrap_or(0) + reasoning.unwrap_or(0);
        (total > 0).then_some(total)
    });

    let mut parts = Vec::new();
    if let Some(input) = input.filter(|value| *value > 0) {
        parts.push(format!("输入 {input}"));
    }
    if let Some(output) = output.filter(|value| *value > 0) {
        parts.push(format!("输出 {output}"));
    }
    if let Some(reasoning) = reasoning.filter(|value| *value > 0) {
        parts.push(format!("推理 {reasoning}"));
    }
    if let Some(cached) = cached.filter(|value| *value > 0) {
        parts.push(format!("缓存 {cached}"));
    }
    if let Some(total) = total.filter(|value| *value > 0) {
        parts.push(format!("总计 {total}"));
    }

    (!parts.is_empty()).then(|| format!("Token {}", parts.join(" · ")))
}

fn output_preview_text(value: &Value, state: WorkflowNodeState) -> String {
    if state == WorkflowNodeState::Failed {
        return tool_error_text(value).unwrap_or_else(|| "节点执行失败".to_string());
    }

    let Some(raw_output) = tool_output_text(value) else {
        return if state == WorkflowNodeState::Running {
            "正在执行工作流节点".to_string()
        } else {
            "节点执行完成".to_string()
        };
    };
    let trimmed = raw_output.trim();
    if trimmed.is_empty()
        || (state == WorkflowNodeState::Running && trimmed == "正在执行工作流节点")
    {
        return if state == WorkflowNodeState::Running {
            "正在执行工作流节点".to_string()
        } else {
            "节点执行完成".to_string()
        };
    }

    if let Ok(mut parsed) = serde_json::from_str::<Value>(trimmed) {
        for key in ["answer", "text", "result"] {
            if let Some(text) = parsed
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                return text.to_string();
            }
        }
        if let Some(map) = parsed.as_object_mut() {
            map.remove("usage");
            if map.is_empty() {
                return "节点执行完成".to_string();
            }
        }
        return serde_json::to_string(&parsed).unwrap_or_else(|_| trimmed.to_string());
    }

    trimmed.to_string()
}

fn node_meta_label(value: &Value, node_type: &str) -> String {
    let mut parts = Vec::new();
    if let Some(index) = metadata_value(value, "index").and_then(Value::as_u64) {
        parts.push(format!("#{index}"));
    }
    if !node_type.is_empty() {
        parts.push(node_type.to_string());
    }
    if let Some(elapsed) = elapsed_label(value) {
        parts.push(format!("耗时 {elapsed}"));
    }
    if let Some(usage) = usage_label(value) {
        parts.push(usage);
    }
    parts.join(" · ")
}

fn chat_summary_text(value: &Value, state: WorkflowNodeState) -> String {
    let preview = output_preview_text(value, state);
    let meta = node_meta_label(value, &metadata_string(value, "node_type"));
    let summary = if meta.is_empty() {
        preview
    } else if preview == status_text(state)
        || preview == "正在执行工作流节点"
        || preview == "节点执行完成"
        || preview == "节点执行失败"
    {
        meta
    } else {
        format!("{preview} · {meta}")
    };
    truncate_chars(&summary, 96)
}

fn workflow_detail_content(value: &Value, state: WorkflowNodeState) -> String {
    if state == WorkflowNodeState::Failed {
        return tool_error_text(value).unwrap_or_else(|| "节点执行失败".to_string());
    }
    let Some(raw_output) = tool_output_text(value) else {
        return status_text(state).to_string();
    };
    let trimmed = raw_output.trim();
    if trimmed.is_empty() {
        return status_text(state).to_string();
    }
    if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
        return serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| trimmed.to_string());
    }
    trimmed.to_string()
}

fn workflow_tool_detail_message(msg_idx: usize, tool_idx: usize, visible: &str) -> Message {
    Message::Chat(message::ChatMessage::OpenToolDetail(msg_idx, tool_idx, visible.to_string()))
}

/// 节点主点击：只要会话已登记且项目路径可用，即可进入对应子会话。
/// 独立详情按钮始终保留，供用户查看原始节点信息。
fn workflow_node_primary_message(
    project_path: Option<&str>,
    value: &Value,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
) -> Message {
    let session_id = metadata_string(value, "session_id");
    let session_directory = metadata_string(value, "session_directory");
    let session_directory = (!session_directory.is_empty()).then_some(session_directory.as_str());
    if !session_id.is_empty()
        && let Some(path) = session_directory
            .or_else(|| project_path.map(str::trim).filter(|path| !path.is_empty()))
    {
        return Message::Project(message::ProjectMessage::OpenProjectSessionPressed(
            path.to_string(),
            session_id,
        ));
    }
    workflow_tool_detail_message(msg_idx, tool_idx, visible)
}

/// 渲染聊天中的工作流节点执行卡片（单行摘要 + 详情弹窗入口）。
pub fn tool_workflow_view<'a>(
    app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
) -> Option<Element<'a, Message>> {
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    if tool_name != "workflow_node" {
        return None;
    }

    let value = serde_json::from_str::<Value>(rest.trim()).ok()?;
    let status = tool_status(&value);
    let state = workflow_node_state(status);
    let visual = tool_visual_state(status);
    let plan_run_card = plan_run_card_display(&value);
    let (title, summary) = if let Some(display) = &plan_run_card {
        (display.primary_text.clone(), display.summary_text.clone())
    } else {
        let title = metadata_string(&value, "title");
        let title = if title.is_empty() { "工作流节点".to_string() } else { title };
        (title, chat_summary_text(&value, state))
    };
    let primary_message = workflow_node_primary_message(
        app.project_path.as_deref(),
        &value,
        msg_idx,
        tool_idx,
        visible,
    );
    let detail_message = workflow_tool_detail_message(msg_idx, tool_idx, visible);

    let detail_btn = button(
        icon_svg(Icon::ChevronRight)
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .style(eye_icon_svg_style),
    )
    .padding([2, 4])
    .style(|theme: &Theme, status| eye_icon_button_style(theme, status))
    .on_press(detail_message);

    let preview_btn = workflow_preview_message(&value)
        .map(|message| icon_button(Icon::Grid1x2, "预览工作流", message));
    let open_session_btn = matches!(
        &primary_message,
        Message::Project(message::ProjectMessage::OpenProjectSessionPressed(..))
    )
    .then(|| icon_button(Icon::ChatTextFill, "打开子会话", primary_message.clone()));

    let mut actions = row![].spacing(4).align_y(Alignment::Center);
    if let Some(preview_btn) = preview_btn {
        actions = actions.push(preview_btn);
    }
    if let Some(open_session_btn) = open_session_btn {
        actions = actions.push(open_session_btn);
    }
    actions = actions.push(detail_btn);

    let card = container(tool_activity_header(
        "workflow_node",
        title,
        summary,
        visual,
        actions.into(),
    ))
    .padding(Padding { top: 4.0, right: 6.0, bottom: 4.0, left: 0.0 });
    let card = if plan_run_card.is_some() {
        card.width(Length::Shrink).max_width(PLAN_RUN_CARD_MAX_WIDTH)
    } else {
        card.width(Length::Fill)
    };
    let card: Element<'a, Message> = mouse_area(card)
        .on_press(primary_message)
        .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
        .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave))
        .into();

    Some(match plan_run_card.and_then(|display| display.tooltip_text) {
        Some(tooltip_text) => Tooltip::new(
            card,
            plan_run_tooltip_content(tooltip_text),
            TooltipPosition::Top,
        )
        .gap(6.0)
        .into(),
        None => card,
    })
}

/// 供详情弹窗复用的节点正文（pretty JSON / 错误文本）。
pub(crate) fn workflow_node_detail_body(value: &Value) -> String {
    let state = workflow_node_state(tool_status(value));
    workflow_detail_content(value, state)
}

#[cfg(test)]
#[path = "workflow_view_tests.rs"]
mod tests;
