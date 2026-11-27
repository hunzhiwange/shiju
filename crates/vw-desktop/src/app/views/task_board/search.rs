use crate::app::components::system_settings_common::{
    icon_svg, settings_close_button, settings_modal_header, settings_modal_layout,
    settings_muted_text_style, settings_text_input,
};
use crate::app::i18n::I18n;
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message, Screen};
use desktop_assets::assets::Icon;
use desktop_task_board::board::{task_status_marker, task_title};
use desktop_task_board::work_views::task_matches_search;
use desktop_task_model::Task;
use desktop_ui_controls::system_settings_common::{
    icons::icon_btn_svg_style, settings_modal_overlay_with_escape_and_backdrop_dismiss,
};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};
use std::cmp::Reverse;

pub(in crate::app) const SEARCH_INPUT_ID: &str = "task-board-search";

pub(in crate::app) fn with_search_modal<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(query) = app.task_board.work_search.as_deref() else {
        return base;
    };
    if app.navigation_tabs.screen != Screen::TaskBoard
        || app.navigation_panels.show_system_settings
        || app.task_board.task_board_create_modal_open
        || app.task_board.task_board_settings_modal_open
    {
        return base;
    }

    let i18n = I18n::new(app.general_settings.desktop_locale);
    let project = app.navigation_tabs.project_path.as_deref().unwrap_or("");
    let close = Message::from(TaskBoardMessage::SearchWorkTasks(None));
    let open = |task: &Task| {
        Message::from(TaskBoardMessage::OpenTaskDetail {
            project_path: project.to_owned(),
            task_id: task.id.clone(),
        })
    };
    let recent = query.trim().is_empty();
    let mut matches: Vec<_> = app
        .task_board
        .task_board_tasks
        .iter()
        .filter(|task| !task.archived && !task.deleted && task_matches_search(task, query))
        .collect();
    matches.sort_by_key(|task| Reverse(task.updated_at_ms));
    if recent {
        matches.truncate(8);
    }
    let input = settings_text_input(i18n.__("搜索任务名称或编号…"), query)
        .id(SEARCH_INPUT_ID)
        .width(Length::Fill)
        .on_input(|query| TaskBoardMessage::SearchWorkTasks(Some(query)).into())
        .on_submit_maybe(matches.first().map(|task| open(task)));
    let header = settings_modal_header(
        row![icon_svg(Icon::Search, 17.0).style(icon_btn_svg_style), input]
            .spacing(10)
            .align_y(Alignment::Center),
        settings_close_button(close.clone()),
    );

    let mut body = column![
        container(
            text(i18n.__(if recent { "最近更新" } else { "搜索结果" }))
                .size(12)
                .style(settings_muted_text_style)
        )
        .padding([4, 10]),
    ]
    .spacing(3)
    .width(Length::Fill);
    if matches.is_empty() {
        let message = if app.task_board.task_board_loading {
            "正在加载任务…"
        } else {
            "没有符合条件的任务"
        };
        body = body.push(container(text(i18n.__(message)).size(13)).padding([12, 10]));
    }
    for task in matches {
        body = body.push(
            button(
                row![
                    task_status_marker(task.status),
                    container(text(task_title(task)).size(14).wrapping(text::Wrapping::None))
                        .width(Length::Fill)
                        .clip(true),
                    container(
                        text(&task.id)
                            .size(11)
                            .style(settings_muted_text_style)
                            .wrapping(text::Wrapping::None)
                    )
                    .max_width(136)
                    .clip(true),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding([9, 10])
            .style(|theme, status| super::style::navigation_button(theme, status, false))
            .on_press(open(task)),
        );
    }
    let card = settings_modal_layout(header, body, None).width(620).height(434);
    settings_modal_overlay_with_escape_and_backdrop_dismiss(base, close, card)
}
