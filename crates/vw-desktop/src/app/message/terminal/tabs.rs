use crate::app::{App, Message};
use iced::Task;
pub(crate) fn reap_retired_task(app: &App) -> Task<Message> {
    desktop_terminal::updates::tabs::reap_retired_task(&app.terminal).map(Message::Terminal)
}
