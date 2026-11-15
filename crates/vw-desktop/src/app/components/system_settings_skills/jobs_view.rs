use crate::app::{App, Message};
pub(super) fn view<'a>(app: &'a App, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    desktop_settings_skills::jobs_view::view(&app.skills_settings.jobs, base, crate::app::message::settings::skills::jobs_view::message)
}
