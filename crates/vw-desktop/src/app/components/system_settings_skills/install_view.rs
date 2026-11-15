use crate::app::{App, Message};
pub(super) fn view<'a>(app: &'a App, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    desktop_settings_skills::install_view::view(&app.skills_settings.install, &app.apps_catalog.host, base, crate::app::message::settings::skills::install_view::message)
}
