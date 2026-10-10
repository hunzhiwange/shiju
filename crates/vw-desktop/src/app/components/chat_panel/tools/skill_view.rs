//! 技能工具的极简聊天流视图。
//!
//! 该视图只展示技能名称，完整技能内容交给已有工具详情弹窗承载。

use iced::widget::{button, container, mouse_area};
use iced::{Element, Length, Padding, Theme};
use serde_json::Value;

use crate::app::assets::Icon;
use crate::app::components::chat_panel::utils::{
    eye_icon_button_style, eye_icon_svg_style, icon_svg, truncate_chars,
};
use crate::app::{App, Message, message};

use super::tool_meta::{tool_activity_header, tool_visual_state};
use super::tool_parse::{tool_error_text, tool_input, tool_output_text, tool_status};
use super::{canonical_tool_name, tool_permission_error_text};

pub fn tool_skill_view<'a>(
    _app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
) -> Option<Element<'a, Message>> {
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    if tool_name != "skill" {
        return None;
    }

    let value = serde_json::from_str::<Value>(rest.trim()).ok()?;
    let status = tool_status(&value);
    let is_error = matches!(status, "error" | "failed" | "denied");
    let visual = tool_visual_state(status);
    let output = tool_output_text(&value).unwrap_or_default();
    let error = tool_permission_error_text(tool_name, &value)
        .or_else(|| tool_error_text(&value))
        .unwrap_or_default();
    let skill_name = skill_display_name(tool_input(&value), &output, &error);
    let title = if is_error { "技能失败" } else { "技能" };

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

    let content: Element<'a, Message> = mouse_area(
        container(tool_activity_header("skill", title, skill_name, visual, detail_btn.into()))
            .padding(Padding { top: 4.0, right: 6.0, bottom: 4.0, left: 0.0 })
            .width(Length::Fill),
    )
    .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
    .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave))
    .into();

    Some(content)
}

pub(super) fn skill_display_name(input: &str, output: &str, error: &str) -> String {
    skill_name_from_input(input)
        .or_else(|| skill_name_from_output(output))
        .or_else(|| skill_name_from_output(error))
        .map(|name| truncate_chars(&name, 80).to_string())
        .unwrap_or_else(|| "未知技能".to_string())
}

pub(super) fn skill_name_from_input(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    if let Ok(value) = serde_json::from_str::<Value>(input) {
        return skill_name_from_json_value(&value);
    }

    Some(input.trim_matches(&['"', '\''][..]).trim())
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
}

pub(super) fn skill_name_from_json_value(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str().map(str::trim).filter(|text| !text.is_empty()) {
        return Some(text.to_string());
    }

    let object = value.as_object()?;
    for key in ["name", "skill", "skill_name", "skillName", "id"] {
        if let Some(text) = object.get(key).and_then(Value::as_str).map(str::trim)
            && !text.is_empty()
        {
            return Some(text.to_string());
        }
    }
    None
}

pub(super) fn skill_name_from_output(output: &str) -> Option<String> {
    quoted_attr_value(output, "name").or_else(|| yaml_name_value(output))
}

pub(super) fn quoted_attr_value(text: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=");
    let start = text.find(&needle)? + needle.len();
    let rest = text[start..].trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let value_start = quote.len_utf8();
    let value = &rest[value_start..];
    let end = value.find(quote)?;
    let value = value[..end].trim();
    if value.is_empty() { None } else { Some(value.to_string()) }
}

pub(super) fn yaml_name_value(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix("name:")?.trim();
        let value = value.trim_matches(&['"', '\''][..]).trim();
        if value.is_empty() { None } else { Some(value.to_string()) }
    })
}
