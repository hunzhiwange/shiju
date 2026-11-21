use crate::app::components::system_settings_common::{
    icon_svg, rounded_action_btn_style, settings_divider, settings_muted_text_style,
};
use crate::app::i18n::{I18n, tasks::status_label};
use crate::app::message::TaskBoardMessage;
use crate::app::state::TaskBoardDetailTab;
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_task_board::board::{task_label_badge, task_status_marker, task_title};
use desktop_task_board::helpers::{execution::truncate_for_ui, logs::localized_log_message};
use desktop_task_model::{SubTaskStatus, Task, TaskStatus, TaskWorkflow};
use desktop_ui_controls::system_settings_common::{
    icons::icon_btn_svg_style, settings_divider_style,
};
use iced::widget::{
    Column, Space, Svg, Text, button, column, container, keyed_column, responsive, row, scrollable,
    text,
};
use iced::{Alignment, Border, Element, Font, Length, Theme};
use std::hash::{DefaultHasher, Hash, Hasher};

pub(super) fn view<'a>(app: &'a App, task: &'a Task) -> Element<'a, Message> {
    let mut hasher = DefaultHasher::new();
    task.id.hash(&mut hasher);
    keyed_column([(
        hasher.finish(),
        responsive(move |size| layout(app, task, size.width >= 660.0)).into(),
    )])
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn layout<'a>(app: &'a App, task: &'a Task, wide: bool) -> Element<'a, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let workflow = app.task_board.task_board_settings.workflow();
    let project = app.navigation_tabs.project_path.as_deref().unwrap_or("");
    let project_name = if project.is_empty() {
        i18n.__("项目").to_owned()
    } else {
        desktop_workspace::basic::display_name_for_path(
            &app.workspace_state.recent_projects_meta,
            project,
        )
    };
    let header = row![
        action(Icon::ArrowLeft, i18n.__("返回"), TaskBoardMessage::CloseTaskLogs),
        container(
            row![
                icon(Icon::FolderClosed),
                muted(project_name.clone()),
                icon(Icon::ChevronRight),
                muted(&task.id)
            ]
            .spacing(7)
            .align_y(Alignment::Center)
        )
        .width(Length::Fill)
        .clip(true),
        action(
            Icon::Pencil,
            i18n.__("编辑"),
            TaskBoardMessage::SelectDetailTab(TaskBoardDetailTab::Configuration),
        ),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let header = container(header).padding([8, 12]).width(Length::Fill);

    let mut content = column![heading(task_title(task), if wide { 30 } else { 25 }),]
        .spacing(24)
        .width(Length::Fill);
    if !task.prompt.trim().is_empty() {
        content = content.push(
            column![
                heading(i18n.__("任务需求"), 16),
                text(task.prompt.trim())
                    .size(14)
                    .line_height(1.55)
                    .wrapping(text::Wrapping::WordOrGlyph),
            ]
            .spacing(9),
        );
    }
    if !task.subtasks.is_empty() {
        content = content.push(checklist(i18n, task));
    }
    let reason = match task.status {
        TaskStatus::Failed => task.last_error.as_deref(),
        TaskStatus::Paused => task.pause_reason.as_deref(),
        _ => None,
    };
    if let Some(reason) = reason.filter(|reason| !reason.trim().is_empty()) {
        content = content.push(
            container(
                row![
                    task_status_marker(task.status),
                    text(reason).size(13).wrapping(text::Wrapping::WordOrGlyph),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([10, 12])
            .width(Length::Fill)
            .style(surface),
        );
    }
    content = content
        .extend(super::work_plan::detail(app, task))
        .push(settings_divider())
        .push(activity(i18n, task));

    let properties = properties(app, task, project, &project_name, workflow.as_ref());
    let body: Element<'_, Message> = if wide {
        row![
            scroll(
                container(container(content).width(Length::Fill).max_width(820).padding(24))
                    .center_x(Length::Fill)
            ),
            container(Space::new()).width(1).height(Length::Fill).style(settings_divider_style),
            container(scroll(container(properties).padding([18, 12]))).width(250),
        ]
        .height(Length::Fill)
        .into()
    } else {
        scroll(container(content.push(settings_divider()).push(properties)).padding([18, 10]))
            .into()
    };

    container(column![header, settings_divider(), body].height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(surface)
        .clip(true)
        .into()
}

fn surface(theme: &Theme) -> container::Style {
    container::Style {
        border: Border {
            color: theme.palette().text.scale_alpha(0.12),
            width: 1.0,
            radius: 9.0.into(),
        },
        ..Default::default()
    }
}

fn scroll<'a>(content: impl Into<Element<'a, Message>>) -> scrollable::Scrollable<'a, Message> {
    scrollable(container(content).width(Length::Fill).padding([0, 7]))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
}

fn heading<'a>(label: impl Into<String>, size: u16) -> Text<'a> {
    text(label.into())
        .size(f32::from(size))
        .font(Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT })
}

