use crate::app::{App, Message};
pub(crate) use desktop_chat_tools::tools::workflow_view::workflow_node_detail_body;
pub fn tool_workflow_view<'a>(app: &'a App, msg_idx: usize, tool_idx: usize, visible: &str) -> Option<iced::Element<'a, Message>> {
 desktop_chat_tools::tools::workflow_view::tool_workflow_view(&super::card_adapter::input(app), msg_idx, tool_idx, visible).map(|view| view.map(super::card_adapter::map_card))
}
