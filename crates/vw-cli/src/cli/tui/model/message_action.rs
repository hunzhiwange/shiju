//! transcript 消息选择与后续操作的视图状态。
//!
//! 本模块只表达用户意图和面板状态，复制、分叉、恢复等副作用由控制器执行。

use super::UiMessageId;

/// 消息操作面板的导航模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MessageActionKind {
    Browse,
    Rewind,
}

/// 用户针对某条消息确认的操作意图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MessageActionIntent {
    Copy { message_id: UiMessageId, text: String },
    Edit { message_id: UiMessageId, text: String, cancel_active_turn: bool },
    Fork { message_id: UiMessageId },
    Restore { message_id: UiMessageId },
    Summarize { message_id: UiMessageId },
    Rewind { message_id: UiMessageId },
}

/// 消息操作模式的当前选择与进入前草稿。
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
