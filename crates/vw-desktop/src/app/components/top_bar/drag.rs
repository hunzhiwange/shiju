use crate::app::Message;
pub(super) fn traffic_light_spacer() -> iced::Element<'static, Message> { desktop_navigation::components::top_bar::drag::traffic_light_spacer().map(super::mapping::map_message) }
pub(super) fn drag_spacer() -> iced::Element<'static, Message> { desktop_navigation::components::top_bar::drag::drag_spacer().map(super::mapping::map_message) }
