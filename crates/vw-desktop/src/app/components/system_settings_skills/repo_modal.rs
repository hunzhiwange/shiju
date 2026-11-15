use crate::app::{App, Message};
pub(super) fn view<'a>(app: &'a App, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    desktop_settings_skills::repo_modal::view(&app.skills_settings.repositories, base, crate::app::message::settings::skills::repo_modal::message)
}
