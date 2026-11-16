use crate::app::{App, Message};
pub fn tool_bash_view<'a>(app: &'a App, m: usize, t: usize, visible: &str) -> Option<iced::Element<'a, Message>> {
 desktop_chat_tools::tools::bash_view::tool_bash_view(&super::card_adapter::input(app), m,t,visible).map(|e| e.map(super::card_adapter::map_card))
}
