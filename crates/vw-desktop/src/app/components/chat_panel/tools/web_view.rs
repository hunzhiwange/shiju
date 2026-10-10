//! Web 工具结果视图。
//!
//! 本模块渲染搜索、抓取和浏览类工具结果，并从 JSON 输出中提取 URL、标题和统计信息。

use iced::widget::{Space, button, column, container, scrollable, text};
/// 重新导出 use iced::{Alignment, Background, Border, Element, Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Background, Border, Element, Length, Theme};

/// 重新导出 use crate::app::assets::Icon，让上层模块通过稳定路径访问。
use crate::app::assets::Icon;
/// 重新导出 use crate::app::components::widgets::RightClickArea，让上层模块通过稳定路径访问。
use crate::app::components::widgets::RightClickArea;
/// 重新导出 use crate::app::{App, Message, message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message, message};

/// 共享工具活动头部、标题与摘要语义。
use super::tool_meta::{
    tool_activity_header, tool_header_label, tool_inline_summary, tool_visual_state,
};
/// 重新导出 use super::tool_parse::{tool_error_text, tool_input, tool_output_text, tool_status, tool_summary_text}，让上层模块通过稳定路径访问。
use super::tool_parse::{
    tool_error_text, tool_input, tool_output_text, tool_status, tool_summary_text,
};
/// 重新导出 use super::{，让上层模块通过稳定路径访问。
use super::{
    ToolPermissionState, ToolTextTarget, canonical_tool_name, selected_chat_text_for_target,
    tool_permission_error_text, tool_permission_state, tool_permission_title, tool_text_editor,
};
/// 重新导出 use crate::app::components::chat_panel::utils::{，让上层模块通过稳定路径访问。
use crate::app::components::chat_panel::utils::{
    chat_context_target_key, chat_scroll_direction, chat_secondary_muted_text_color,
    eye_icon_button_style, eye_icon_svg_style, icon_svg, simplified_block_style,
    simplified_code_block_style, truncate_chars, truncate_lines_middle,
};

