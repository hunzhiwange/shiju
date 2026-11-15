//! Terminal view assembly and local message mapping.
use crate::app::{App, Message};
use crate::app::message::{TerminalMessage, ViewMessage};
pub fn view(app: &App) -> iced::Element<'_, Message> {
    desktop_terminal::terminal_panel::view(desktop_terminal::ViewInput {
        tabs: &app.terminal.tabs,
        active_id: app.terminal.active_id,
        is_visible: app.terminal.is_visible,
        height: app.terminal.height,
        tab_context_menu_id: app.terminal.tab_context_menu_id,
        tab_context_menu_pos: app.terminal.tab_context_menu_pos,
        desktop_locale: app.desktop_locale,
        focused: app.focus_area == crate::app::FocusArea::Terminal,
        #[cfg(not(target_arch = "wasm32"))]
        panes: &app.terminal.panes,
        #[cfg(not(target_arch = "wasm32"))]
        content_menu: app.terminal.content_menu,
        #[cfg(not(target_arch = "wasm32"))]
        is_fullscreen: app.terminal.is_fullscreen,
    }).map(map_message)
}
fn map_message(message: desktop_terminal::message::Message) -> Message {
    use desktop_terminal::message::Message as Local;
    match message {
        Local::Terminal(message) => Message::Terminal(map_terminal(message)),
        Local::Batch(messages) => Message::Batch(messages.into_iter().map(map_message).collect()),
        Local::Hide => Message::View(ViewMessage::ToggleTerminalPanel),
    }
}
fn map_terminal(message: desktop_terminal::message::TerminalMessage) -> TerminalMessage {
    use desktop_terminal::message::TerminalMessage as Local;
    match message {
        Local::Add => TerminalMessage::Add,
        #[cfg(not(target_arch = "wasm32"))]
        Local::AddToPane(v0) => TerminalMessage::AddToPane(v0),
        #[cfg(not(target_arch = "wasm32"))]
        Local::ToggleSplitMenu(v0) => TerminalMessage::ToggleSplitMenu(v0),
        #[cfg(not(target_arch = "wasm32"))]
        Local::CloseSplitMenu => TerminalMessage::CloseSplitMenu,
        #[cfg(not(target_arch = "wasm32"))]
        Local::Split(v0, v1, v2) => TerminalMessage::Split(v0, v1, v2),
        #[cfg(not(target_arch = "wasm32"))]
        Local::ResizePane(v0, v1) => TerminalMessage::ResizePane(v0, v1),
        #[cfg(not(target_arch = "wasm32"))]
        Local::DockPane(v0, v1, v2) => TerminalMessage::DockPane(v0, v1, v2),
        #[cfg(not(target_arch = "wasm32"))]
        Local::TogglePaneMaximized(v0, v1) => TerminalMessage::TogglePaneMaximized(v0, v1),
        Local::Select(v0) => TerminalMessage::Select(v0),
        Local::Close(v0) => TerminalMessage::Close(v0),
        Local::Event(v0) => TerminalMessage::Event(v0),
        Local::RenameStart(v0) => TerminalMessage::RenameStart(v0),
        Local::RenameChanged(v0, v1) => TerminalMessage::RenameChanged(v0, v1),
        Local::RenameSave(v0) => TerminalMessage::RenameSave(v0),
        Local::RenameCancel(v0) => TerminalMessage::RenameCancel(v0),
        Local::TabContextOpen(v0, v1, v2) => TerminalMessage::TabContextOpen(v0, v1, v2),
        Local::TabContextClose => TerminalMessage::TabContextClose,
        #[cfg(not(target_arch = "wasm32"))]
        Local::ContentMenuOpen(v0, v1) => TerminalMessage::ContentMenuOpen(v0, v1),
        #[cfg(not(target_arch = "wasm32"))]
        Local::ContentMenuClose => TerminalMessage::ContentMenuClose,
        #[cfg(not(target_arch = "wasm32"))]
        Local::ContentMenuAction(v0, v1) => TerminalMessage::ContentMenuAction(v0, v1),
        #[cfg(not(target_arch = "wasm32"))]
        Local::ToggleFullscreen => TerminalMessage::ToggleFullscreen,
    }
}
