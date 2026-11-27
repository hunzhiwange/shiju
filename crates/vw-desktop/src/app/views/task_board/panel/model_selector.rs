use crate::app::{App, Message};
use crate::app::message::{TaskBoardMessage, SettingsMessage, ViewMessage};
use desktop_task_board::model_selector::{SelectorView, SelectorMessage};
pub(super) fn view(app: &App) -> SelectorView<'_> {
    let board = &app.task_board;
    SelectorView {
        draft_model: &board.task_board_draft.model,
        draft_acp_agent: board.task_board_draft.acp_agent.as_deref(),
        bulk_has_acp_tasks: board.task_board_tasks.iter().any(|task| {
            Some(task.status) == board.task_board_bulk_active_status
                && board.task_board_selected_tasks.contains(&task.id)
                && task.acp_agent.is_some()
        }),
        bulk_model: &board.task_board_bulk_model_input,
        task_board_model_popover: board.task_board_model_popover,
        task_board_bulk_model_popover: board.task_board_bulk_model_popover,
        desktop_locale: app.general_settings.desktop_locale,
        model_settings: &app.model_settings,
    }
}
pub(super) fn map(message: SelectorMessage) -> Message {
    match message {
        SelectorMessage::ToggleModelPopover => Message::TaskBoard(TaskBoardMessage::ToggleModelPopover),
        SelectorMessage::ToggleBulkModelPopover => Message::TaskBoard(TaskBoardMessage::ToggleBulkModelPopover),
        SelectorMessage::CloseModelPopover => Message::TaskBoard(TaskBoardMessage::CloseModelPopover),
        SelectorMessage::CloseBulkModelPopover => Message::TaskBoard(TaskBoardMessage::CloseBulkModelPopover),
        SelectorMessage::ModelSelected(v) => Message::TaskBoard(TaskBoardMessage::ModelSelected(v)),
        SelectorMessage::UpdateEditingTaskModel(v) => Message::TaskBoard(TaskBoardMessage::UpdateEditingTaskModel(v)),
        SelectorMessage::BulkModelSelected(v) => Message::TaskBoard(TaskBoardMessage::BulkModelSelected(v)),
        SelectorMessage::ModelQueryChanged(v) => Message::Settings(SettingsMessage::ModelQueryChanged(v)),
        SelectorMessage::OpenModelDetail(p, m) => Message::View(ViewMessage::OpenSystemSettingsModelDetail(p,m)),
        SelectorMessage::OpenModels => Message::View(ViewMessage::OpenSystemSettingsTab(crate::app::components::system_settings::SystemTab::Models)),
        SelectorMessage::OpenProviders => Message::View(ViewMessage::OpenSystemSettingsTab(crate::app::components::system_settings::SystemTab::Providers)),
    }
}
pub fn build_bulk_model_selector(app: &App) -> iced::Element<'_, Message> {
    desktop_task_board::model_selector::build_bulk_model_selector(view(app)).map(map)
}