fn muted<'a>(label: impl Into<String>) -> Text<'a> {
    text(label.into()).size(12).style(settings_muted_text_style)
}

fn icon(icon: Icon) -> Svg<'static> {
    icon_svg(icon, 14.0).style(icon_btn_svg_style)
}

fn action<'a>(
    glyph: Icon,
    label: &'a str,
    message: TaskBoardMessage,
) -> button::Button<'a, Message> {
    button(row![icon(glyph), text(label).size(13)].spacing(6).align_y(Alignment::Center))
        .padding([6, 10])
        .style(rounded_action_btn_style)
        .on_press(message.into())
}

fn checklist<'a>(i18n: I18n, task: &'a Task) -> Element<'a, Message> {
    let completed = task.subtasks.iter().filter(|item| item.completed).count();
    let mut items = column![].width(Length::Fill);
    for (index, item) in task.subtasks.iter().enumerate() {
        let status = match item.status {
            _ if item.completed => TaskStatus::Completed,
            SubTaskStatus::Pending => TaskStatus::Pool,
            SubTaskStatus::Running => TaskStatus::Running,
            SubTaskStatus::Completed => TaskStatus::Completed,
            SubTaskStatus::Failed => TaskStatus::Failed,
        };
        if index > 0 {
            items = items.push(settings_divider());
        }
        items = items.push(
            container(
                row![
                    task_status_marker(status),
                    text(&item.content).size(13).width(Length::Fill),
                    muted(status_label(i18n, status))
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .padding([9, 12])
            .width(Length::Fill),
        );
    }
    column![
        row![heading(i18n.__("子任务"), 16), muted(format!("{completed}/{}", task.subtasks.len()))]
            .spacing(9)
            .align_y(Alignment::Center),
        container(items).style(surface),
    ]
    .spacing(9)
    .into()
}

fn time(timestamp: u64) -> String {
    if timestamp == 0 {
        "—".into()
    } else {
        desktop_i18n::chat_time::format_chat_time_label(timestamp)
    }
}

fn activity<'a>(i18n: I18n, task: &'a Task) -> Element<'a, Message> {
    let mut entries = column![].width(Length::Fill);
    for entry in task.logs.iter().rev().take(6) {
        let lead =
            entry.status_to.map_or_else(|| Element::from(icon(Icon::Journals)), task_status_marker);
        entries = entries
            .push(
                container(
                    row![
                        lead,
                        column![
                            muted(time(entry.timestamp_ms)),
                            text(truncate_for_ui(&localized_log_message(i18n, entry), 1000))
                                .size(13)
                                .wrapping(text::Wrapping::WordOrGlyph),
                        ]
                        .width(Length::Fill)
                        .spacing(5),
                    ]
                    .spacing(10),
                )
                .padding([10, 12])
                .width(Length::Fill),
            )
            .push(settings_divider());
    }
    entries = entries.push(
        container(
            row![
                icon(Icon::Clock),
                text(i18n.__("任务创建")).size(13).width(Length::Fill),
                muted(time(task.created_at_ms)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([10, 12])
        .width(Length::Fill),
    );
    column![heading(i18n.__("动态"), 17), container(entries).style(surface),].spacing(10).into()
}

fn field<'a>(
    label: impl Into<String>,
    value: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![muted(label).width(70), value.into()]
        .spacing(8)
        .width(Length::Fill)
        .align_y(Alignment::Center)
        .into()
}

fn properties<'a>(
    app: &'a App,
    task: &'a Task,
    project: &str,
    project_name: &str,
    workflow: Option<&TaskWorkflow>,
) -> Column<'a, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let status = workflow.and_then(|workflow| workflow.status_definition(task.status)).map_or_else(
        || status_label(i18n, task.status),
        |status| {
            if matches!(task.status, TaskStatus::Custom(_)) {
                status.label.clone()
            } else {
                i18n.__(&status.label).to_owned()
            }
        },
    );
    let assignee =
        if task.assignee.trim().is_empty() { i18n.__("未分配") } else { &task.assignee };
    let workflow_name =
        workflow.map_or_else(|| "—".into(), |workflow| i18n.__(workflow.label()).to_owned());
    let mut fields = column![
        heading(i18n.__("属性"), 16),
        field(
            i18n.__("状态"),
            row![task_status_marker(task.status), text(status).size(13)]
                .spacing(7)
                .align_y(Alignment::Center)
        ),
        field(i18n.__("负责人"), text(assignee).size(13)),
        field(i18n.__("项目"), text(project_name.to_owned()).size(13)),
        field(i18n.__("工作流"), text(workflow_name).size(13)),
        field(i18n.__("优先级"), text(format!("P{}", task.priority)).size(13)),
        field(i18n.__("模型"), text(desktop_task_board::model_selector::model_display_label(i18n, &task.model)).size(13)),
    ]
    .spacing(12)
    .width(Length::Fill);
    let labels =
        app.task_board.task_labels.iter().filter(|label| task.label_ids.contains(&label.id));
    if !task.label_ids.is_empty() {
        fields = fields.push(row(labels.map(task_label_badge)).spacing(4).wrap());
    }
    for (name, value) in super::properties::task_summary(app, task) {
        fields = fields.push(field(name, text(value).size(13).width(Length::Fill)));
    }
    let personal =
        app.task_board.personal_tasks.get(project).is_some_and(|ids| ids.contains(&task.id));
    let mut actions = column![heading(i18n.__("快捷操作"), 15)].spacing(7).push(action(
        Icon::CheckSquare,
        i18n.__(if personal { "移出我的任务" } else { "加入我的任务" }),
        TaskBoardMessage::TogglePersonalTask(task.id.clone()),
    ));
    #[cfg(not(target_arch = "wasm32"))]
    {
        actions = actions.extend(super::modals::quick_actions::entry(app, &task.id));
    }
    let logs = action(
        Icon::Journals,
        i18n.__("查看日志"),
        TaskBoardMessage::SelectDetailTab(TaskBoardDetailTab::Logs),
    );
    let mut execution = column![heading(i18n.__("执行日志"), 15), logs].spacing(8);
    if let Some(duration_ms) = task.display_execution_duration_ms(runtime_time::now_ms()) {
        let duration = display_format::format_duration((duration_ms / 1000) as i64);
        let duration = if duration.is_empty() { "0s".into() } else { duration };
        execution = execution.push(field(i18n.__("执行时长"), text(duration).size(13)));
    }
    if let Some(branch) =
        task.selected_worktree_branch.as_deref().filter(|branch| !branch.is_empty())
    {
        execution = execution.push(field(i18n.__("工作树"), text(branch).size(13)));
    }
    execution = execution.extend(super::work_plan::history(app, task));
    column![fields, settings_divider(), actions, settings_divider(), execution]
        .spacing(16)
        .width(Length::Fill)
}
