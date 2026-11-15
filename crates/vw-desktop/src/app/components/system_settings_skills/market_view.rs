use crate::app::{App, Message};
pub(super) fn view<'a>(app: &'a App, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    desktop_settings_skills::market_view::view(&app.skills_settings.market, base, crate::app::message::settings::skills::market_view::message)
}
