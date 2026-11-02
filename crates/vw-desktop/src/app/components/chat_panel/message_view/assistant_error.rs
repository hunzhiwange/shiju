//! 助手 API 错误卡片视图。

use iced::widget::{column, container, text};
use iced::{Background, Border, Element, Length, Theme};

use crate::app::{App, Message};

use super::super::tool_text_support::chat_text_font_name;
use super::super::tools::{ToolTextTarget, tool_text_editor};
use super::super::utils::bold_font;
use super::styles::{MESSAGE_TEXT_SIZE, is_dark_theme, subtle_card_shadow};
use super::text::message_editor_body;

/// 从错误消息中提取 URL
///
/// 从包含错误信息的文本中提取 HTTP/HTTPS URL。
/// 用于在显示 API 错误时提供可点击的接口地址。
///
/// # 参数
/// - `message`: 可能包含 URL 的错误消息
///
/// # 返回值
/// - `Some(url)`: 找到 URL 时返回完整的 URL 字符串
/// - `None`: 未找到 URL
pub(super) fn extract_url_from_error_message(message: &str) -> Option<String> {
    let start = ["https://", "http://"].iter().filter_map(|prefix| message.find(prefix)).min()?;

    let tail = &message[start..];
    let end = tail
        .char_indices()
        .find_map(
            |(idx, ch)| {
                if ch.is_whitespace() || ch == ')' || ch == '"' { Some(idx) } else { None }
            },
        )
        .unwrap_or(tail.len());

    let url = tail[..end].trim_matches(|ch| ch == '(' || ch == ')' || ch == '"');

    if url.is_empty() { None } else { Some(url.to_string()) }
}

/// 渲染助手 API 错误视图
///
/// 当助手消息内容是 JSON 格式的 API 错误时，渲染一个错误提示卡片。
/// 包含错误消息、接口地址和重试提示。
///
/// # 参数
/// - `content`: 消息内容（应为 JSON 字符串）
///
/// # 返回值
/// - `Some(element)`: 内容为有效的 API 错误时返回错误视图元素
/// - `None`: 内容不是 API 错误格式
///
/// # 错误格式要求
/// JSON 必须包含：
/// - `name`: "APIError"
/// - `message`: 错误描述文本
/// - `is_retryable`: 可选，指示是否可重试
pub(super) fn assistant_api_error_view<'a>(
    app: &'a App,
    msg_idx: usize,
    text_idx: Option<usize>,
    content: &str,
) -> Option<Element<'a, Message>> {
    let value = serde_json::from_str::<serde_json::Value>(content.trim()).ok()?;

    let name = value.get("name").and_then(|item| item.as_str())?;
    if name != "APIError" {
        return None;
    }

    let message = value.get("message").and_then(|item| item.as_str())?.trim().to_string();
    if message.is_empty() {
        return None;
    }

    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let retryable = value.get("is_retryable").and_then(|item| item.as_bool());
    let retry_hint = match retryable {
        Some(true) => "系统会自动重试，请稍候。",
        Some(false) => "该请求不可自动重试，请检查网络或 API 配置后重试。",
        None => "请检查网络或 API 配置后重试。",
    };

    let mut detail_col = column![
        text(i18n.__("请求失败")).size(14).font(bold_font()).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.extended_palette().danger.base.color) }
        }),
        text(message.clone()).size(14).style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.extended_palette().danger.base.color.scale_alpha(0.95)),
        }),
    ]
    .spacing(6);

    if let Some(url) = extract_url_from_error_message(&message) {
        detail_col = detail_col.push(
            text(i18n.format("接口地址: %s", &[url.into()]).expect("valid API address template"))
                .size(14)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.85)),
                }),
        );
    }

    detail_col = detail_col.push(text(i18n.__(retry_hint)).size(14).style(|theme: &Theme| {
        iced::widget::text::Style {
            color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.90)),
        }
    }));

    if let Some(text_idx) = text_idx {
        if let Some(view) = tool_text_editor(
            app,
            ToolTextTarget::SpecialMessageText { msg_idx, text_idx },
            chat_text_font_name(),
            MESSAGE_TEXT_SIZE,
            false,
            true,
        ) {
            detail_col = detail_col.push(container(view).width(Length::Fill));
        }
    } else if let Some(view) = message_editor_body(
        app,
        msg_idx,
        app.theme().extended_palette().danger.base.color.scale_alpha(0.95),
    ) {
        detail_col = detail_col.push(view);
    }

    Some(
        container(detail_col)
            .padding([12, 14])
            .width(Length::Fill)
            .style(|theme: &Theme| {
                let ext = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(Background::Color(
                        ext.danger.base.color.scale_alpha(if is_dark_theme(theme) {
                            0.10
                        } else {
                            0.07
                        }),
                    )),
                    border: Border {
                        width: 1.0,
                        color: ext.danger.base.color.scale_alpha(if is_dark_theme(theme) {
                            0.34
                        } else {
                            0.26
                        }),
                        radius: 14.0.into(),
                    },
                    shadow: subtle_card_shadow(theme),
                    ..Default::default()
                }
            })
            .into(),
    )
}
