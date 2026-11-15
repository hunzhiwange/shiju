//! 宿主输入装配与领域消息映射。
use crate::app::{App, Message};
use iced::Element;
use desktop_settings_runtime::system_settings_reliability as domain;
fn inputs(app: &App) -> domain::View<'_> {
 domain::View { reliability_settings: &app.reliability_settings,
 desktop_locale: app.desktop_locale, app_theme: &app.app_theme,
 last_copied_code_hash: app.last_copied_code_hash,
 render_help: super::system_settings_hooks::render_help,
 }
}
pub fn view(app: &App) -> Element<'_, Message> {
 domain::view(inputs(app)).map(super::system_settings_hooks::map_message)
}
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let Some((title, text, close)) = domain::help(inputs(app)) else { return dialog; };
    super::system_settings_common::with_settings_help_modal(
        app, dialog, title, text, super::system_settings_hooks::map_message(close),
    )
}
#[cfg(test)]
#[path = "system_settings_reliability_ui_refresh_21_tests.rs"]
mod ui_refresh_21_tests;
