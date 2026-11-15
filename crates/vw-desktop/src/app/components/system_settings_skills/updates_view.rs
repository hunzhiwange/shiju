use crate::app::{App, Message};
pub(super) fn view<'a>(app: &'a App, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    desktop_settings_skills::updates_view::view(&app.skills_settings.updates, &app.apps_catalog.host, base, crate::app::message::settings::skills::updates_view::message)
}
