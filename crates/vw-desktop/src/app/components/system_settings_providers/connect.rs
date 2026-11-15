use crate::app::{App, Message};
use iced::Element;
use crate::app::components::system_settings_models::{map_view_message, view_style};
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let Some((close, card)) = desktop_settings_providers::connect::modal(&app.provider_settings, view_style(app)) else { return dialog };
    crate::app::components::system_settings_common::settings_modal_overlay(Some(dialog), map_view_message(close), card.map(map_view_message))
}
