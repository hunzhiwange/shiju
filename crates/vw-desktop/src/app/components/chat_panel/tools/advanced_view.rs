//! 渲染高级工具结果视图。
//! 该视图把复杂工具输出折叠成摘要、元数据和可展开正文，保持聊天流可扫描。

use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Element, Length, Theme};
use serde_json::{Map, Value};

use crate::app::assets::Icon;
use crate::app::components::overlays::PointBelowOverlay;
use crate::app::components::widgets::RightClickArea;
use crate::app::state::{
    AdvancedToolSurfaceSpec, AdvancedToolSurfaceState, explicit_advanced_tool_surface_spec,
};
use crate::app::{App, Message, message};

use super::tool_meta::{tool_header_label, tool_header_title, tool_inline_summary};
use super::tool_parse::{
    tool_error_text, tool_input, tool_output_text, tool_render_hint_metadata, tool_result_data,
    tool_status, tool_summary_text,
};
use super::{
    ToolTextTarget, canonical_tool_name, selected_chat_text_for_target, tool_permission_error_text,
    tool_permission_state, tool_permission_title, tool_text_editor,
};
use crate::app::components::chat_panel::utils::{
    chat_context_menu, chat_context_target_key, chat_scroll_direction,
    chat_secondary_muted_text_color, eye_icon_button_style, eye_icon_svg_style, icon_svg,
    simplified_block_style, simplified_code_block_style, truncate_chars, truncate_lines_middle,
};

pub(super) fn is_advanced_surface_tool(tool_name: &str) -> bool {
    matches!(tool_name, "AgentTool" | "Agent" | "browser" | "browser_open" | "open_browser_page")
        || explicit_advanced_tool_surface_spec(tool_name).is_some()
}

