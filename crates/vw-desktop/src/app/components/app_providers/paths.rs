use crate::app::Message;
use super::list::Help;
use iced::Element;
pub(super) fn overlay<'a>(form: &'a desktop_provider_apps::state::paths::Form, help: Help<'_>, base: Element<'a, Message>) -> Element<'a, Message> {
    let card = desktop_provider_apps::components::paths::card(form, help.provider_help()).map(super::edit::map);
    let close = if form.saving { Message::None } else { crate::app::message::app_providers::paths::message(crate::app::message::app_providers::paths::Event::Close) };
    super::super::system_settings_common::settings_modal_overlay(Some(base), close, card)
}
