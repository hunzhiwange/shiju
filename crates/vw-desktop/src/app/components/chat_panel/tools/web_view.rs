use crate::app::{App, Message};
pub fn tool_web_view<'a>(app: &'a App, msg_idx: usize, tool_idx: usize, visible: &str) -> Option<iced::Element<'a, Message>> {
 desktop_chat_tools::tools::web_view::tool_web_view(&super::card_adapter::input(app), msg_idx, tool_idx, visible).map(|view| view.map(super::card_adapter::map_card))
}
