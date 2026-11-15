use super::error_style::subtle_card_shadow;
use crate::presentation::{bold_font, is_dark_theme};
use iced::widget::{column, container, text};
use iced::{Background, Border, Element, Length, Theme};

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
pub fn extract_url_from_error_message(message: &str) -> Option<String> {
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

/// 已校验的 API 错误；无效载荷在宿主查询编辑器之前拒绝。
pub struct AssistantApiError {
    message: String,
    retryable: Option<bool>,
}

pub fn parse_assistant_api_error(content: &str) -> Option<AssistantApiError> {
    let value = serde_json::from_str::<serde_json::Value>(content.trim()).ok()?;

    let name = value.get("name").and_then(|item| item.as_str())?;
    if name != "APIError" {
        return None;
    }

    let message = value.get("message").and_then(|item| item.as_str())?.trim().to_string();
    if message.is_empty() {
        return None;
    }

    let retryable = value.get("is_retryable").and_then(|item| item.as_bool());
    Some(AssistantApiError { message, retryable })
}

/// 组合领域错误详情与宿主提供的可选择编辑器。
pub fn assistant_api_error_view<'a, M: 'a>(
    i18n: desktop_i18n::I18n,
    editor: Option<Element<'a, M>>,
    error: AssistantApiError,
) -> Element<'a, M> {
    let AssistantApiError { message, retryable } = error;
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

    if let Some(view) = editor {
        detail_col = detail_col.push(view);
    }

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
        .into()
}
