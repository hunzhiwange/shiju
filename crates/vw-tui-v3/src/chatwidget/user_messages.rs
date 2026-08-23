//! Plain user-message data retained by the local input queue.

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
