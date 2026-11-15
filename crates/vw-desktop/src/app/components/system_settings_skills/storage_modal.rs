use crate::app::{App, Message};
pub(super) fn view<'a>(app: &'a App, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    desktop_settings_skills::storage_modal::view(&app.skills_settings.storage, base, crate::app::message::settings::skills::storage_modal::message)
}
