pub use desktop_usage::center::overview::{Change, apply, default_query};
pub fn view(query: &vw_api_usage::UsageQuery, menu_open: bool) -> iced::Element<'_, crate::app::Message> { desktop_usage::center::overview::view(query, menu_open).map(|change| crate::app::Message::View(crate::app::message::ViewMessage::UsageCenter(super::Event::Filter(change)))) }
