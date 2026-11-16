//! 桌面编辑器注入与领域错误卡片适配。
use super::super::tool_text_support::chat_text_font_name;
use super::super::tools::{ToolTextTarget, tool_text_editor};
use super::styles::MESSAGE_TEXT_SIZE;
use super::text::message_editor_body;
use crate::app::{App, Message};
use iced::Element;
pub(super) fn assistant_api_error_view<'a>(
    app: &'a App,
    msg_idx: usize,
    text_idx: Option<usize>,
    content: &str,
) -> Option<Element<'a, Message>> {
    // Invalid payloads must not touch the editor path.
    let error =
        desktop_chat_view::message_view::assistant_error::parse_assistant_api_error(content)?;
    let editor = if let Some(text_idx) = text_idx {
        tool_text_editor(
            app,
            ToolTextTarget::SpecialMessageText { msg_idx, text_idx },
            chat_text_font_name(),
            MESSAGE_TEXT_SIZE,
            false,
            true,
        )
        .map(|view| iced::widget::container(view).width(iced::Length::Fill).into())
    } else {
        message_editor_body(
            app,
            msg_idx,
            app.theme().extended_palette().danger.base.color.scale_alpha(0.95),
        )
    };
    Some(desktop_chat_view::message_view::assistant_error::assistant_api_error_view(
        crate::app::i18n::I18n::new(app.desktop_locale),
        editor,
        error,
    ))
}
