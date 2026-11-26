use crate::app::components::system_settings_common::{
    icon_svg, rounded_action_btn_style, settings_muted_text_style, settings_segment_button_style,
};
use crate::app::i18n::I18n;
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_task_board::board::{task_status_marker, task_title};
use desktop_task_board::work_views::BuiltinWorkView;
use desktop_ui_controls::system_settings_common::icons::icon_btn_svg_style;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Border, Element, Font, Length, Theme};

pub fn view(app: &App) -> Element<'_, Message> {
    let project = app.navigation_tabs.project_path.as_deref().unwrap_or("");
    let board = &app.task_board;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let mut content = column![
        text(i18n.__("工作"))
            .size(16)
            .font(Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT })
    ]
    .spacing(6);
    if board.task_board_loading {
        content = content.push(text(i18n.__("正在加载任务…")).size(12));
    }
    for (view, label, icon) in [
        (BuiltinWorkView::MyTasks, i18n.__("我的任务"), Icon::CheckSquare),
        (BuiltinWorkView::AgentExecuting, i18n.__("我的智能体在做"), Icon::Robot),
        (BuiltinWorkView::AllTasks, i18n.__("全部任务"), Icon::Clipboard),
    ] {
        let tasks: Vec<_> = app
            .task_board
            .task_board_tasks
            .iter()
            .filter(|task| app.task_board.work_task_matches(project, view, task))
            .collect();
        let selected = board.work_view == view && board.selected_work_view.is_none();
        content = content.push(
            button(
                row![
                    icon_svg(icon, 14.0).style(icon_btn_svg_style),
                    text(i18n.__(label)).size(13).width(Length::Fill),
                    text(tasks.len().to_string()).size(11).style(settings_muted_text_style),
                ]
                .spacing(7)
                .align_y(Alignment::Center),
            )
            .padding([8, 10])
            .style(move |theme, status| settings_segment_button_style(theme, status, selected))
            .width(Length::Fill)
            .on_press(Message::TaskBoard(TaskBoardMessage::SelectWorkView(view))),
        );
        for task in tasks.into_iter().take(5) {
            let selected = board.task_board_editing_task_id.as_ref() == Some(&task.id);
            let detail = button(
                row![
                    task_status_marker(task.status),
                    container(text(task_title(task)).size(12).wrapping(text::Wrapping::None))
                        .width(Length::Fill)
                        .clip(true),
                ]
                .spacing(7)
                .align_y(Alignment::Center),
            )
            .padding([6, 10])
            .style(move |theme, status| settings_segment_button_style(theme, status, selected))
            .width(Length::Fill)
            .on_press(Message::TaskBoard(TaskBoardMessage::OpenTaskDetail {
                project_path: project.to_owned(),
                task_id: task.id.clone(),
            }));
            let logs = button(text(i18n.__("日志")).size(12))
                .padding([6, 10])
                .style(rounded_action_btn_style)
                .on_press(Message::TaskBoard(TaskBoardMessage::ViewTaskLogs {
                    project_path: project.to_owned(),
                    task_id: task.id.clone(),
                }));
            content = content.push(row![detail, logs].spacing(4).align_y(iced::Alignment::Center));
        }
    }
    content = content.push(text(i18n.__("保存的视图")).size(12).style(settings_muted_text_style));
    if app.task_board.saved_work_views.is_empty() {
        content =
            content.push(text(i18n.__("暂无保存视图")).size(12).style(settings_muted_text_style));
    }
    for view in &app.task_board.saved_work_views {
        let selected = board.selected_work_view.as_ref() == Some(&view.id);
        content = content.push(
            button(text(&view.name).size(13))
                .padding([8, 10])
                .style(move |theme, status| settings_segment_button_style(theme, status, selected))
                .width(Length::Fill)
                .on_press(Message::TaskBoard(TaskBoardMessage::OpenSavedWorkView(view.id.clone()))),
        );
    }
    content = content.push(text(i18n.__("项目")).size(12).style(settings_muted_text_style));
    let labels = desktop_workspace::projects::recent_project_labels(
        &app.workspace_state.recent_projects,
        &app.workspace_state.recent_projects_meta,
    );
    for (path, label) in app.workspace_state.recent_projects.iter().zip(labels) {
        let current = path == project;
        content = content.push(
            button(
                row![
                    icon_svg(Icon::FolderClosed, 14.0).style(icon_btn_svg_style),
                    container(text(label).size(13).wrapping(text::Wrapping::None))
                        .width(Length::Fill)
                        .clip(true),
                ]
                .spacing(7)
                .align_y(Alignment::Center),
            )
            .padding([8, 10])
            .style(move |theme, status| settings_segment_button_style(theme, status, current))
            .width(Length::Fill)
            .on_press(Message::TaskBoard(TaskBoardMessage::OpenWorkProject(path.clone()))),
        );
    }
    container(
        scrollable(container(content).padding(iced::Padding {
            top: 0.,
            bottom: 0.,
            left: 7.,
            right: 7.,
        }))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        )),
    )
    .width(220)
    .height(Length::Fill)
    .padding([8, 5])
    .style(|theme: &Theme| container::Style {
        background: Some(theme.palette().text.scale_alpha(0.025).into()),
        border: Border {
            color: theme.palette().text.scale_alpha(0.10),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    })
    .into()
}
