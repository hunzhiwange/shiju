//! 渲染高级工具结果视图。
//! 该视图把复杂工具输出折叠成摘要、元数据和可展开正文，保持聊天流可扫描。

use iced::widget::{Space, button, column, container, mouse_area, scrollable, text};
use iced::{Background, Border, Element, Length, Padding, Theme};
use serde_json::{Map, Value};

use crate::app::assets::Icon;
use crate::app::components::widgets::RightClickArea;
use crate::app::state::{
    AdvancedToolSurfaceSpec, AdvancedToolSurfaceState, explicit_advanced_tool_surface_spec,
};
use crate::app::{App, Message, message};

use super::tool_meta::{tool_activity_header, tool_header_label, tool_visual_state};
use super::tool_parse::{
    tool_error_text, tool_input, tool_output_text, tool_render_hint_metadata, tool_result_data,
    tool_status, tool_summary_text,
};
use super::{
    ToolTextTarget, canonical_tool_name, selected_chat_text_for_target, tool_permission_state,
    tool_text_editor,
};
use crate::app::components::chat_panel::utils::{
    chat_context_target_key, chat_scroll_direction, chat_secondary_muted_text_color,
    eye_icon_button_style, eye_icon_svg_style, icon_svg, simplified_block_style,
    simplified_code_block_style, truncate_chars, truncate_lines_middle,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AgentToolCardState {
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
}

fn agent_tool_card_state(status: &str) -> AgentToolCardState {
    match status {
        "running" => AgentToolCardState::Running,
        "waiting" => AgentToolCardState::Waiting,
        "error" | "failed" | "denied" => AgentToolCardState::Failed,
        "cancelled" => AgentToolCardState::Cancelled,
        _ => AgentToolCardState::Completed,
    }
}

pub(super) fn agent_tool_detail_message(msg_idx: usize, tool_idx: usize, visible: &str) -> Message {
    Message::Chat(message::ChatMessage::OpenToolDetail(msg_idx, tool_idx, visible.to_string()))
}

pub(super) fn nested_string<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str().map(str::trim).filter(|value| !value.is_empty())
}

pub(super) fn advanced_base_title(
    i18n: crate::app::i18n::I18n,
    tool_name: &str,
    explicit_spec: Option<AdvancedToolSurfaceSpec>,
) -> String {
    if let Some(spec) = explicit_spec {
        return spec.label.to_string();
    }

    match tool_name {
        "AgentTool" | "Agent" => "AgentTool".to_string(),
        "browser" => i18n.__("浏览器").to_string(),
        "browser_open" | "open_browser_page" => i18n.__("打开页面").to_string(),
        _ => i18n.__(&tool_header_label(tool_name)).to_string(),
    }
}

