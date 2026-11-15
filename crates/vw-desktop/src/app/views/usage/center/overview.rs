pub use desktop_usage::center::overview::{Change, apply, default_query};
pub fn view(query: &vw_api_usage::UsageQuery) -> iced::Element<'_, crate::app::Message> { desktop_usage::center::overview::view(query).map(|change| crate::app::Message::View(crate::app::message::ViewMessage::UsageCenter(super::Event::Filter(change)))) }
