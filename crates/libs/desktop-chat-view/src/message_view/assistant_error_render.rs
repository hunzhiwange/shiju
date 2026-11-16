//! 桌面编辑器注入与领域错误卡片适配。
use super::render_snapshot::MessageViewSnapshot;
use super::render_support::message_editor_body;
use super::render_support::tool_text_editor;
use super::styles::MESSAGE_TEXT_SIZE;
use desktop_chat_tools::tool_text_support::ToolTextTarget;
use desktop_chat_tools::tool_text_support::chat_text_font_name;
use iced::Element;
pub fn assistant_api_error_view<'a, M: Clone + 'static>(
    app: &MessageViewSnapshot<'a, M>,
    msg_idx: usize,
    text_idx: Option<usize>,
    content: &str,
) -> Option<Element<'a, M>> {
    // Invalid payloads must not touch the editor path.
    let error = super::assistant_error::parse_assistant_api_error(content)?;
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
    Some(super::assistant_error::assistant_api_error_view(
        desktop_i18n::I18n::new(app.desktop_locale),
        editor,
        error,
    ))
}
