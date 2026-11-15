use crate::app::{App, Message};
pub(super) fn view<'a>(app: &App, base: iced::Element<'a, Message>, form: &'a crate::app::state::PluginInstallForm) -> iced::Element<'a, Message> {
    desktop_settings_skills::plugin_install_modal::view(app.desktop_locale, &app.app_theme, base, form, crate::app::message::settings::plugin_install::message)
}
