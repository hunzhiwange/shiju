//! TUI 通知队列的纯状态模型。
//!
//! 通知是否到期与如何渲染由上层决定；此处仅保留排序、持久性和时间元数据。

/// 通知的展示优先级，枚举顺序也是排序顺序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum NotificationPriority {
    Info,
    Success,
    Warning,
    Error,
}

/// 一条可过期或持久显示的 TUI 通知。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiNotification {
    pub(crate) id: String,
    pub(crate) message: String,
    pub(crate) priority: NotificationPriority,
    pub(crate) created_ms: u64,
    pub(crate) expires_ms: Option<u64>,
    pub(crate) persistent: bool,
}

/// 当前通知集合及被更高优先级对话框抑制的计数。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct NotificationStore {
    pub(crate) items: Vec<TuiNotification>,
    pub(crate) suppressed_dialogs: usize,
}
