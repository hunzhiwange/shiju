use crate::app::components::system_settings_common::rounded_action_btn_style;
use crate::app::i18n::{I18n, tasks::status_label};
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};
use desktop_task_board::work_views::BuiltinWorkView;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};

pub fn view(app: &App) -> Element<'_, Message> {
    let project = app.navigation_tabs.project_path.as_deref().unwrap_or("");
    let board = &app.task_board;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let workflow = board.task_board_settings.workflow();
    let mut content = column![text("工作").size(20)].spacing(8);
    if board.task_board_loading {
        content = content.push(text(i18n.__("正在加载任务…")).size(12));
    }
    let plan_open = super::work_plan::current(app).is_some_and(|(_, plan)| plan.open);
    content = content.push(
        button(text(if plan_open { "• 工作计划" } else { "工作计划" }).size(14))
            .padding([8, 10]).style(rounded_action_btn_style).width(Length::Fill)
            .on_press_maybe((!project.is_empty()).then_some(Message::TaskBoard(TaskBoardMessage::OpenWorkPlanView))),
    );
    for (view, label) in [
        (BuiltinWorkView::MyTasks, "我的任务"),
        (BuiltinWorkView::AgentExecuting, "我的智能体在做"),
        (BuiltinWorkView::AllTasks, "全部任务"),
    ] {
        let tasks: Vec<_> = app
            .task_board
            .task_board_tasks
            .iter()
            .filter(|task| app.task_board.work_task_matches(project, view, task))
            .collect();
        let selected = if !plan_open && app.task_board.work_view == view && app.task_board.selected_work_view.is_none() { "• " } else { "" };
        content = content.push(
            button(text(format!("{selected}{label}  {}", tasks.len())).size(14))
                .padding([8, 10])
                .style(rounded_action_btn_style)
                .width(Length::Fill)
                .on_press(Message::TaskBoard(TaskBoardMessage::SelectWorkView(view))),
        );
        if tasks.is_empty() && !board.task_board_loading {
            content = content.push(text("暂无任务").size(12));
        }
        for task in tasks.into_iter().take(5) {
            let preview: String = task.prompt.trim().replace(['\n', '\r'], " ").chars().take(24).collect();
            let title = if preview.is_empty() { &task.id } else { &preview };
            let status = workflow
                .as_ref()
                .and_then(|workflow| workflow.status_definition(task.status))
                .map_or_else(
                    || status_label(i18n, task.status),
                    |status| {
                        if matches!(task.status, crate::app::task::TaskStatus::Custom(_)) {
                            status.label.clone()
                        } else {
                            i18n.__(&status.label).to_owned()
                        }
                    },
                );
            let selected = if board.task_board_editing_task_id.as_ref() == Some(&task.id) {
                "• "
            } else {
                ""
            };
            let detail = button(column![
                text(format!("{selected}{title}")).size(12),
                text(status).size(11),
            ].spacing(2))
            .padding([6, 10])
            .style(rounded_action_btn_style)
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
    content = content
        .push(text("按当前项目工作流的执行中状态筛选").size(11))
        .push(text("全部视图").size(16))
        .push(text("当前项目保存的快捷入口").size(11));
    if app.task_board.saved_work_views.is_empty() {
        content = content.push(text("暂无保存视图").size(12));
    }
    for view in &app.task_board.saved_work_views {
        let selected = if !plan_open && app.task_board.selected_work_view.as_ref() == Some(&view.id) { "• " } else { "" };
        content = content.push(
            button(text(format!("{selected}{}", view.name)).size(13))
                .padding([8, 10]).style(rounded_action_btn_style).width(Length::Fill)
                .on_press(Message::TaskBoard(TaskBoardMessage::OpenSavedWorkView(view.id.clone()))),
        );
    }
    content = content.push(text("项目").size(16));
    let labels = desktop_workspace::projects::recent_project_labels(
        &app.workspace_state.recent_projects,
        &app.workspace_state.recent_projects_meta,
    );
    for (path, label) in app.workspace_state.recent_projects.iter().zip(labels) {
        let current = if path == project { "• " } else { "" };
        content = content.push(
            button(text(format!("{current}{label}")).size(13))
                .padding([8, 10])
                .style(rounded_action_btn_style)
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
    .width(240)
    .height(Length::Fill)
    .into()
}
