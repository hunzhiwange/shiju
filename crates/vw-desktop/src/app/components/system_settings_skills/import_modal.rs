use crate::app::{App, Message};
pub(super) fn view<'a>(app: &'a App, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    desktop_settings_skills::import_modal::view(&app.skills_settings.import, base, crate::app::message::settings::skills::import_modal::message)
}
