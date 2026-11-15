use crate::app::{App, Message};
use iced::Task;
use super::TerminalMessage;
pub use desktop_terminal::updates::config::ConfigPersistence;
pub fn update(app: &mut App, message: TerminalMessage) -> Task<Message> { super::update(app, message) }
