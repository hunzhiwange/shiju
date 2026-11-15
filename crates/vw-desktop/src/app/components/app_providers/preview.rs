use crate::app::Message;
use super::list::Help;
use iced::Element;
pub(super) fn overlay<'a>(form: &'a desktop_provider_apps::state::preview::Form, help: Help<'_>, base: Element<'a, Message>) -> Element<'a, Message> {
    let card = desktop_provider_apps::components::preview::card(form, help.provider_help()).map(super::edit::map);
    let close = if form.save.is_some() { Message::None } else { crate::app::message::app_providers::preview::message(crate::app::message::app_providers::preview::Event::Close) };
    super::super::system_settings_common::settings_modal_overlay(Some(base), close, card)
}
