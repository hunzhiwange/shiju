use crate::app::components::system_settings_common::rounded_action_btn_style;
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};
use desktop_task_board::work_views::BuiltinWorkView;
use iced::widget::{button, column, container, scrollable, text};
use iced::{Element, Length};

pub fn view(app: &App) -> Element<'_, Message> {
    let project = app.navigation_tabs.project_path.as_deref().unwrap_or("");
    let mut content = column![text("工作").size(20)].spacing(8);
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
        let selected = if app.task_board.work_view == view { "• " } else { "" };
        content = content.push(
            button(text(format!("{selected}{label}  {}", tasks.len())).size(14))
                .padding([8, 10])
                .style(rounded_action_btn_style)
                .width(Length::Fill)
                .on_press(Message::TaskBoard(TaskBoardMessage::SelectWorkView(view))),
        );
        if tasks.is_empty() {
            content = content.push(text("暂无任务").size(12));
        }
        for task in tasks.into_iter().take(5) {
            let preview: String = task.prompt.replace(['\n', '\r'], " ").chars().take(36).collect();
            content = content.push(
                button(text(format!("{} {}", task.id, preview)).size(12))
                    .padding([6, 10])
                    .style(rounded_action_btn_style)
                    .width(Length::Fill)
                    .on_press(Message::TaskBoard(TaskBoardMessage::ViewTaskLogs(task.id.clone()))),
            );
        }
    }
    content = content
        .push(text("按当前项目工作流的执行中状态筛选").size(11))
        .push(text("全部视图").size(16))
        .push(text("暂无保存视图").size(12))
        .push(text("项目").size(16));
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
