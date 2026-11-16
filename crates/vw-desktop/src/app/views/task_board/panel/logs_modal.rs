use crate::app::{App, Message};
use crate::app::task::Task;
#[allow(dead_code)]
pub fn build_task_logs_modal<'a>(app: &'a App, task: &'a Task) -> iced::Element<'a, Message> {
 desktop_task_board::panel::logs_modal::build_task_logs_modal_content(crate::app::i18n::I18n::new(app.general_settings.desktop_locale), &app.task_board.task_board_new_subtask_content, task).map(Message::TaskBoard)
}