pub(super) fn advanced_summary(
    i18n: crate::app::i18n::I18n,
    tool_name: &str,
    value: &Value,
) -> String {
    if let Some(summary) = tool_summary_text(value).filter(|summary| !summary.trim().is_empty()) {
        return summary;
    }

    let input = tool_input(value);
    if let Some(summary) = crate::app::components::chat_panel::tools::localized_tool_inline_summary(
        i18n, tool_name, input,
    )
    .filter(|summary| !summary.trim().is_empty())
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
                (false, true) => {
                    i18n.format("调用 %s", &[(agent).into()]).expect("valid tool display template")
                }
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
    i18n: crate::app::i18n::I18n,
    tool_name: &str,
    value: &Value,
    explicit_spec: Option<AdvancedToolSurfaceSpec>,
) -> String {
    let render_metadata = tool_render_hint_metadata(value);
    let result_data = tool_result_data(value);
    let mut parts = Vec::new();

    if let Some(spec) = explicit_spec {
        parts.push(
            i18n.format("状态: %s", &[(spec.state.label()).into()])
                .expect("valid tool display template"),
        );
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
    i18n: crate::app::i18n::I18n,
    tool_name: &str,
    value: &Value,
    explicit_spec: Option<AdvancedToolSurfaceSpec>,
) -> String {
    if let Some(spec) = explicit_spec {
        if let Some(error_text) = tool_error_text(value).filter(|text| !text.trim().is_empty()) {
            return error_text;
        }

        return match spec.state {
            AdvancedToolSurfaceState::Available => i18n
                .format("%s 已接入当前会话工具面。", &[(spec.label).into()])
                .expect("valid tool display template"),
            AdvancedToolSurfaceState::Planned => i18n
                .format(
                    "%s 当前已明确标记为 planned，桌面端先提供清晰状态，不补完整后端。",
                    &[spec.label.into()],
                )
                .expect("valid planned surface template"),
        };
    }

    if matches!(tool_name, "AgentTool" | "Agent")
        && let Some(message) = result_data_message(value)
    {
        return message;
    }

    String::new()
}

pub(super) fn advanced_success_body(
    i18n: crate::app::i18n::I18n,
    tool_name: &str,
    raw_output: &str,
    value: &Value,
) -> String {
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
            format_browser_success_body(i18n, tool_name, value)
                .unwrap_or_else(|| raw_output.to_string())
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

pub(super) fn format_browser_success_body(
    i18n: crate::app::i18n::I18n,
    tool_name: &str,
    value: &Value,
) -> Option<String> {
    let data = tool_result_data(value)?.as_object()?;
    if matches!(tool_name, "browser_open" | "open_browser_page") {
        let browser = data.get("browser").and_then(Value::as_str).map(str::trim).unwrap_or("");
        let url = data.get("url").and_then(Value::as_str).map(str::trim).unwrap_or("");
        return match (browser.is_empty(), url.is_empty()) {
            (true, true) => None,
            (false, true) => Some(
                i18n.format("已在 %s 中打开页面。", &[(browser).into()])
                    .expect("valid tool display template"),
            ),
            (true, false) => Some(
                i18n.format("已打开 %s", &[(url).into()]).expect("valid tool display template"),
            ),
            (false, false) => Some(
                i18n.format("已在 %s 中打开 %s", &[(browser).into(), (url).into()])
                    .expect("valid tool display template"),
            ),
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
        return Some(
            i18n.format("结果路径: %s", &[(path).into()]).expect("valid tool display template"),
        );
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

pub(super) fn compact_browser_success_summary(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_chars(&compact, 120).to_string()
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
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    if !is_advanced_surface_tool(tool_name) {
        return None;
    }

    let value = serde_json::from_str::<Value>(rest.trim()).ok()?;
    let explicit_spec = explicit_advanced_tool_surface_spec(tool_name);
    let status = tool_status(&value);
    let visual = tool_visual_state(status);
    let is_error = matches!(status, "error" | "failed" | "denied");
    let is_running = matches!(status, "running" | "waiting");
    let output = tool_output_text(&value).unwrap_or_default();
    let output = output.trim();
    let body_text = if is_error {
        crate::app::components::chat_panel::tools::localized_tool_permission_error_text(
            crate::app::i18n::I18n::new(app.desktop_locale),
            tool_name,
            &value,
        )
        .or_else(|| tool_error_text(&value))
        .unwrap_or_default()
    } else if matches!(tool_name, "browser" | "browser_open" | "open_browser_page") {
        advanced_success_body(i18n, tool_name, output, &value)
    } else if !output.is_empty() {
        advanced_success_body(i18n, tool_name, output, &value)
    } else {
        advanced_fallback_body(i18n, tool_name, &value, explicit_spec)
    };
    let body_text = body_text.trim().to_string();

    let permission_state = tool_permission_state(tool_name, &value);
    let base_title = advanced_base_title(i18n, tool_name, explicit_spec);
    let title = if let Some(permission_state) = permission_state {
        crate::app::components::chat_panel::tools::localized_tool_permission_title(
            crate::app::i18n::I18n::new(app.desktop_locale),
            base_title.as_str(),
            permission_state,
        )
    } else if is_running {
        i18n.format("%s 运行中", &[base_title.as_str().into()])
            .expect("valid tool display template")
    } else if is_error {
        i18n.format("%s 失败", &[base_title.as_str().into()]).expect("valid tool display template")
    } else {
        base_title.clone()
    };
    let mut summary = advanced_summary(i18n, tool_name, &value);
    let metadata_text = advanced_metadata_text(i18n, tool_name, &value, explicit_spec);
    let collapse_success_body =
        matches!(tool_name, "browser" | "browser_open" | "open_browser_page")
            && !is_running
            && !is_error;
    if collapse_success_body && summary.trim().is_empty() && !body_text.is_empty() {
        summary = compact_browser_success_summary(&body_text);
    }
    if collapse_success_body && summary.trim().is_empty() {
        summary = if metadata_text.trim().is_empty() {
            crate::app::i18n::I18n::new(app.desktop_locale).__("查看详情").to_string()
        } else {
            truncate_chars(metadata_text.trim(), 80).to_string()
        };
    }

    if !is_running && summary.trim().is_empty() && body_text.is_empty() && metadata_text.is_empty()
    {
        return None;
    }

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

    if matches!(tool_name, "AgentTool" | "Agent") {
        let state = agent_tool_card_state(status);
        if summary.trim().is_empty() && !body_text.is_empty() {
            summary = truncate_chars(body_text.as_str(), 80).to_string();
        }
        if summary.trim().is_empty() {
            summary = match state {
                AgentToolCardState::Running => {
                    crate::app::i18n::I18n::new(app.desktop_locale).__("子代理正在执行").to_string()
                }
                AgentToolCardState::Waiting => {
                    crate::app::i18n::I18n::new(app.desktop_locale).__("正在等待子代理").to_string()
                }
                AgentToolCardState::Completed => {
                    crate::app::i18n::I18n::new(app.desktop_locale).__("子代理执行完成").to_string()
                }
                AgentToolCardState::Failed => {
                    crate::app::i18n::I18n::new(app.desktop_locale).__("子代理执行失败").to_string()
                }
                AgentToolCardState::Cancelled => {
                    crate::app::i18n::I18n::new(app.desktop_locale).__("子代理已取消").to_string()
                }
            };
        }

        let detail_message = agent_tool_detail_message(msg_idx, tool_idx, visible);

        let card: Element<'a, Message> = mouse_area(
            container(tool_activity_header(
                tool_name,
                base_title,
                summary,
                visual,
                detail_btn.into(),
            ))
            .padding(Padding { top: 4.0, right: 6.0, bottom: 4.0, left: 0.0 })
            .width(Length::Fill),
        )
        .on_press(detail_message.clone())
        .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
        .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave))
        .into();

        return Some(
            RightClickArea::new(card, Box::new(move |_| detail_message.clone()))
                .use_global_position()
                .preserve_on_right_click()
                .into(),
        );
    }

    // 浏览器成功态：单行摘要 + 右侧详情入口，完整结果进弹窗（对齐 skill/mcp）。
    if collapse_success_body {
        return Some(
            mouse_area(
                container(tool_activity_header(
                    tool_name,
                    title,
                    truncate_chars(summary.trim(), 80),
                    visual,
                    detail_btn.into(),
                ))
                .padding(Padding { top: 4.0, right: 6.0, bottom: 4.0, left: 0.0 })
                .width(Length::Fill),
            )
            .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
            .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave))
            .into(),
        );
    }

    let key = ((msg_idx as u64) << 32) | (tool_idx as u64);
    let is_hovered = app.chat_tool_hovered_idx == Some(key);
    let context_key = chat_context_target_key(msg_idx, Some(tool_idx));
    let context_text = selected_chat_text_for_target(app, context_key)
        .unwrap_or_else(|| if body_text.is_empty() { summary.clone() } else { body_text.clone() });

    let detail_slot: Element<'a, Message> =
        if is_hovered { detail_btn.into() } else { Space::new().width(Length::Fixed(22.0)).into() };

    let head_row: Element<'a, Message> =
        tool_activity_header(tool_name, title, summary.clone(), visual, detail_slot).into();

    let mut content = column![container(head_row).width(Length::Fill)].spacing(8);
    if !metadata_text.is_empty() {
        content = content.push(text(metadata_text).size(12).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(chat_secondary_muted_text_color(theme)) }
        }));
    }

    let body: Element<'a, Message> = if is_running {
        container(
            text(
                i18n.format("%s中…", &[base_title.as_str().into()])
                    .expect("valid tool display template"),
            )
            .size(14)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.85)),
            }),
        )
        .width(Length::Fill)
        .into()
    } else if body_text.is_empty() {
        container(
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("没有可展示的高级工具结果"))
                .size(14)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(chat_secondary_muted_text_color(theme)),
                }),
        )
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
        .use_global_position()
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
        .use_global_position()
        .preserve_on_right_click()
        .into()
    };

    content = content.push(body);

    let content: Element<'a, Message> =
        container(content).padding([2, 6]).width(Length::Fill).style(simplified_block_style).into();

    Some(content)
}
