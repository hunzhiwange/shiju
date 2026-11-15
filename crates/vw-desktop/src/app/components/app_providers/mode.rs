use crate::app::Message;
use super::list::Help;
use iced::Element;
pub(super) fn overlay<'a>(state: &'a crate::app::state::app_providers::list::ListState, form: &'a desktop_provider_apps::state::mode::Form, help: Help<'_>, base: Element<'a, Message>) -> Element<'a, Message> {
    let card = desktop_provider_apps::components::mode::card(state.snapshot.as_ref(), form, help.provider_help()).map(super::edit::map);
    let close = if form.writing() { Message::None } else { crate::app::message::app_providers::mode::message(crate::app::message::app_providers::mode::Event::Close) };
    super::super::system_settings_common::settings_modal_overlay(Some(base), close, card)
}
