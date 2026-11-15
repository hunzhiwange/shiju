use crate::app::{App, Message};
use crate::app::message::TaskBoardMessage;
use desktop_task_board::board::{BoardView, BoardSlots};
use std::rc::Rc;
impl From<TaskBoardMessage> for Message { fn from(message: TaskBoardMessage) -> Self { Self::TaskBoard(message) } }
fn view(app: &App) -> BoardView<'_, Message> {
    BoardView {
        task_board_settings: &app.task_board_settings,
        task_board_tasks: &app.task_board_tasks,
        task_board_selected_tasks: &app.task_board_selected_tasks,
        task_board_bulk_active_status: app.task_board_bulk_active_status,
        task_board_bulk_priority_input: &app.task_board_bulk_priority_input,
        task_board_bulk_acp_agent: &app.task_board_bulk_acp_agent,
        task_board_bulk_reasoning_effort: app.task_board_bulk_reasoning_effort,
        task_board_bulk_task_split_mode: app.task_board_bulk_task_split_mode,
        task_board_dragging: &app.task_board_dragging,
        cursor_position: app.cursor_position,
        task_board_context_menu: &app.task_board_context_menu,
        task_board_column_has_vertical_scrollbar: &app.task_board_column_has_vertical_scrollbar,
        desktop_locale: app.desktop_locale,
        slots: BoardSlots {
            model: Rc::new(move || super::panel::build_bulk_model_selector(app)),
            agent: Rc::new(move || super::panel::build_bulk_agent_selector(app)),
            executor: Rc::new(move || super::panel::build_bulk_executor_selector(app)),
            reasoning_effort: Rc::new(move || super::panel::build_bulk_reasoning_effort_selector(app)),
            task_split: Rc::new(move || super::panel::build_bulk_task_split_selector(app)),
            context_menu: Rc::new(move |task| super::modals::build_context_menu(crate::app::i18n::I18n::new(app.desktop_locale), task)),
        },
    }
}
pub fn build_kanban_board(app: &App) -> iced::Element<'_, Message> {
    desktop_task_board::board::build_kanban_board(&view(app))
}

#[cfg(test)]
#[path = "board_adapter_tests.rs"]
mod board_adapter_tests;
