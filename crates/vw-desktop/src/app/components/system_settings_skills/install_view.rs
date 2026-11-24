use crate::app::{App, Message};
pub(super) fn view<'a>(
    app: &'a App,
    base: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    desktop_settings_skills::install_view::view(
        desktop_i18n::I18n::new(app.general_settings.desktop_locale),
        &app.skills_settings.install,
        &app.apps_catalog.host,
        base,
        crate::app::message::settings::skills::install_view::message,
    )
}
