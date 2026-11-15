use crate::app::Message;
use super::list::Help;
use iced::Element;
pub(super) fn overlay<'a>(state: &'a crate::app::state::app_providers::list::ListState, form: &'a desktop_provider_apps::state::snippets::Form, app: vw_api_app_identity::AppId, help: Help<'_>, base: Element<'a, Message>) -> Element<'a, Message> {
    let card = desktop_provider_apps::components::snippets::card(state.snapshot.as_ref(), form, app, help.provider_help()).map(super::edit::map);
    let close = if form.pending == Some(desktop_provider_apps::state::snippets::Operation::Save) { Message::None } else { crate::app::message::app_providers::snippets::message(crate::app::message::app_providers::snippets::Event::Close) };
    super::super::system_settings_common::settings_modal_overlay(Some(base), close, card)
}
