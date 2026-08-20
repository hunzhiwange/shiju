//! MCP 工具的单行聊天流视图。
//!
//! 聊天流只保留 MCP 名称和详情入口，完整输入输出统一交给工具详情弹窗展示。

use iced::widget::{button, container, mouse_area};
use iced::{Element, Length, Padding, Theme};
use serde_json::Value;

use crate::app::assets::Icon;
use crate::app::components::chat_panel::utils::{
    eye_icon_button_style, eye_icon_svg_style, icon_svg, truncate_chars,
};
use crate::app::{App, Message, message};

use super::tool_meta::{tool_activity_header, tool_visual_state};
use super::tool_parse::{
    tool_input, tool_name_from_raw, tool_output_text, tool_status, tool_summary_text,
};

pub fn tool_mcp_view<'a>(
    _app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
) -> Option<Element<'a, Message>> {
    let (_, rest) = visible.split_once('\n')?;
    let tool_name = tool_name_from_raw(visible)?;
    if !is_mcp_tool_raw(visible) {
        return None;
    }

    let value = serde_json::from_str::<Value>(rest.trim()).ok()?;
    let status = tool_status(&value);
    let visual = tool_visual_state(status);
    let title = match status {
        "running" => "MCP 运行中",
        "error" | "failed" | "denied" => "MCP 失败",
        _ => "MCP",
    };
    let summary = mcp_display_name(&tool_name, &value);

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

    Some(
        mouse_area(
            container(
                tool_activity_header("MCP", title, summary, visual, detail_btn.into()),
            )
            .padding(Padding { top: 4.0, right: 6.0, bottom: 4.0, left: 0.0 })
            .width(Length::Fill),
        )
        .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
        .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave))
        .into(),
    )
}

pub(crate) fn is_mcp_tool_name(tool_name: &str) -> bool {
    let normalized = tool_name.trim().to_ascii_lowercase();
    normalized == "mcp"
        || normalized.starts_with("mcp_")
        || matches!(
            normalized.as_str(),
            "list_mcp_resources" | "list_mcp_resource_templates" | "read_mcp_resource"
        )
}

pub(crate) fn is_mcp_tool_raw(raw: &str) -> bool {
    let Some((_, rest)) = raw.split_once('\n') else {
        return false;
    };
    if tool_name_from_raw(raw).as_deref().is_some_and(is_mcp_tool_name) {
        return true;
    }

    let Ok(value) = serde_json::from_str::<Value>(rest.trim()) else {
        return false;
    };
    if looks_like_mcp_payload(&value) {
        return true;
    }

    [tool_output_text(&value).as_deref(), Some(tool_input(&value))]
        .into_iter()
        .flatten()
        .filter_map(|candidate| serde_json::from_str::<Value>(candidate.trim()).ok())
        .any(|candidate| looks_like_mcp_payload(&candidate))
}

pub(super) fn mcp_display_name(tool_name: &str, value: &Value) -> String {
    if let Some(summary) = tool_summary_text(value).filter(|summary| !summary.trim().is_empty()) {
        return truncate_chars(summary.trim(), 80).to_string();
    }

    for candidate in
        [tool_output_text(value).as_deref(), Some(tool_input(value))].into_iter().flatten()
    {
        if let Ok(candidate) = serde_json::from_str::<Value>(candidate.trim())
            && let Some(name) = mcp_server_name(&candidate)
        {
            return truncate_chars(name, 80).to_string();
        }
    }

    let normalized = tool_name.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "list_mcp_resources" => "资源列表".to_string(),
        "list_mcp_resource_templates" => "资源模板".to_string(),
        "read_mcp_resource" => "读取资源".to_string(),
        "mcp" => "详情".to_string(),
        _ if is_mcp_tool_name(&normalized) => normalized
            .strip_prefix("mcp_")
            .filter(|name| !name.is_empty())
            .unwrap_or("详情")
            .replace('_', " "),
        _ => "详情".to_string(),
    }
}

pub(super) fn looks_like_mcp_payload(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(looks_like_mcp_payload),
        Value::Object(object) => {
            let is_capability_report = object.contains_key("unsupported_capabilities")
                && (object.contains_key("auth_policy") || object.contains_key("auth_methods"));
            let is_server_inventory = object.get("server").is_some_and(Value::is_object)
                && ["resources", "prompts", "tools"]
                    .into_iter()
                    .any(|key| object.contains_key(key));
            is_capability_report
                || is_server_inventory
                || ["result", "data", "content"]
                    .into_iter()
                    .filter_map(|key| object.get(key))
                    .any(looks_like_mcp_payload)
        }
        _ => false,
    }
}

pub(super) fn mcp_server_name(value: &Value) -> Option<&str> {
    match value {
        Value::Array(items) => items.iter().find_map(mcp_server_name),
        Value::Object(object) => {
            if let Some(name) = object
                .get("server")
                .and_then(Value::as_object)
                .and_then(|server| server.get("name"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty())
            {
                return Some(name);
            }

            for key in ["serverName", "server_name"] {
                if let Some(name) = object
                    .get(key)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                {
                    return Some(name);
                }
            }

            ["result", "data", "content"]
                .into_iter()
                .filter_map(|key| object.get(key))
                .find_map(mcp_server_name)
        }
        _ => None,
    }
}

pub(crate) fn format_mcp_detail_content(content: &str) -> String {
    let content = content.trim();
    serde_json::from_str::<Value>(content)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| content.to_string())
}
