use super::{
    BoardView, build_task_card, kanban_board, kanban_columns, now_ms, sort_task_cards,
    task_status_marker, task_title, with_task_menu,
};
use crate::message::TaskBoardMessage;
use crate::work_views::{
    LaneGrouping, TaskBoardViewMode, WorkDisplay, WorkDisplayGroup, WorkPlanView, WorkSort,
};
use desktop_i18n::{I18n, chat_time::format_chat_time_label};
use desktop_task_model::{TASK_AGENT_MAIN, Task, TaskStatus, normalize_task_acp_agent_input};
use desktop_ui_controls::system_settings_common::{
    settings_muted_text_style, settings_segment_button_style,
};
use iced::widget::{Column, Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Border, Element, Font, Length, Theme};
use std::collections::{BTreeMap, HashMap, HashSet};
use vw_api_agent_scheduler::AgentSchedulerTaskPlanPlacementDto::Child;

const COLUMN_WIDTH: f32 = 240.0;
const COLUMN_GAP: f32 = 12.0;

pub struct DisplayView<'a> {
    pub mode: TaskBoardViewMode,
    pub options: WorkDisplay,
    pub collapsed: &'a HashSet<WorkDisplayGroup>,
    pub plan: Option<&'a WorkPlanView>,
    pub agent_labels: HashMap<&'a str, &'a str>,
    pub allow_legacy_reorder: bool,
}

pub fn build_work_display<'a, M: Clone + From<TaskBoardMessage> + 'a>(
    app: &BoardView<'a, M>,
    display: DisplayView<'a>,
) -> Element<'a, M> {
    let statuses = kanban_columns(app);
    let status_keys = statuses.iter().map(|column| column.status).collect::<HashSet<_>>();
    let mut tasks = app
        .task_board_tasks
        .iter()
        .filter(|task| {
            !task.deleted
                && !task.archived
                && status_keys.contains(&task.status)
                && app.task_visible.as_ref().is_none_or(|visible| visible(task))
        })
        .collect::<Vec<_>>();
    sort_tasks(&mut tasks, display.options);

    match display.mode {
        TaskBoardViewMode::Kanban => kanban_board(
            app,
            Some(&tasks),
            display.options.hide_empty_statuses,
            display.allow_legacy_reorder,
        ),
        TaskBoardViewMode::List => rows_view(app, &display, &tasks, false),
        TaskBoardViewMode::Table => rows_view(app, &display, &tasks, true),
        TaskBoardViewMode::Swimlane => swimlanes(app, &display, &tasks),
    }
}

fn sort_tasks(tasks: &mut [&Task], options: WorkDisplay) {
    sort_task_cards(tasks);
    match options.sort {
        WorkSort::Created => {}
        WorkSort::Updated => tasks.sort_by_key(|task| std::cmp::Reverse(updated_at(task))),
        WorkSort::Priority => tasks.sort_by_key(|task| std::cmp::Reverse(task.priority)),
    }
    if options.ascending {
        tasks.reverse();
    }
}

fn updated_at(task: &Task) -> u64 {
    if task.updated_at_ms == 0 { task.created_at_ms } else { task.updated_at_ms }
}

pub(super) fn scrollbar() -> scrollable::Scrollbar {
    scrollable::Scrollbar::new().width(4).scroller_width(4)
}

