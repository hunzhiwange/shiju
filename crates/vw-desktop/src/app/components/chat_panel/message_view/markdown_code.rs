use iced::Element;
use crate::app::{App, Message};
use crate::app::i18n::I18n;
use super::super::utils::is_recent_copy;
use super::parse::hash_chat_content;
pub(super) fn code_block_view<'a>(app: &App, language: Option<&'a str>, code: &'a str, body: Element<'a, Message>) -> Element<'a, Message> {
    let copied = app.last_copied_code_hash.is_some() && is_recent_copy(app, hash_chat_content(code));
    desktop_chat_view::message_view::markdown_code::code_block_view(I18n::new(app.desktop_locale), copied, move || Message::CopyCode(code.to_owned()), language, code, body)
}