/// 处理 tool web view 对应的局部职责。
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
pub fn tool_web_view<'a>(
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
    if !matches!(tool_name, "web_fetch" | "fetch_webpage" | "http_request" | "web_search") {
        return None;
    }

    let value = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    let status = tool_status(&value);
    let visual = tool_visual_state(status);
    let is_error = matches!(status, "error" | "failed" | "denied");
    let is_running = status == "running";
    let output = tool_output_text(&value).unwrap_or_default();
    let output = output.trim();
    let body_text = if is_error {
        tool_permission_error_text(tool_name, &value)
            .or_else(|| tool_error_text(&value))
            .or_else(|| (!output.is_empty()).then(|| output.to_string()))
            .unwrap_or_default()
    } else {
        output.to_string()
    };
    let body_text = body_text.trim().to_string();

    let permission_state = tool_permission_state(tool_name, &value);
    let base_title = tool_header_label(tool_name);
    let title = web_tool_title(base_title.as_str(), permission_state, is_running, is_error);
    let summary = tool_summary_text(&value)
        .or_else(|| tool_inline_summary(tool_name, tool_input(&value)))
        .unwrap_or_default();
    let metadata_text = web_metadata_text(tool_name, &value);

    if !is_running && summary.trim().is_empty() && body_text.is_empty() && metadata_text.is_empty()
    {
        return None;
    }

    if matches!(tool_name, "web_fetch" | "fetch_webpage" | "http_request") {
        let header_summary = if metadata_text.is_empty() { summary } else { metadata_text.clone() };
        let content = tool_activity_header(
            tool_name,
            title,
            header_summary,
            visual,
            Space::new().width(Length::Shrink).into(),
        );

        return Some(
            button(container(content).width(Length::Fill).padding([2, 6]))
                .width(Length::Fill)
                .padding(0)
                .style(|theme: &Theme, status| eye_icon_button_style(theme, status))
                .on_press(Message::Chat(message::ChatMessage::OpenToolDetail(
                    msg_idx,
                    tool_idx,
                    visible.to_string(),
                )))
                .into(),
        );
    }

    let key = ((msg_idx as u64) << 32) | (tool_idx as u64);
    let is_hovered = app.chat_tool_hovered_idx == Some(key);
    let context_key = chat_context_target_key(msg_idx, Some(tool_idx));
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
    let detail_slot: Element<'a, Message> = if is_hovered {
        detail_btn.into()
    } else {
        // Space 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Space::new().width(Length::Fixed(22.0)).into()
    };

    let head_row: Element<'a, Message> =
        tool_activity_header(tool_name, title, summary.clone(), visual, detail_slot).into();

    let mut content = column![container(head_row).width(Length::Fill)].spacing(8);
    if !metadata_text.is_empty() {
        content = content.push(text(metadata_text).size(12).style(|theme: &Theme| {
            // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            iced::widget::text::Style {
                // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                color: Some(chat_secondary_muted_text_color(theme)),
            }
        }));
    }

    let body: Element<'a, Message> = if is_running {
        container(text(format!("{}中…", base_title)).size(14).style(|theme: &Theme| {
            // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            iced::widget::text::Style {
                // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.85)),
            }
        }))
        .width(Length::Fill)
        .into()
    } else if body_text.is_empty() {
        container(text("没有可展示的网页结果").size(14).style(|theme: &Theme| {
            // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            iced::widget::text::Style {
                // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                color: Some(chat_secondary_muted_text_color(theme)),
            }
        }))
        .padding([10, 12])
        .width(Length::Fill)
        .style(simplified_block_style)
        .into()
    } else if is_error {
        let err_body = tool_text_editor(
            app,
            // ToolTextTarget 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            ToolTextTarget::ToolCardText {
                msg_idx,
                tool_idx,
                // text_idx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                text_idx: 0,
            },
            "Noto Sans CJK SC",
            14.0,
            false,
            true,
        )
        .unwrap_or_else(|| {
            container(text(truncate_chars(body_text.as_str(), 200)).size(14).style(
                |theme: &Theme| iced::widget::text::Style {
                    // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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
                        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                        background: Some(Background::Color(
                            ext.danger.base.color.scale_alpha(0.07),
                        )),
                        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                        border: Border {
                            // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                            width: 1.0,
                            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                            color: ext.danger.base.color.scale_alpha(0.30),
                            // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                            radius: 14.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
            // Box 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Box::new(move |point| {
                // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Message::Chat(message::ChatMessage::OpenMessageContextMenu {
                    // target 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    target: context_key,
                    // x 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    x: point.x,
                    // y 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    y: point.y,
                    // text 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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
            // ToolTextTarget 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            ToolTextTarget::ToolCardText {
                msg_idx,
                tool_idx,
                // text_idx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                text_idx: 0,
            },
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
            // Box 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Box::new(move |point| {
                // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Message::Chat(message::ChatMessage::OpenMessageContextMenu {
                    // target 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    target: context_key,
                    // x 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    x: point.x,
                    // y 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    y: point.y,
                    // text 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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

/// 处理 web tool title 对应的局部职责。
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
pub(super) fn web_tool_title(
    base_title: &str,
    // permission_state 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    permission_state: Option<ToolPermissionState>,
    // is_running 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    is_running: bool,
    // is_error 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    is_error: bool,
) -> String {
    if is_running {
        format!("{}中", base_title)
    } else if let Some(permission_state) = permission_state {
        tool_permission_title(base_title, permission_state)
    } else if is_error {
        format!("{}失败", base_title)
    } else {
        base_title.to_string()
    }
}

/// 处理 web metadata text 对应的局部职责。
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
pub(super) fn web_metadata_text(tool_name: &str, value: &serde_json::Value) -> String {
    let mut parts = Vec::new();

    match tool_name {
        "web_fetch" | "fetch_webpage" | "http_request" => {
            if let Some(url) = web_string_field(value, &["url", "urls", "query"]) {
                parts.push(truncate_chars(url.as_str(), 96).to_string());
            }
            if let Some(provider) = web_metadata_field(value, "provider") {
                parts.push(provider);
            }
            if let Some(format) = web_metadata_field(value, "format") {
                parts.push(format!("格式 {format}"));
            }
            if web_metadata_bool(value, "truncated") {
                parts.push("已截断".to_string());
            }
        }
        "web_search" => {
            if let Some(query) = web_string_field(value, &["query"]) {
                parts.push(truncate_chars(query.as_str(), 96).to_string());
            }
            if let Some(provider) = web_metadata_field(value, "provider") {
                parts.push(provider);
            }
            if let Some(result_count) = web_metadata_number(value, "result_count") {
                parts.push(format!("{} 条结果", result_count));
            }
        }
        _ => {}
    }

    parts.join(" · ")
}

/// 处理 web metadata field 对应的局部职责。
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
pub(super) fn web_metadata_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get("renderHint")
        .and_then(|item| item.get("metadata"))
        .and_then(|item| item.get(key))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            value
                .get("render_hint")
                .and_then(|item| item.get("metadata"))
                .and_then(|item| item.get(key))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(ToString::to_string)
        })
        .or_else(|| {
            value
                .get("data")
                .and_then(|item| item.get(key))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(ToString::to_string)
        })
}

/// 处理 web metadata number 对应的局部职责。
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
pub(super) fn web_metadata_number(value: &serde_json::Value, key: &str) -> Option<u64> {
    value
        .get("renderHint")
        .and_then(|item| item.get("metadata"))
        .and_then(|item| item.get(key))
        .and_then(serde_json::Value::as_u64)
        .or_else(|| {
            value
                .get("render_hint")
                .and_then(|item| item.get("metadata"))
                .and_then(|item| item.get(key))
                .and_then(serde_json::Value::as_u64)
        })
        .or_else(|| {
            value.get("data").and_then(|item| item.get(key)).and_then(serde_json::Value::as_u64)
        })
}

/// 处理 web metadata bool 对应的局部职责。
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
pub(super) fn web_metadata_bool(value: &serde_json::Value, key: &str) -> bool {
    value
        .get("renderHint")
        .and_then(|item| item.get("metadata"))
        .and_then(|item| item.get(key))
        .and_then(serde_json::Value::as_bool)
        .or_else(|| {
            value
                .get("render_hint")
                .and_then(|item| item.get("metadata"))
                .and_then(|item| item.get(key))
                .and_then(serde_json::Value::as_bool)
        })
        .or_else(|| {
            value.get("data").and_then(|item| item.get(key)).and_then(serde_json::Value::as_bool)
        })
        .unwrap_or(false)
}

/// 处理 web string field 对应的局部职责。
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
pub(super) fn web_string_field(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if *key == "urls"
            && let Some(url) = value
                .get("input")
                .and_then(serde_json::Value::as_str)
                .and_then(|input| serde_json::from_str::<serde_json::Value>(input).ok())
                .and_then(|input| input.get("urls").and_then(serde_json::Value::as_array).cloned())
                .and_then(|items| items.first().cloned())
                .and_then(|item| item.as_str().map(ToString::to_string))
        {
            return Some(url);
        }

        if let Some(field) = value
            .get("data")
            .and_then(|item| item.get(key))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToString::to_string)
        {
            return Some(field);
        }
    }

    // 同类 Web 工具的字段名并不完全一致，按候选 key 查找能兼容旧输出。
    keys.iter().find_map(|key| {
        // serde_json 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        serde_json::from_str::<serde_json::Value>(tool_input(value).trim())
            .ok()
            .and_then(|input| input.get(*key).cloned())
            .and_then(|item| item.as_str().map(str::trim).map(ToString::to_string))
            .filter(|text| !text.is_empty())
    })
}