pub(super) fn well_style(theme: &Theme, drop_target: bool) -> container::Style {
    container::Style {
        background: Some(theme.palette().text.scale_alpha(0.035).into()),
        border: Border {
            color: if drop_target {
                theme.palette().primary.scale_alpha(0.75)
            } else {
                theme.palette().text.scale_alpha(0.10)
            },
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn empty<'a, M: 'a>(locale: config_ui::DesktopLocale) -> Element<'a, M> {
    container(text(I18n::new(locale).__("无任务")).size(13).style(settings_muted_text_style))
        .center_x(Length::Fill)
        .padding(22)
        .into()
}

fn strong() -> Font {
    Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT }
}

fn status_field<'a, M: 'a>(status: TaskStatus, label: String, heading: bool) -> Element<'a, M> {
    row![
        task_status_marker(status),
        text(label)
            .size(13)
            .font(if heading { strong() } else { Font::DEFAULT })
            .wrapping(text::Wrapping::None),
    ]
    .spacing(7)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn status_heading<'a, M: 'a>(
    status: TaskStatus,
    label: String,
    count: usize,
) -> Element<'a, M> {
    row![
        status_field(status, label, true),
        text(count.to_string()).size(12).style(settings_muted_text_style),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn group_button<'a, M: Clone + From<TaskBoardMessage> + 'a>(
    content: Element<'a, M>,
    group: WorkDisplayGroup,
    collapsed: bool,
) -> Element<'a, M> {
    button(
        row![text(if collapsed { "▸" } else { "▾" }).size(13), content]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([8, 10])
    .style(|theme, status| settings_segment_button_style(theme, status, false))
    .on_press(M::from(TaskBoardMessage::ToggleWorkDisplayGroup(group)))
    .into()
}

fn parent_id<'a>(plan: Option<&'a WorkPlanView>, task: &Task) -> Option<&'a str> {
    match plan?.placement(&task.id)? {
        Child { parent_task_id, .. } => Some(parent_task_id),
        _ => None,
    }
}

fn table_order<'a>(tasks: &[&'a Task], plan: Option<&WorkPlanView>) -> Vec<(&'a Task, usize)> {
    let ids = tasks.iter().map(|task| task.id.as_str()).collect::<HashSet<_>>();
    let mut roots = Vec::new();
    let mut children = HashMap::<&str, Vec<&Task>>::new();
    for &task in tasks {
        if let Some(parent) = parent_id(plan, task).filter(|id| ids.contains(id)) {
            children.entry(parent).or_default().push(task);
        } else {
            roots.push(task);
        }
    }

    let mut traversal = Vec::with_capacity(tasks.len());
    let mut seen = HashSet::new();
    let mut stack = Vec::new();
    // Start detached or cyclic groups as roots too, so a stale placement cannot hide a task.
    for task in roots.into_iter().chain(tasks.iter().copied()) {
        stack.push((task, 0));
        while let Some((task, level)) = stack.pop() {
            if seen.insert(task.id.as_str()) {
                traversal.push((task, level));
                if let Some(children) = children.get(task.id.as_str()) {
                    stack.extend(children.iter().rev().map(|child| (*child, level + 1)));
                }
            }
        }
    }
    traversal
}

