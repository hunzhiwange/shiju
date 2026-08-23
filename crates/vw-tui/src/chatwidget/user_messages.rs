//! Plain user-message data retained by the local input queue.
//! 本模块负责输入队列保留的普通用户消息。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use crate::vibe::event::V3Attachment;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct UserMessage {
    pub(super) text: String,
    pub(super) attachments: Vec<V3Attachment>,
}

impl UserMessage {
    pub(super) fn new(text: impl Into<String>, attachments: Vec<V3Attachment>) -> Self {
        Self { text: text.into(), attachments }
    }
}
