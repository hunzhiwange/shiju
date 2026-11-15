use crate::app::{App, Message};
use iced::Task;
use super::TerminalMessage;
pub fn update(app: &mut App, message: TerminalMessage) -> Task<Message> { super::update(app, message) }
pub(crate) fn reap_retired_task(app: &App) -> Task<Message> { desktop_terminal::updates::tabs::reap_retired_task(&app.terminal).map(Message::Terminal) }
