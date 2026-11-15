pub mod computer_control_state;
#[cfg(not(target_arch = "wasm32"))]
pub mod computer_control_update;
#[cfg(not(target_arch = "wasm32"))]
pub mod system_settings_computer_control;
pub mod security_state;
pub mod security_message;
pub mod system_settings_security;
mod security_help;
/// Only the values actually read by the security page; no global application access.
pub struct SecurityView<'a> {
    pub security_settings: &'a security_state::SecuritySettingsState,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: &'a iced::Theme,
    pub last_copied_code_hash: Option<u64>,
    pub help_text: for<'b> fn(&'b str, std::borrow::Cow<'b, str>, Option<iced::Color>, desktop_i18n::I18n) -> iced::Element<'b, security_message::Message>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsTabDescriptor<T> {
    pub tab: T,
    pub label: &'static str,
    pub keywords: &'static [&'static str],
}
impl<T> SettingsTabDescriptor<T> {
    pub const fn new(tab: T, label: &'static str, keywords: &'static [&'static str]) -> Self { Self { tab, label, keywords } }
}

pub mod app_init;

mod security_config;
pub mod security_update;

pub mod save_feedback;
