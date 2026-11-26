use crate::app::message::TaskBoardMessage;
use crate::app::state::TaskBoardDetailTab;
use crate::app::{App, Message, Screen};
use iced::Length;
use iced::widget::{Space, column, responsive, row};
mod board;
mod common;
mod control;
mod detail;
mod drag;
pub(in crate::app) mod modals;
mod panel;
mod properties;
mod work_display;
mod work_sidebar;
mod work_view_editor;
pub fn view(app: &App) -> iced::Element<'_, Message> {
    let task = app.task_board.task_board_viewing_logs.as_ref();
    let show_detail = task.is_some() && !app.task_board.task_board_worktree_pixel_office;
    let input = desktop_task_board::page::PageView {
        task_board_worktree_pixel_office: app.task_board.task_board_worktree_pixel_office,
        kanban: true,
        viewing_logs: !app.task_board.task_board_create_modal_open && task.is_some()
            && app.task_board.task_board_detail_tab == TaskBoardDetailTab::Logs,
        task_board_settings_modal_open: app.task_board.task_board_settings_modal_open,
    };
    let page = desktop_task_board::page::assemble(
        input,
        desktop_task_board::page::PageBuilders {
            header: Box::new(move || {
                if show_detail {
                    Space::new().into()
                } else {
                    control::build_control_buttons(app)
                }
            }),
            board: Box::new(move || {
                if let Some(task) = task {
                    return responsive(move |size| {
                        let sidebar = (size.width >= 700.0).then(|| work_sidebar::view(app));
                        row(sidebar.into_iter().chain([detail::view(app, task)]))
                            .spacing(12)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .into()
                    })
                    .into();
                }
                row![
                    work_sidebar::view(app),
                    column![work_display::view(app)].width(Length::Fill).height(Length::Fill)
                ]
                .spacing(12)
                .height(iced::Length::Fill)
                .into()
            }),
            drag: Box::new(|| drag::build_drag_preview_layer(app)),
            detail: Box::new(move || panel::build_panel(app, true, show_detail)),
            settings: Box::new(|| modals::build_task_settings_modal(app)),
            context_closed: Message::TaskBoard(TaskBoardMessage::ContextMenuClosed),
            close_logs: Message::TaskBoard(if show_detail {
                TaskBoardMessage::SelectDetailTab(TaskBoardDetailTab::Basic)
            } else {
                TaskBoardMessage::CloseTaskLogs
            }),
            close_settings: Message::TaskBoard(TaskBoardMessage::CloseSettingsModal),
        },
    );
    detail::with_field_modal(app, page)
}

pub(in crate::app) fn with_create_modal<'a>(
    app: &'a App,
    base: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    if app.navigation_tabs.screen != Screen::TaskBoard
        || app.navigation_panels.show_system_settings
        || !app.task_board.task_board_create_modal_open
        || app.task_board.task_board_settings_modal_open
    {
        return base;
    }

    desktop_ui_controls::system_settings_common::settings_modal_overlay_with_escape(
        base,
        Message::TaskBoard(TaskBoardMessage::CreateTaskCancelled),
        panel::build_panel(app, false, false),
    )
}
