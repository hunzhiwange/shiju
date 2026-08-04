#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum NotificationPriority {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiNotification {
    pub(crate) id: String,
    pub(crate) message: String,
    pub(crate) priority: NotificationPriority,
    pub(crate) created_ms: u64,
    pub(crate) expires_ms: Option<u64>,
    pub(crate) persistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct NotificationStore {
    pub(crate) items: Vec<TuiNotification>,
    pub(crate) suppressed_dialogs: usize,
}