fn string_from_map(map: Option<&Map<String, Value>>, key: &str) -> Option<String> {
    map.and_then(|items| items.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(super) fn nested_string<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str().map(str::trim).filter(|value| !value.is_empty())
}

pub(super) fn advanced_base_title(
    tool_name: &str,
    explicit_spec: Option<AdvancedToolSurfaceSpec>,
) -> String {
    if let Some(spec) = explicit_spec {
        return spec.label.to_string();
    }

    match tool_name {
        "AgentTool" | "Agent" => "AgentTool".to_string(),
        "browser" => "浏览器".to_string(),
        "browser_open" | "open_browser_page" => "打开页面".to_string(),
        _ => tool_header_label(tool_name),
    }
}

pub(super) fn advanced_summary(tool_name: &str, value: &Value) -> String {
    if let Some(summary) = tool_summary_text(value).filter(|summary| !summary.trim().is_empty()) {
        return summary;
    }

    let input = tool_input(value);
    if let Some(summary) =
        tool_inline_summary(tool_name, input).filter(|summary| !summary.trim().is_empty())
    {
        return summary;
    }

    let Ok(input_json) = serde_json::from_str::<Value>(input.trim()) else {
        return String::new();
    };

    match tool_name {
        "AgentTool" | "Agent" => {
            let agent = input_json
                .get("agent")
                .or_else(|| input_json.get("subagent_type"))
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            let prompt = input_json
                .get("prompt")
                .or_else(|| input_json.get("task"))
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            match (agent.is_empty(), prompt.is_empty()) {
                (true, true) => String::new(),
                (false, true) => format!("调用 {}", agent),
                (true, false) => truncate_chars(prompt, 80).to_string(),
                (false, false) => format!("{} · {}", agent, truncate_chars(prompt, 56)),
            }
        }
        "browser" => {
            let action = input_json
                .get("action")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("browser");
            let url = input_json.get("url").and_then(Value::as_str).map(str::trim).unwrap_or("");
            let selector =
                input_json.get("selector").and_then(Value::as_str).map(str::trim).unwrap_or("");

            if !url.is_empty() {
                format!("{} · {}", action, truncate_chars(url, 56))
            } else if !selector.is_empty() {
                format!("{} · {}", action, truncate_chars(selector, 56))
            } else {
                action.to_string()
            }
        }
        _ => String::new(),
    }
}

pub(super) fn advanced_metadata_text(
    tool_name: &str,
    value: &Value,
    explicit_spec: Option<AdvancedToolSurfaceSpec>,
) -> String {
    let render_metadata = tool_render_hint_metadata(value);
    let result_data = tool_result_data(value);
    let mut parts = Vec::new();

    if let Some(spec) = explicit_spec {
        parts.push(format!("状态: {}", spec.state.label()));
    }

    match tool_name {
        "AgentTool" | "Agent" => {
            if let Some(agent) = string_from_map(result_data.and_then(Value::as_object), "agent") {
                parts.push(agent);
            }
            if let Some(session_id) =
                string_from_map(result_data.and_then(Value::as_object), "session_id")
            {
                parts.push(format!("session {}", session_id));
            }
        }
        "browser" => {
            if let Some(action) = string_from_map(render_metadata, "action") {
                parts.push(action);
            }
            if let Some(backend) = string_from_map(render_metadata, "backend") {
                parts.push(backend);
            }
            if let Some(url) = result_data.and_then(|data| nested_string(data, &["result", "url"]))
            {
                parts.push(truncate_chars(url, 72).to_string());
            } else if let Some(title) =
                result_data.and_then(|data| nested_string(data, &["result", "title"]))
            {
                parts.push(truncate_chars(title, 72).to_string());
            }
        }
        "browser_open" | "open_browser_page" => {
            if let Some(browser) = string_from_map(render_metadata, "browser") {
                parts.push(browser);
            }
            if let Some(url) = string_from_map(render_metadata, "url").or_else(|| {
                result_data
                    .and_then(Value::as_object)
                    .and_then(|data| string_from_map(Some(data), "url"))
            }) {
                parts.push(truncate_chars(&url, 72).to_string());
            }
        }
        _ => {}
    }

    parts.join(" · ")
}

pub(super) fn advanced_fallback_body(
    tool_name: &str,
    value: &Value,
    explicit_spec: Option<AdvancedToolSurfaceSpec>,
) -> String {
    if let Some(spec) = explicit_spec {
        if let Some(error_text) = tool_error_text(value).filter(|text| !text.trim().is_empty()) {
            return error_text;
        }

        return match spec.state {
            AdvancedToolSurfaceState::Available => {
                format!("{} 已接入当前会话工具面。", spec.label)
            }
            AdvancedToolSurfaceState::Planned => {
                format!(
                    "{} 当前已明确标记为 planned，桌面端先提供清晰状态，不补完整后端。",
                    spec.label
                )
            }
        };
    }

    if matches!(tool_name, "AgentTool" | "Agent")
        && let Some(message) = result_data_message(value)
    {
        return message;
    }

    String::new()
}

pub(super) fn advanced_success_body(tool_name: &str, raw_output: &str, value: &Value) -> String {
    match tool_name {
        "AgentTool" | "Agent" => {
            if let Ok(parsed) = serde_json::from_str::<Value>(raw_output)
                && let Some(message) = parsed.get("message").and_then(Value::as_str)
            {
                let message = message.trim();
                if !message.is_empty() {
                    return message.to_string();
                }
            }
            if let Some(message) = result_data_message(value) {
                return message;
            }
            raw_output.to_string()
        }
        "browser" | "browser_open" | "open_browser_page" => {
            format_browser_success_body(tool_name, value).unwrap_or_else(|| raw_output.to_string())
        }
        "tool_search" => format_tool_search_body(value).unwrap_or_else(|| raw_output.to_string()),
        "verify_plan_execution" => {
            format_verify_plan_execution_body(value).unwrap_or_else(|| raw_output.to_string())
        }
        _ => raw_output.to_string(),
    }
}

pub(super) fn result_data_message(value: &Value) -> Option<String> {
    tool_result_data(value)
        .and_then(Value::as_object)
        .and_then(|data| data.get("message"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(ToString::to_string)
}

pub(super) fn format_tool_search_body(value: &Value) -> Option<String> {
    let data = tool_result_data(value)?.as_object()?;
    let items = data.get("items")?.as_array()?;
    if items.is_empty() {
        return Some("未找到匹配工具。".to_string());
    }

    let mut lines = Vec::new();
    for item in items.iter().take(6) {
        let object = item.as_object()?;
        let label = object
            .get("display_name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .or_else(|| object.get("id").and_then(Value::as_str).map(str::trim))
            .unwrap_or("unknown");
        let reason = object
            .get("reason")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or("matched");
        lines.push(format!("- {label}: {reason}"));
    }

    let total = data.get("count").and_then(Value::as_u64).unwrap_or(items.len() as u64) as usize;
    if total > lines.len() {
        lines.push(format!("... 还有 {} 个结果", total.saturating_sub(lines.len())));
    }
    Some(lines.join("\n"))
}

pub(super) fn format_verify_plan_execution_body(value: &Value) -> Option<String> {
    let data = tool_result_data(value)?.as_object()?;
    let ready = data.get("ready").and_then(Value::as_bool).unwrap_or(false);
    let todo_count = data.get("todo_count").and_then(Value::as_u64).unwrap_or(0);
    let pending_count = data.get("pending_count").and_then(Value::as_u64).unwrap_or(0);
    let in_progress_count = data.get("in_progress_count").and_then(Value::as_u64).unwrap_or(0);

    if ready {
        return Some(format!(
            "已满足执行条件。\nTodo: {}，待处理: {}，进行中: {}",
            todo_count, pending_count, in_progress_count
        ));
    }

    let blockers = data
        .get("blockers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| format!("- {text}"))
        .collect::<Vec<_>>();
    if blockers.is_empty() {
        return None;
    }
    Some(blockers.join("\n"))
}

pub(super) fn format_browser_success_body(tool_name: &str, value: &Value) -> Option<String> {
    let data = tool_result_data(value)?.as_object()?;
    if matches!(tool_name, "browser_open" | "open_browser_page") {
        let browser = data.get("browser").and_then(Value::as_str).map(str::trim).unwrap_or("");
        let url = data.get("url").and_then(Value::as_str).map(str::trim).unwrap_or("");
        return match (browser.is_empty(), url.is_empty()) {
            (true, true) => None,
            (false, true) => Some(format!("已在 {browser} 中打开页面。")),
            (true, false) => Some(format!("已打开 {url}")),
            (false, false) => Some(format!("已在 {browser} 中打开 {url}")),
        };
    }

    let result = data.get("result")?;
    if let Some(message) = nested_string(result, &["message"]) {
        return Some(message.to_string());
    }
    if let Some(text) = nested_string(result, &["text"]) {
        return Some(text.to_string());
    }
    if let Some(path) = nested_string(result, &["path"]) {
        return Some(format!("结果路径: {path}"));
    }

    let title = nested_string(result, &["title"]).unwrap_or("");
    let url = nested_string(result, &["url"]).unwrap_or("");
    if !title.is_empty() && !url.is_empty() {
        return Some(format!("{title}\n{url}"));
    }
    if !title.is_empty() {
        return Some(title.to_string());
    }
    if !url.is_empty() {
        return Some(url.to_string());
    }

    serde_json::to_string_pretty(result).ok()
}

/// 执行 tool_advanced_view 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn tool_advanced_view<'a>(
    app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
) -> Option<Element<'a, Message>> {
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    if !is_advanced_surface_tool(tool_name) {
        return None;
    }

    let value = serde_json::from_str::<Value>(rest.trim()).ok()?;
    let explicit_spec = explicit_advanced_tool_surface_spec(tool_name);
    let status = tool_status(&value);
    let is_error = matches!(status, "error" | "denied");
    let is_running = status == "running";
    let output = tool_output_text(&value).unwrap_or_default();
    let output = output.trim();
    let body_text = if is_error {
        tool_permission_error_text(tool_name, &value)
            .or_else(|| tool_error_text(&value))
            .unwrap_or_default()
    } else if !output.is_empty() {
        advanced_success_body(tool_name, output, &value)
    } else {
        advanced_fallback_body(tool_name, &value, explicit_spec)
    };
    let body_text = body_text.trim().to_string();

    let permission_state = tool_permission_state(tool_name, &value);
    let base_title = advanced_base_title(tool_name, explicit_spec);
    let title = if let Some(permission_state) = permission_state {
        tool_permission_title(base_title.as_str(), permission_state)
    } else if is_running {
        format!("{} 运行中", base_title)
    } else if is_error {
        format!("{} 失败", base_title)
    } else {
        base_title.clone()
    };
    let summary = advanced_summary(tool_name, &value);
    let metadata_text = advanced_metadata_text(tool_name, &value, explicit_spec);

    if !is_running && summary.trim().is_empty() && body_text.is_empty() && metadata_text.is_empty()
    {
        return None;
    }

    let key = ((msg_idx as u64) << 32) | (tool_idx as u64);
    let is_hovered = app.chat_tool_hovered_idx == Some(key);
    let context_key = chat_context_target_key(msg_idx, Some(tool_idx));
    let context_menu_open = app.chat_context_menu_target == Some(context_key);
    let context_menu_anchor = app.chat_context_menu_pos.unwrap_or((12.0, 26.0));
    let context_text = selected_chat_text_for_target(app, context_key)
        .unwrap_or_else(|| if body_text.is_empty() { summary.clone() } else { body_text.clone() });

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

    let mut head_left =
        row![tool_header_title(tool_name, title, is_error)].spacing(10).align_y(Alignment::Center);
    if !summary.trim().is_empty() {
        head_left = head_left.push(text(summary.clone()).size(13).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(chat_secondary_muted_text_color(theme)) }
        }));
    }

    let head_row: Element<'a, Message> =
        row![head_left, container(Space::new()).width(Length::Fill), detail_slot]
            .align_y(Alignment::Center)
            .into();

    let mut content = column![container(head_row).width(Length::Fill)].spacing(8);
    if !metadata_text.is_empty() {
        content = content.push(text(metadata_text).size(12).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(chat_secondary_muted_text_color(theme)) }
        }));
    }

    let body: Element<'a, Message> = if is_running {
        container(text(format!("{}中…", base_title)).size(14).style(|theme: &Theme| {
            iced::widget::text::Style {
                color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.85)),
            }
        }))
        .width(Length::Fill)
        .into()
    } else if body_text.is_empty() {
        container(text("没有可展示的高级工具结果").size(14).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(chat_secondary_muted_text_color(theme)) }
        }))
        .padding([10, 12])
        .width(Length::Fill)
        .style(simplified_block_style)
        .into()
    } else if is_error {
        let err_body = tool_text_editor(
            app,
            ToolTextTarget::ToolCardText { msg_idx, tool_idx, text_idx: 0 },
            "Noto Sans CJK SC",
            14.0,
            false,
            true,
        )
        .unwrap_or_else(|| {
            container(text(truncate_chars(body_text.as_str(), 200)).size(14).style(
                |theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().danger.base.color.scale_alpha(0.95)),
                },
            ))
            .width(Length::Fill)
            .into()
        });

        RightClickArea::new(
            container(err_body)
                .padding([10, 12])
                .width(Length::Fill)
                .style(|theme: &Theme| {
                    let ext = theme.extended_palette();
                    iced::widget::container::Style {
                        background: Some(Background::Color(
                            ext.danger.base.color.scale_alpha(0.07),
                        )),
                        border: Border {
                            width: 1.0,
                            color: ext.danger.base.color.scale_alpha(0.30),
                            radius: 14.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
            Box::new(move |point| {
                Message::Chat(message::ChatMessage::OpenMessageContextMenu {
                    target: context_key,
                    x: point.x,
                    y: point.y,
                    text: context_text.clone(),
                })
            }),
        )
        .preserve_on_right_click()
        .into()
    } else {
        let preview = truncate_lines_middle(body_text.as_str(), 120, 600);
        let code = tool_text_editor(
            app,
            ToolTextTarget::ToolCardText { msg_idx, tool_idx, text_idx: 0 },
            "JetBrains Mono",
            14.0,
            false,
            false,
        )
        .unwrap_or_else(|| {
            text(preview).size(14).font(iced::Font::with_name("JetBrains Mono")).into()
        });
        let body: Element<'a, Message> = scrollable(
            container(code)
                .width(Length::Fill)
                .padding([10, 12])
                .style(simplified_code_block_style),
        )
        .direction(chat_scroll_direction())
        .height(Length::Fixed(220.0))
        .into();

        RightClickArea::new(
            body,
            Box::new(move |point| {
                Message::Chat(message::ChatMessage::OpenMessageContextMenu {
                    target: context_key,
                    x: point.x,
                    y: point.y,
                    text: context_text.clone(),
                })
            }),
        )
        .preserve_on_right_click()
        .into()
    };

    content = content.push(body);

    let content: Element<'a, Message> =
        container(content).padding([2, 6]).width(Length::Fill).style(simplified_block_style).into();

    Some(if let Some(menu) = chat_context_menu(context_menu_open) {
        PointBelowOverlay::new(content, menu)
            .show(true)
            .anchor(iced::Point::new(context_menu_anchor.0, context_menu_anchor.1))
            .gap(0.0)
            .on_close(Message::Chat(message::ChatMessage::CloseMessageContextMenu))
            .into()
    } else {
        content
    })
}
