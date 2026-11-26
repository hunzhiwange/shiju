use crate::app::{App, Message};
use crate::app::message::TaskBoardMessage;
use desktop_task_board::board::{BoardView, BoardSlots, DisplayView};
use std::rc::Rc;
impl From<TaskBoardMessage> for Message { fn from(message: TaskBoardMessage) -> Self { Self::TaskBoard(message) } }
fn view(app: &App) -> BoardView<'_, Message> {
    BoardView {
        task_visible: Some(Rc::new(move |task| app.task_board.work_display_task_visible(app.navigation_tabs.project_path.as_deref().unwrap_or(""), task))),
        task_board_settings: &app.task_board.task_board_settings,
        task_labels: &app.task_board.task_labels,
        task_board_tasks: &app.task_board.task_board_tasks,
        task_board_selected_tasks: &app.task_board.task_board_selected_tasks,
        task_board_bulk_active_status: app.task_board.task_board_bulk_active_status,
        task_board_bulk_priority_input: &app.task_board.task_board_bulk_priority_input,
        task_board_bulk_acp_agent: &app.task_board.task_board_bulk_acp_agent,
        task_board_bulk_reasoning_effort: app.task_board.task_board_bulk_reasoning_effort,
        task_board_dragging: &app.task_board.task_board_dragging,
        cursor_position: app.cursor_position,
        task_board_context_menu: &app.task_board.task_board_context_menu,
        task_board_column_has_vertical_scrollbar: &app.task_board.task_board_column_has_vertical_scrollbar,
        desktop_locale: app.general_settings.desktop_locale,
        slots: BoardSlots {
            model: Rc::new(move || super::panel::build_bulk_model_selector(app)),
            agent: Rc::new(move || super::panel::build_bulk_agent_selector(app)),
            executor: Rc::new(move || super::panel::build_bulk_executor_selector(app)),
            reasoning_effort: Rc::new(move || super::panel::build_bulk_reasoning_effort_selector(app)),
            context_menu: Rc::new(move |task| super::modals::build_context_menu(crate::app::i18n::I18n::new(app.general_settings.desktop_locale), task)),
        },
    }
}
pub fn build_work_display(app: &App) -> iced::Element<'_, Message> {
    let board = &app.task_board;
    if board.task_board_loading && board.task_board_tasks.is_empty() {
        let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
        return iced::widget::container(iced::widget::text(i18n.__("正在加载任务…")).size(13))
            .center_x(iced::Length::Fill)
            .padding(24)
            .into();
    }
    desktop_task_board::board::build_work_display(&view(app), DisplayView {
        mode: board.task_board_view_mode,
        options: board.work_display,
        collapsed: &board.collapsed_work_groups,
        agent_labels: app.agents_settings.entries.iter()
            .map(|entry| (entry.key.as_str(), entry.label.as_str()))
            .collect(),
        allow_legacy_reorder: board.work_display.sort == Default::default()
            && !board.work_display.ascending
            && board.work_search.trim().is_empty(),
    })
}

#[cfg(test)]
fn build_kanban_board(app: &App) -> iced::Element<'_, Message> {
    build_work_display(app)
}

#[cfg(test)]
#[path = "board_adapter_tests.rs"]
mod board_adapter_tests;
