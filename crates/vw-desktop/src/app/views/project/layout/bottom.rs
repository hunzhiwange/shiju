use crate::app::{App, Message, message, components::terminal_panel};
use desktop_workspace::views::project::layout::bottom as domain;
pub fn bottom_panel(app: &App) -> iced::Element<'_, Message> { domain::bottom_panel(app.terminal.is_visible, app.terminal.height, if app.terminal.is_visible { terminal_panel::view(app) } else { iced::widget::Space::new().into() }, Message::Terminal(message::TerminalMessage::DragStarted)) }
pub(super) fn fullscreen_terminal(app: &App) -> iced::Element<'_, Message> { domain::fullscreen_terminal(terminal_panel::view(app)) }
#[cfg(test)]
#[path = "bottom_tests.rs"]
mod bottom_tests;
