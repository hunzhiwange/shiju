use crate::app::message::TaskBoardMessage;
use crate::app::state::TaskBoardDetailTab;
use crate::app::{App, Message, Screen};
use iced::Length;
use iced::widget::{Space, responsive, row};
mod board;
mod common;
mod control;
mod detail;
mod drag;
pub(in crate::app) mod modals;
mod navigation;
mod panel;
mod properties;
mod search;
mod style;
mod work_display;
mod work_filter_advanced;
mod work_filter_bar;
mod work_filter_date;
mod work_filter_picker;
mod work_filter_scope;
mod work_sidebar;
mod work_view_editor;

pub(in crate::app) use search::{SEARCH_INPUT_ID, with_search_modal};
pub(in crate::app) use work_display::WORK_VIEW_NAME_INPUT_ID;
pub(in crate::app) use work_filter_picker::{WORK_FILTER_SEARCH_INPUT_ID, inherited_filter_query};
pub(in crate::app) use work_filter_scope::configured_work_agent;

pub fn view(app: &App) -> iced::Element<'_, Message> {
    let task = app.task_board.task_board_viewing_logs.as_ref();
    let show_detail = task.is_some() && !app.task_board.task_board_worktree_pixel_office;
    let input = desktop_task_board::page::PageView {
        task_board_worktree_pixel_office: app.task_board.task_board_worktree_pixel_office,
        kanban: true,
        viewing_logs: !app.task_board.task_board_create_modal_open
            && task.is_some()
            && app.task_board.task_board_detail_tab == TaskBoardDetailTab::Logs,
        task_board_settings_modal_open: app.task_board.task_board_settings_modal_open,
    };
    let page = desktop_task_board::page::assemble(
        input,
        desktop_task_board::page::PageBuilders {
            navigation: Some(navigation::view(app)),
            header: Box::new(move || {
                if app.task_board.task_board_worktree_pixel_office {
                    control::build_control_buttons(app)
                } else {
                    Space::new().into()
                }
            }),
            board: Box::new(move || {
                responsive(move |size| {
                    let show_sidebar = size.width >= 550.0;
                    let sidebar_width = if size.width >= 728.0 { 224.0 } else { 188.0 };
                    let sidebar = show_sidebar.then(|| work_sidebar::view(app, sidebar_width));
                    let content = if let Some(task) = task {
                        detail::view(app, task)
                    } else {
                        work_display::view(app, !show_sidebar)
                    };
                    row(sidebar.into_iter().chain([content]))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                })
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
