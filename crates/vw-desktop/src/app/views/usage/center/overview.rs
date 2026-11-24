pub use desktop_usage::center::overview::{Change, FilterMenu, apply, default_query};
pub fn view(
    i18n: desktop_i18n::I18n,
    query: &vw_api_usage::UsageQuery,
    open_menu: Option<FilterMenu>,
) -> iced::Element<'_, crate::app::Message> {
    desktop_usage::center::overview::view(i18n, query, open_menu).map(|change| {
        crate::app::Message::View(crate::app::message::ViewMessage::UsageCenter(
            super::Event::Filter(change),
        ))
    })
}
