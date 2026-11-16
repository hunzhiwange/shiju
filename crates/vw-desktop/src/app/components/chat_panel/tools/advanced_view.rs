use crate::app::{Message, message};
use desktop_chat_tools::advanced_input::Message as LocalMessage;
pub(super) fn map_message(local: LocalMessage) -> Message {
    Message::Chat(match local {
        LocalMessage::OpenToolDetail(m, t, v) => message::ChatMessage::OpenToolDetail(m, t, v),
        LocalMessage::ToolHover(m, t) => message::ChatMessage::ToolHover(m, t),
        LocalMessage::ToolHoverLeave => message::ChatMessage::ToolHoverLeave,
        LocalMessage::OpenMessageContextMenu { target, x, y, text } => {
            message::ChatMessage::OpenMessageContextMenu { target, x, y, text }
        }
        LocalMessage::Text(v) => return super::super::tool_text_support::map_message(v),
    })
}
