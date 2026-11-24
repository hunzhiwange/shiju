use crate::app::{App, Message};
pub(super) fn view<'a>(
    app: &'a App,
    base: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    desktop_settings_skills::jobs_view::view(
        desktop_i18n::I18n::new(app.general_settings.desktop_locale),
        &app.skills_settings.jobs,
        base,
        crate::app::message::settings::skills::jobs_view::message,
    )
}
