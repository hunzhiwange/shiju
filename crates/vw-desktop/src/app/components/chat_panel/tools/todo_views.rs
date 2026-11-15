//! 桌面输入投影及存储回退注入。
use crate::app::{App, Message};
pub fn tool_todowrite_compact_view<'a>(app: &'a App, m: usize, t: usize, visible: &str) -> Option<iced::Element<'a, Message>> {
 desktop_chat_tools::tools::todo_views::tool_todowrite_compact_view(&super::card_adapter::input(app),m,t,visible).map(|v| v.map(super::card_adapter::map_card))
}
pub fn tool_todos_view<'a>(app: &'a App, m: usize, t: usize, visible: &str) -> Option<iced::Element<'a, Message>> {
 desktop_chat_tools::tools::todo_views::tool_todos_view(&super::card_adapter::input(app),m,t,visible, || crate::app::components::input_panel::todo_panel::read_todos_for_panel(app)).map(|v| v.map(super::card_adapter::map_card))
}
