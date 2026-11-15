#[cfg(not(target_arch = "wasm32"))]
use iced::widget::{Id, pane_grid::{Direction, Pane, ResizeEvent, Target}};
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy)]
pub enum ContentAction {
    NewTerminal,
    Split(Direction),
    Copy,
    Paste,
    PasteText,
    SelectAll,
    Clear,
    AskInChat,
    InsertInChat,
    CloseTab,
}

#[derive(Debug, Clone)]
pub enum TerminalMessage {
    Add,
    #[cfg(not(target_arch = "wasm32"))]
    AddToPane(Pane),
    #[cfg(not(target_arch = "wasm32"))]
    ToggleSplitMenu(Pane),
    #[cfg(not(target_arch = "wasm32"))]
    CloseSplitMenu,
    #[cfg(not(target_arch = "wasm32"))]
    Split(Id, Pane, Direction),
    #[cfg(not(target_arch = "wasm32"))]
    ResizePane(Id, ResizeEvent),
    #[cfg(not(target_arch = "wasm32"))]
    DockPane(Id, Pane, Target),
    #[cfg(not(target_arch = "wasm32"))]
    TogglePaneMaximized(Id, Pane),
    Select(u64),
    Close(u64),
    #[cfg(not(target_arch = "wasm32"))]
    Event(iced_term::Event),
    #[cfg(target_arch = "wasm32")]
    Event(()),
    RenameStart(u64),
    RenameChanged(u64, String),
    RenameSave(u64),
    RenameCancel(u64),
    TabContextOpen(u64, f32, f32),
    TabContextClose,
    #[cfg(not(target_arch = "wasm32"))]
    ContentMenuOpen(u64, iced::Point),
    #[cfg(not(target_arch = "wasm32"))]
    ContentMenuClose,
    #[cfg(not(target_arch = "wasm32"))]
    ContentMenuAction(u64, ContentAction),
    #[cfg(not(target_arch = "wasm32"))]
    ToggleFullscreen,
}

#[derive(Debug, Clone)]
pub enum Message { Terminal(TerminalMessage), Batch(Vec<Message>), Hide }
