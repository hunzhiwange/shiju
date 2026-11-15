use crate::app::{App, Message, message};
use desktop_chat_tools::advanced_input::{AdvancedInput, Message as LocalMessage};
fn map_message(local: LocalMessage) -> Message {
 Message::Chat(match local {
 LocalMessage::OpenToolDetail(m,t,v) => message::ChatMessage::OpenToolDetail(m,t,v),
 LocalMessage::ToolHover(m,t) => message::ChatMessage::ToolHover(m,t),
 LocalMessage::ToolHoverLeave => message::ChatMessage::ToolHoverLeave,
 LocalMessage::OpenMessageContextMenu {target,x,y,text} => message::ChatMessage::OpenMessageContextMenu {target,x,y,text},
 LocalMessage::Text(v) => return super::super::tool_text_support::map_message(v),
 })
}
#[cfg(test)]
pub(super) fn agent_tool_detail_message(m: usize,t: usize,v: &str) -> Message {
 map_message(desktop_chat_tools::tools::advanced_view::agent_tool_detail_message(m,t,v))
}
pub fn tool_advanced_view<'a>(app: &'a App, msg_idx: usize, tool_idx: usize, visible: &str) -> Option<iced::Element<'a, Message>> {
 desktop_chat_tools::tools::advanced_view::tool_advanced_view(AdvancedInput {
 i18n: crate::app::i18n::I18n::new(app.desktop_locale),
 hovered: app.chat_tool_hovered_idx,
 text: super::super::tool_text_support::text_state(app),
 }, msg_idx, tool_idx, visible).map(|element| element.map(map_message))
}
