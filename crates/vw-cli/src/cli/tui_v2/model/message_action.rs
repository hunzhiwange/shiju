use super::UiMessageId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MessageActionKind {
    Browse,
    Rewind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MessageActionIntent {
    Copy { message_id: UiMessageId, text: String },
    Edit { message_id: UiMessageId, text: String, cancel_active_turn: bool },
    Fork { message_id: UiMessageId },
    Restore { message_id: UiMessageId },
    Summarize { message_id: UiMessageId },
    Rewind { message_id: UiMessageId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MessageActionState {
    pub(crate) active: bool,
    pub(crate) selected_id: Option<UiMessageId>,
    pub(crate) expanded: bool,
    pub(crate) kind: MessageActionKind,
    pub(crate) preserved_prompt_draft: String,
}

impl Default for MessageActionState {
    fn default() -> Self {
        Self {
            active: false,
            selected_id: None,
            expanded: false,
            kind: MessageActionKind::Browse,
            preserved_prompt_draft: String::new(),
        }
    }
}
