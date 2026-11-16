use crate::app::{App, Message};
pub(crate) fn with_modal<'a>(app: &'a App, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
 if !app.web_install.visible { return base; }
 let (close, card) = desktop_navigation::components::install_web_modal::with_modal(
  desktop_navigation::components::install_web_modal::Input {
   web_install: &app.web_install, desktop_update: &app.desktop_update,
   desktop_locale: app.general_settings.desktop_locale, app_theme: &app.appearance.app_theme, last_copied_code_hash: app.clipboard_state.last_copied_code_hash,
  }, super::install_cli_modal::help);
 super::system_settings_common::settings_modal_overlay(Some(base), super::install_cli_modal::map(close), card.map(super::install_cli_modal::map))
}
