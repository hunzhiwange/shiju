use crate::app::{App, Message};
use iced::Element;
use crate::app::components::system_settings_models::{map_view_message, view_style};
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let Some((close, card)) = desktop_settings_providers::catalog::modal(&app.provider_settings, view_style(app)) else { return dialog };
    crate::app::components::system_settings_common::settings_modal_overlay(Some(dialog), map_view_message(close), card.map(map_view_message))
}
pub(crate) fn deployments<'a>(form: &'a crate::app::state::app_providers::model_catalog::CatalogForm, generation: u64, i18n: crate::app::i18n::I18n, theme: &'a iced::Theme, base: Element<'a, Message>) -> Element<'a, Message> {
 let style = desktop_settings_providers::catalog::CatalogStyle { i18n, theme, help_text: crate::app::components::system_settings_models::help_text };
 let (close, card) = desktop_settings_providers::catalog::deployments(form, generation, i18n, style);
 crate::app::components::system_settings_common::settings_modal_overlay(Some(base), map_view_message(close), card.map(map_view_message))
}
