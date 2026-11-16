//! 通用帮助的桌面消息映射。
use crate::app::{App, Message, message::settings::SettingsMessage};
use crate::app::components::system_settings::SystemTab;
use iced::Element;
use desktop_settings_appearance::help as domain;
pub use domain::help_open_for_tab;
pub(super) fn help_button(locale: config_ui::DesktopLocale, tab: SystemTab) -> Option<Element<'static, Message>> { domain::help_button(locale, tab, Message::Settings(SettingsMessage::SystemHelpOpen(tab))) }
pub fn with_help_modal<'a>(app: &App, base: Element<'a, Message>, active_tab: SystemTab, open_tab: Option<SystemTab>) -> Element<'a, Message> {
 domain::with_help_modal(app.general_settings.desktop_locale, app.clipboard_state.last_copied_code_hash, Message::CopyCode, Message::Settings(SettingsMessage::SystemHelpClose), base, active_tab, open_tab)
}
