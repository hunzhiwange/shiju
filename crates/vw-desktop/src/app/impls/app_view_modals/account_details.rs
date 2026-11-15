use super::{App, Message};
use iced::Element;
pub(crate) fn with_account_details<'a>(app: &'a App, root: Element<'a, Message>) -> Element<'a, Message> {
    match desktop_account_ui::account_details::details(&app.account, app.desktop_locale) {
        Some(card) => desktop_ui_controls::system_settings_common::settings_modal_overlay(Some(root), Message::Account(crate::app::message::AccountMessage::CloseDetails), card.map(Message::Account)),
        None => root,
    }
}
