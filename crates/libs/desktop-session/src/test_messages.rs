//! 两组流投影测试共用的最小消息夹具。
use session_ui_types::{ChatMessage, ChatRole};

pub(super) fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}
