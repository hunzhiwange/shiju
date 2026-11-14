#[derive(Debug, PartialEq, Eq)]
pub enum UserEvent {
    Resize(u32, u32),
    Move(i32, i32),
    OpenUrl(String),
    PopupClosed(u64),
    OpenExternal(String),
    #[cfg(not(coverage))]
    ConfirmExternal,
    NavigationChanged(String),
    ToolbarReady,
    ClearBrowser,
    Back,
    Forward,
    Reload,
    SetAlwaysOnTop(bool),
    Show,
    Hide,
    Close,
}
