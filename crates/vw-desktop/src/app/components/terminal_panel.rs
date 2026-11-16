//! Terminal view assembly and local message mapping.
use crate::app::{App, Message};
use crate::app::message::ViewMessage;
pub fn view(app: &App) -> iced::Element<'_, Message> {
    desktop_terminal::terminal_panel::view(desktop_terminal::ViewInput {
        tabs: &app.terminal.tabs,
        active_id: app.terminal.active_id,
        is_visible: app.terminal.is_visible,
        height: app.terminal.height,
        tab_context_menu_id: app.terminal.tab_context_menu_id,
        tab_context_menu_pos: app.terminal.tab_context_menu_pos,
        desktop_locale: app.general_settings.desktop_locale,
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
        Local::Terminal(message) => Message::Terminal(message),
        Local::Batch(messages) => Message::Batch(messages.into_iter().map(map_message).collect()),
        Local::Hide => Message::View(ViewMessage::ToggleTerminalPanel),
    }
}
