//! 渲染工作流节点执行进度。
//! 聊天流中的 workflow_node 工具块会被原位更新，用于展示节点状态、耗时和输出。

use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, mouse_area, row, text};
use iced::{Alignment, Background, Border, Element, Length, Theme};
use serde_json::Value;

use crate::app::assets::Icon;
use crate::app::components::chat_panel::utils::{
    chat_secondary_muted_text_color, chat_secondary_subtle_text_color, copy_tooltip_content,
    eye_icon_button_style, eye_icon_svg_style, icon_svg, simplified_block_style,
    simplified_code_block_style,
};
use crate::app::{App, Message, message};

use super::{
    canonical_tool_name, tool_error_text, tool_header_title, tool_output_text, tool_status,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkflowNodeState {
    Running,
    Completed,
    Failed,
}

fn workflow_node_state(status: &str) -> WorkflowNodeState {
    match status {
        "running" => WorkflowNodeState::Running,
        "error" | "failed" | "denied" => WorkflowNodeState::Failed,
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

fn status_label(state: WorkflowNodeState) -> &'static str {
    match state {
        WorkflowNodeState::Running => "运行中",
        WorkflowNodeState::Completed => "完成",
        WorkflowNodeState::Failed => "失败",
    }
}

fn status_text(state: WorkflowNodeState) -> &'static str {
    match state {
        WorkflowNodeState::Running => "正在执行节点",
        WorkflowNodeState::Completed => "节点执行完成",
        WorkflowNodeState::Failed => "节点执行失败",
    }
}

fn status_badge_style(theme: &Theme, state: WorkflowNodeState) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let color = match state {
        WorkflowNodeState::Running => palette.primary.base.color,
        WorkflowNodeState::Completed => palette.success.base.color,
        WorkflowNodeState::Failed => palette.danger.base.color,
    };
    iced::widget::container::Style {
        background: Some(Background::Color(color.scale_alpha(0.10))),
        border: Border { width: 1.0, color: color.scale_alpha(0.24), radius: 999.0.into() },
        text_color: Some(color),
        ..Default::default()
    }
}

fn output_block_style(theme: &Theme, state: WorkflowNodeState) -> iced::widget::container::Style {
    if state != WorkflowNodeState::Failed {
        return simplified_code_block_style(theme);
    }

    let danger = theme.extended_palette().danger.base.color;
    iced::widget::container::Style {
        background: Some(Background::Color(danger.scale_alpha(0.07))),
        border: Border { width: 1.0, color: danger.scale_alpha(0.30), radius: 14.0.into() },
        text_color: Some(danger),
        ..Default::default()
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
        return serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| trimmed.to_string());
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

/// 渲染聊天中的工作流节点执行卡片。
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
    let state = workflow_node_state(tool_status(&value));
    let is_error = state == WorkflowNodeState::Failed;
    let title = metadata_string(&value, "title");
    let title = if title.is_empty() { "工作流节点".to_string() } else { title };
    let node_type = metadata_string(&value, "node_type");
    let meta_label = node_meta_label(&value, &node_type);
    let preview_text = output_preview_text(&value, state);

    let key = ((msg_idx as u64) << 32) | (tool_idx as u64);
    let is_hovered = app.chat_tool_hovered_idx == Some(key);

    let preview_button = workflow_preview_message(&value)
        .map(|message| icon_button(Icon::Grid1x2, "预览工作流", message))
        .unwrap_or_else(|| Space::new().width(Length::Fixed(0.0)).into());
    let hover_slot: Element<'a, Message> =
        if is_hovered { preview_button } else { Space::new().width(Length::Fixed(22.0)).into() };

    let title_row = row![
        tool_header_title("workflow_node", title, is_error),
        container(text(status_label(state)).size(12))
            .padding([2, 8])
            .style(move |theme: &Theme| status_badge_style(theme, state)),
        container(Space::new()).width(Length::Fill),
        hover_slot,
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let meta_row = row![
        text(status_text(state)).size(13).style(move |theme: &Theme| {
            let color = match state {
                WorkflowNodeState::Running => theme.extended_palette().primary.base.color,
                WorkflowNodeState::Completed => chat_secondary_muted_text_color(theme),
                WorkflowNodeState::Failed => theme.extended_palette().danger.base.color,
            };
            iced::widget::text::Style { color: Some(color) }
        }),
        text(meta_label).size(12).style(|theme: &Theme| iced::widget::text::Style {
            color: Some(chat_secondary_subtle_text_color(theme)),
        }),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let output_color_state = state;
    let output_block =
        container(text(preview_text).size(13).font(iced::Font::with_name("JetBrains Mono")).style(
            move |theme: &Theme| iced::widget::text::Style {
                color: Some(if output_color_state == WorkflowNodeState::Failed {
                    theme.extended_palette().danger.base.color
                } else {
                    theme.palette().text
                }),
            },
        ))
        .padding([10, 12])
        .width(Length::Fill)
        .style(move |theme: &Theme| output_block_style(theme, state));

    let content = column![title_row, meta_row, output_block].spacing(8);
    let card = mouse_area(
        container(content).padding([10, 12]).width(Length::Fill).style(simplified_block_style),
    )
    .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
    .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave));

    Some(card.into())
}

#[cfg(test)]
#[path = "workflow_view_tests.rs"]
mod tests;
