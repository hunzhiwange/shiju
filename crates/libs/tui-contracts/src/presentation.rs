//! Shared composer and chat presentation data.
//! 输入框与聊天展示共享的纯数据。

use crate::event::V3Attachment;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserMessage {
    pub text: String,
    pub attachments: Vec<V3Attachment>,
}

impl UserMessage {
    pub fn new(text: impl Into<String>, attachments: Vec<V3Attachment>) -> Self {
        Self { text: text.into(), attachments }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MentionBinding {
    pub sigil: char,
    pub mention: String,
    pub path: String,
}

/// Current main-surface activity supplied by the outer bridge.
/// 目前由外桥提供的主要地面活动.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct V3ChatStatus {
    pub label: Option<String>,
    pub detail: Option<String>,
    pub is_task_running: bool,
}

impl V3ChatStatus {
    /// Creates a visible running status.
    /// 创建可见的运行状态。
    #[must_use]
    pub fn running(label: impl Into<String>, detail: Option<String>) -> Self {
        Self { label: Some(label.into()), detail, is_task_running: true }
    }

    /// Creates an idle status with no separate status surface.
    /// 创建一个没有单独状态表面的闲置状态。
    #[must_use]
    pub fn idle() -> Self {
        Self::default()
    }

    pub fn is_running(&self) -> bool {
        self.is_task_running
    }
}
