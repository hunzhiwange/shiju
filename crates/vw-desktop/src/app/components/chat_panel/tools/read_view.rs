//! 桌面只读输入投影与卡片消息映射。
pub fn tool_read_view<'a>(app: &'a crate::app::App, msg_idx: usize, tool_idx: usize, visible: &str) -> Option<iced::Element<'a, crate::app::Message>> {
 desktop_chat_tools::tools::read_view::tool_read_view(&super::card_adapter::input(app), msg_idx, tool_idx, visible).map(|v| v.map(super::card_adapter::map_card))
}
pub fn tool_read_compact_view<'a>(app: &'a crate::app::App, visible: &str) -> Option<iced::Element<'a, crate::app::Message>> {
 desktop_chat_tools::tools::read_view::tool_read_compact_view(&super::card_adapter::input(app), visible).map(|v| v.map(super::card_adapter::map_card))
}
