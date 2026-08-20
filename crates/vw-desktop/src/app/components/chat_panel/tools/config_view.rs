//! 渲染配置工具结果。
//! 视图解析结构化配置操作结果，并以明确状态展示成功、失败和变更值。

use iced::widget::{Space, button, column, container, mouse_area, row, text};
use iced::{Background, Border, Element, Length, Theme};
use serde::Deserialize;
use serde_json::Value;

use crate::app::assets::Icon;
use crate::app::components::chat_panel::utils::{
    bold_font, chat_secondary_muted_text_color, chat_secondary_text_color, eye_icon_button_style,
    eye_icon_svg_style, icon_svg, simplified_block_style, truncate_chars,
};
use crate::app::{App, Message, message};

use super::canonical_tool_name;
use super::tool_meta::{tool_activity_header, tool_visual_state};
use super::tool_parse::{
    tool_error_text, tool_input, tool_output_text, tool_result_data, tool_status,
};

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ConfigResultData {
    success: bool,
    #[serde(default)]
    operation: Option<String>,
    #[serde(default)]
    setting: Option<String>,
    #[serde(default)]
    value: Option<Value>,
    #[serde(default)]
    previous_value: Option<Value>,
    #[serde(default)]
    new_value: Option<Value>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct ConfigInputData {
    #[serde(default)]
    setting: Option<String>,
    #[serde(default)]
    value: Option<Value>,
    #[serde(default)]
    section: Option<String>,
}

fn value_label(value: &Value) -> String {
    match value {
        Value::Null => "default".to_string(),
        Value::String(text) => text.clone(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        other => serde_json::to_string(other).unwrap_or_else(|_| other.to_string()),
    }
}

fn parse_config_result_from_output(output: &str) -> Option<ConfigResultData> {
    let trimmed = output.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

fn parse_config_result(value: &Value) -> Option<ConfigResultData> {
    tool_result_data(value)
        .and_then(|data| serde_json::from_value::<ConfigResultData>(data.clone()).ok())
        .or_else(|| {
            tool_output_text(value).and_then(|output| parse_config_result_from_output(&output))
        })
}

fn parse_config_input(input: &str) -> Option<ConfigInputData> {
    let trimmed = input.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

fn summary_from_result(result: &ConfigResultData) -> String {
    if !result.success {
        return result.error.clone().unwrap_or_else(|| "配置操作失败".to_string());
    }

    match result.operation.as_deref() {
        Some("get") => match (&result.setting, &result.value) {
            (Some(setting), Some(value)) => format!("{} = {}", setting, value_label(value)),
            _ => "当前配置概览".to_string(),
        },
        Some("set") => match (&result.setting, &result.new_value) {
            (Some(setting), Some(value)) => format!("{} -> {}", setting, value_label(value)),
            _ => "配置已更新".to_string(),
        },
        _ => "配置".to_string(),
    }
}

fn summary_from_input(input: &ConfigInputData) -> String {
    if let Some(setting) = input.setting.as_deref().map(str::trim).filter(|value| !value.is_empty())
    {
        if let Some(value) = input.value.as_ref() {
            return format!("设置 {} = {}", setting, value_label(value));
        }
        return format!("读取 {}", setting);
    }

    match input.section.as_deref() {
        Some("proxy") => "读取 proxy 高级配置".to_string(),
        Some("model_routing") => "读取 model_routing 高级配置".to_string(),
        _ => "读取配置概览".to_string(),
    }
}

fn success_body<'a>(result: &ConfigResultData) -> Option<Element<'a, Message>> {
    match result.operation.as_deref() {
        Some("get") => {
            if let (Some(setting), Some(value)) = (&result.setting, &result.value) {
                let display = value_label(value);
                return Some(
                    row![
                        text(setting.clone()).size(12).font(bold_font()).style(|theme: &Theme| {
                            iced::widget::text::Style {
                                color: Some(chat_secondary_text_color(theme)),
                            }
                        }),
                        text(truncate_chars(&display, 180).to_string()).size(12).style(
                            |theme: &Theme| iced::widget::text::Style {
                                color: Some(chat_secondary_muted_text_color(theme)),
                            }
                        )
                    ]
                    .spacing(8)
                    .into(),
                );
            }

            if let Some(value) = result.value.as_ref() {
                if let Some(object) = value.as_object() {
                    let keys = object.keys().take(5).cloned().collect::<Vec<_>>();
                    let mut text_value = if keys.is_empty() {
                        "已返回配置概览，点击查看详情。".to_string()
                    } else {
                        format!("包含 {} 个配置段：{}", object.len(), keys.join("、"))
                    };
                    if object.len() > keys.len() {
                        text_value.push('…');
                    }
                    return Some(
                        text(truncate_chars(&text_value, 180).to_string())
                            .size(12)
                            .style(|theme: &Theme| iced::widget::text::Style {
                                color: Some(chat_secondary_muted_text_color(theme)),
                            })
                            .into(),
                    );
                }
            }

            None
        }
        Some("set") => {
            let mut body = column![].spacing(6);

            if let Some(previous_value) = result.previous_value.as_ref() {
                let display = value_label(previous_value);
                body = body.push(
                    row![
                        text("旧值").size(12).font(bold_font()).style(|theme: &Theme| {
                            iced::widget::text::Style {
                                color: Some(chat_secondary_text_color(theme)),
                            }
                        }),
                        text(truncate_chars(&display, 180).to_string()).size(12).style(
                            |theme: &Theme| iced::widget::text::Style {
                                color: Some(chat_secondary_muted_text_color(theme)),
                            }
                        )
                    ]
                    .spacing(8),
                );
            }

            if let Some(new_value) = result.new_value.as_ref() {
                let display = value_label(new_value);
                body = body.push(
                    row![
                        text("新值").size(12).font(bold_font()).style(|theme: &Theme| {
                            iced::widget::text::Style {
                                color: Some(chat_secondary_text_color(theme)),
                            }
                        }),
                        text(truncate_chars(&display, 180).to_string()).size(12).style(
                            |theme: &Theme| iced::widget::text::Style {
                                color: Some(chat_secondary_muted_text_color(theme)),
                            }
                        )
                    ]
                    .spacing(8),
                );
            }

            Some(body.into())
        }
        _ => None,
    }
}

/// 执行 tool_config_view 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn tool_config_view<'a>(
    app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
) -> Option<Element<'a, Message>> {
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    if !tool_name.eq_ignore_ascii_case("config") {
        return None;
    }

    let value = serde_json::from_str::<Value>(rest.trim()).ok()?;
    let status = tool_status(&value);
    let is_running = status == "running";
    let is_waiting = matches!(status, "waiting" | "queued");
    let visual = tool_visual_state(status);
    let result = parse_config_result(&value);
    let has_lifecycle = matches!(
        status,
        "waiting" | "queued" | "running" | "completed" | "success" | "error" | "failed"
            | "denied" | "cancelled"
    );
    let allows_header_without_result = matches!(
        status,
        "waiting" | "queued" | "running" | "error" | "failed" | "denied" | "cancelled"
    );

    if result.is_none() && !allows_header_without_result {
        return None;
    }

    let input = parse_config_input(tool_input(&value));
    let is_error =
        matches!(status, "error" | "failed" | "denied")
            || result.as_ref().is_some_and(|item| !item.success);
    let summary = result
        .as_ref()
        .map(summary_from_result)
        .filter(|summary| !summary.trim().is_empty())
        .or_else(|| input.as_ref().map(summary_from_input))
        .unwrap_or_default();
    let summary = truncate_chars(summary.replace(['\n', '\r'], " ").trim(), 120).to_string();

    if summary.is_empty() && !has_lifecycle && !is_error {
        return None;
    }

    let title = if is_running || is_waiting {
        if input.as_ref().and_then(|item| item.value.as_ref()).is_some() {
            "设置配置"
        } else {
            "读取配置"
        }
    } else if is_error {
        "配置失败"
    } else if result.as_ref().and_then(|item| item.operation.as_deref()) == Some("set") {
        "设置配置"
    } else {
        "配置"
    };

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
    let detail_slot: Element<'a, Message> =
        if is_hovered { detail_btn.into() } else { Space::new().width(Length::Fixed(22.0)).into() };

    let head_row = tool_activity_header("config", title, summary, visual, detail_slot);

    let body: Option<Element<'a, Message>> = if is_error {
        let error_text = result
            .as_ref()
            .and_then(|item| item.error.as_deref())
            .map(ToOwned::to_owned)
            .or_else(|| tool_error_text(&value))
            .unwrap_or_else(|| "配置操作失败".to_string());

        Some(
            container(text(truncate_chars(&error_text, 220).to_string()).size(12).style(
                |theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().danger.base.color),
                },
            ))
            .padding([8, 10])
            .width(Length::Fill)
            .style(|theme: &Theme| {
                let ext = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(Background::Color(ext.danger.base.color.scale_alpha(0.07))),
                    border: Border {
                        width: 1.0,
                        color: ext.danger.base.color.scale_alpha(0.30),
                        radius: 12.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into(),
        )
    } else if is_running || is_waiting {
        let pending = if input.as_ref().and_then(|item| item.value.as_ref()).is_some() {
            if is_waiting { "等待写入配置…" } else { "正在写入配置…" }
        } else {
            if is_waiting { "等待读取配置…" } else { "正在读取配置…" }
        };
        Some(
            text(pending)
                .size(12)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(chat_secondary_muted_text_color(theme)),
                })
                .into(),
        )
    } else {
        result.as_ref().and_then(success_body)
    };

    let mut content = column![head_row].spacing(10);
    if let Some(body) = body {
        content = content.push(body);
    }

    Some(
        mouse_area(
            container(content).padding([8, 10]).width(Length::Fill).style(simplified_block_style),
        )
        .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
        .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave))
        .into(),
    )
}

#[cfg(test)]
#[path = "tests/config_view.rs"]
mod tests;

#[cfg(test)]
#[path = "config_view_tests.rs"]
mod coverage_tests;
