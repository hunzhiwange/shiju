//! TUI 通知队列的纯状态模型。
//!
//! 通知是否到期与如何渲染由上层决定；此处仅保留排序、持久性和时间元数据。

/// 通知的展示优先级，枚举顺序也是排序顺序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationPriority {
    Info,
    Success,
    Warning,
    Error,
}

/// 一条可过期或持久显示的 TUI 通知。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiNotification {
    pub id: String,
    pub message: String,
    pub priority: NotificationPriority,
    pub created_ms: u64,
    pub expires_ms: Option<u64>,
    pub persistent: bool,
}

/// 当前通知集合及被更高优先级对话框抑制的计数。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NotificationStore {
    pub items: Vec<TuiNotification>,
    pub suppressed_dialogs: usize,
}