fn table_style(theme: &Theme, header: bool) -> container::Style {
    container::Style {
        background: header.then(|| theme.palette().text.scale_alpha(0.055).into()),
        border: Border {
            color: theme.palette().text.scale_alpha(0.13),
            width: 0.5,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn cell<'a, M: 'a>(value: impl Into<Element<'a, M>>, portion: u16, header: bool) -> Element<'a, M> {
    container(value)
        .width(Length::FillPortion(portion))
        .height(38)
        .align_y(Alignment::Center)
        .padding([0, 10])
        .clip(true)
        .style(move |theme| table_style(theme, header))
        .into()
}

fn rows_view<'a, M: Clone + From<TaskBoardMessage> + 'a>(
    app: &BoardView<'a, M>,
    display: &DisplayView<'a>,
    tasks: &[&'a Task],
    table: bool,
) -> Element<'a, M> {
    let i18n = I18n::new(app.desktop_locale);
    let columns = kanban_columns(app);
    let labels = columns
        .iter()
        .map(|column| {
            let label = if matches!(column.status, TaskStatus::Custom(_)) {
                column.label.clone()
            } else {
                i18n.__(&column.label).to_owned()
            };
            (column.status, label)
        })
        .collect::<HashMap<_, _>>();
    let mut body = column![].width(Length::Fill).spacing(if table { 0 } else { 4 });
    let grouped = !table || display.options.table_group_status;
    if grouped {
        for column in &columns {
            let members = tasks
                .iter()
                .copied()
                .filter(|task| task.status == column.status)
                .collect::<Vec<_>>();
            if display.options.hide_empty_statuses && members.is_empty() {
                continue;
            }
            let group = WorkDisplayGroup::Status(column.status);
            let collapsed = display.collapsed.contains(&group);
            body = body.push(group_button(
                status_heading(column.status, labels[&column.status].clone(), members.len()),
                group,
                collapsed,
            ));
            if !collapsed {
                body = append_rows(body, app, display, &labels, &members, table);
            }
        }
        if tasks.is_empty() && (display.options.hide_empty_statuses || columns.is_empty()) {
            body = body.push(empty(app.desktop_locale));
        }
    } else {
        body = append_rows(body, app, display, &labels, tasks, table);
    }
    let body = scrollable(container(body).width(Length::Fill).padding([0, 7]))
        .direction(scrollable::Direction::Vertical(scrollbar()))
        .width(Length::Fill)
        .height(Length::Fill);
    if !table {
        return body.into();
    }
    let heading = |label| text(i18n.__(label)).size(12).font(strong());
    column![
        container(row![
            cell(heading("任务"), 8, true),
            cell(heading("状态"), 3, true),
            cell(heading("优先级"), 2, true),
            cell(heading("负责人"), 3, true),
            cell(heading("更新时间"), 4, true),
        ])
        .padding([0, 7]),
        body,
    ]
    .into()
}

fn append_rows<'a, M: Clone + From<TaskBoardMessage> + 'a>(
    mut body: Column<'a, M>,
    app: &BoardView<'a, M>,
    display: &DisplayView<'a>,
    labels: &HashMap<TaskStatus, String>,
    tasks: &[&'a Task],
    table: bool,
) -> Column<'a, M> {
    if tasks.is_empty() {
        return body.push(empty(app.desktop_locale));
    }
    let rows = if table && display.options.table_hierarchy {
        table_order(tasks, display.plan)
    } else {
        tasks.iter().map(|&task| (task, 0)).collect()
    };
    for (task, depth) in rows {
        let assignee = if task.assignee.trim().is_empty() { "—" } else { &task.assignee };
        let title = text(task_title(task)).size(13).wrapping(text::Wrapping::None);
        let id =
            text(&task.id).size(11).wrapping(text::Wrapping::None).style(settings_muted_text_style);
        let content: Element<'_, M> = if table {
            let updated = match updated_at(task) {
                0 => "—".to_owned(),
                timestamp => format_chat_time_label(timestamp),
            };
            row![
                cell(
                    row![
                        Space::new().width((depth.min(5) * 12) as f32),
                        container(id).width(78).clip(true),
                        container(title).width(Length::Fill).clip(true),
                    ]
                    .spacing(7)
                    .align_y(Alignment::Center),
                    8,
                    false,
                ),
                cell(status_field(task.status, labels[&task.status].clone(), false), 3, false),
                cell(text(format!("P{}", task.priority)).size(12), 2, false),
                cell(text(assignee).size(12).wrapping(text::Wrapping::None), 3, false),
                cell(text(updated).size(12).wrapping(text::Wrapping::None), 4, false),
            ]
            .into()
        } else {
            row![
                container(id).width(90).clip(true),
                container(title).width(Length::Fill).clip(true),
                text(format!("P{}", task.priority)).size(11).style(settings_muted_text_style),
                container(text(assignee).size(12).wrapping(text::Wrapping::None))
                    .width(94)
                    .clip(true),
            ]
            .spacing(10)
            .padding([7, 10])
            .align_y(Alignment::Center)
            .into()
        };
        let row = button(content)
            .padding(0)
            .width(Length::Fill)
            .style(|theme, status| {
                let mut style = button::text(theme, status);
                style.background = match status {
                    button::Status::Hovered => {
                        Some(theme.palette().primary.scale_alpha(0.06).into())
                    }
                    button::Status::Pressed => {
                        Some(theme.palette().primary.scale_alpha(0.12).into())
                    }
                    _ => None,
                };
                style
            })
            .on_press(M::from(TaskBoardMessage::EditTask(task.id.clone())));
        body =
            body.push(with_task_menu(app, task, row.into(), app.task_board_context_menu.as_ref()));
    }
    body
}

fn lane_identity(display: &DisplayView<'_>, task: &Task, i18n: I18n) -> (String, String) {
    match display.options.lane_grouping {
        LaneGrouping::Assignee => {
            let name = task.assignee.trim();
            let label = if name.is_empty() { i18n.__("未分配") } else { name };
            (name.to_owned(), label.to_owned())
        }
        LaneGrouping::Agent => {
            if let Some(acp) = task.acp_agent.as_deref().and_then(normalize_task_acp_agent_input) {
                return (format!("acp:{acp}"), crate::draft::task_acp_agent_label(Some(&acp)));
            }
            let agent =
                task.agent.as_deref().filter(|agent| !agent.is_empty()).unwrap_or(TASK_AGENT_MAIN);
            let label = display
                .agent_labels
                .get(agent)
                .copied()
                .filter(|label| !label.is_empty())
                .unwrap_or(agent);
            (format!("builtin:{agent}"), label.to_owned())
        }
    }
}

fn swimlanes<'a, M: Clone + From<TaskBoardMessage> + 'a>(
    app: &BoardView<'a, M>,
    display: &DisplayView<'a>,
    tasks: &[&'a Task],
) -> Element<'a, M> {
    if tasks.is_empty() {
        return empty(app.desktop_locale);
    }
    let i18n = I18n::new(app.desktop_locale);
    let statuses = kanban_columns(app);
    let mut lanes = BTreeMap::<_, Vec<_>>::new();
    let mut counts = HashMap::<_, usize>::new();
    for &task in tasks {
        let (id, label) = lane_identity(display, task, i18n);
        lanes.entry((label, id)).or_default().push(task);
        *counts.entry(task.status).or_default() += 1;
    }
    let dragging = app.task_board_dragging.as_ref();
    let dragging_lane = dragging.and_then(|(id, _)| {
        tasks.iter().find(|task| &task.id == id).map(|task| lane_identity(display, task, i18n).0)
    });
    let columns = statuses
        .iter()
        .filter(|column| {
            !display.options.hide_empty_statuses
                || dragging.is_some()
                || counts.contains_key(&column.status)
        })
        .collect::<Vec<_>>();
    let now = now_ms();
    let mut headings = row![].spacing(COLUMN_GAP);
    for column in &columns {
        let label = if matches!(column.status, TaskStatus::Custom(_)) {
            column.label.clone()
        } else {
            i18n.__(&column.label).to_owned()
        };
        headings = headings.push(
            container(status_heading(
                column.status,
                label,
                counts.get(&column.status).copied().unwrap_or(0),
            ))
            .width(COLUMN_WIDTH)
            .padding([8, 10])
            .style(|theme| well_style(theme, false))
            .clip(true),
        );
    }
    let width =
        COLUMN_WIDTH * columns.len() as f32 + COLUMN_GAP * columns.len().saturating_sub(1) as f32;
    let mut board = column![headings].width(width).spacing(8);
    for ((label, id), members) in lanes {
        let group = WorkDisplayGroup::Lane(display.options.lane_grouping, id.clone());
        let collapsed = display.collapsed.contains(&group);
        board = board.push(group_button(
            row![
                text(label).size(13).font(strong()),
                text(members.len().to_string()).size(12).style(settings_muted_text_style),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
            group,
            collapsed,
        ));
        if collapsed {
            continue;
        }
        let mut cells = row![].spacing(COLUMN_GAP);
        for column in &columns {
            let status = column.status;
            let target = dragging_lane.as_ref() == Some(&id)
                && dragging.is_some_and(|(_, from)| *from != status);
            let mut cards = column![].spacing(7).width(Length::Fill);
            let mut has_cards = false;
            for &task in members.iter().filter(|task| task.status == status) {
                has_cards = true;
                cards = cards.push(build_task_card(
                    app,
                    task,
                    status,
                    dragging.is_some_and(|(id, _)| id == &task.id),
                    dragging.is_some(),
                    app.cursor_position,
                    (*app.task_board_context_menu).clone(),
                    now,
                ));
            }
            if !has_cards {
                cards = cards.push(empty(app.desktop_locale));
            }
            let cell = container(cards)
                .width(COLUMN_WIDTH)
                .height(Length::Fill)
                .padding(8)
                .style(move |theme| well_style(theme, target));
            let cell: Element<'_, M> = if target {
                iced::widget::mouse_area(cell)
                    .on_release(M::from(TaskBoardMessage::DropOnStatus {
                        to_status: status,
                        insert_index: None,
                    }))
                    .into()
            } else {
                cell.into()
            };
            cells = cells.push(cell);
        }
        board = board.push(cells);
    }
    scrollable(container(board).padding([0, 7]))
        .direction(scrollable::Direction::Both { vertical: scrollbar(), horizontal: scrollbar() })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
