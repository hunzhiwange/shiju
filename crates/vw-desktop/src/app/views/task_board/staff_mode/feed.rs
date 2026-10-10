//! 人员编排视图中间任务推进流。

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Color, Element, Length};

use crate::app::assets::Icon;
use crate::app::components::overlays::BelowOverlay;
use crate::app::components::system_settings_common::settings_text_input_style;
use crate::app::message::TaskBoardMessage;
use crate::app::state::TaskBoardStaffFeedTab;
use crate::app::task::TaskStatus;
#[cfg(test)]
use crate::app::task::staff::TaskBoardStaffEventKind;
use crate::app::task::staff::{
    TaskBoardStaffAssignment, TaskBoardStaffCommand, TaskBoardStaffCommandGroup,
    TaskBoardStaffCommandQueue, TaskBoardStaffEvent, TaskBoardStaffMemberLoad,
    TaskBoardStaffProjection,
};
use crate::app::{Message, time};

use super::styles;

const TASK_PREVIEW_CHARS: usize = 120;
const STAFF_MENTION_DROPDOWN_MAX_WIDTH: f32 = 520.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StaffMatrixBridgeSnapshot {
    pub enabled: bool,
    pub cursor: u64,
    pub processed_count: usize,
}

pub(super) fn build_feed_panel<'a>(
    projection: &'a TaskBoardStaffProjection,
    command_queue: &'a TaskBoardStaffCommandQueue,
    _events: &'a [TaskBoardStaffEvent],
    _active_tab: TaskBoardStaffFeedTab,
    filter_member_id: Option<&'a str>,
    status_filters: &'a [TaskStatus],
    command_input: &'a str,
    matrix_bridge: StaffMatrixBridgeSnapshot,
    height: Length,
) -> Element<'a, Message> {
    let filtered_member_name = filter_member_id.and_then(|member_id| {
        projection
            .member_loads
            .iter()
            .find(|member| member.member_id == member_id)
            .map(|member| member.member_name.as_str())
    });
    let now_ms = time::now_ms();
    let status_counts = task_status_counts(projection, filter_member_id);
    let assignments = filtered_assignments(projection, filter_member_id, status_filters);
    let visible_task_count = assignments.len();
    let visible_unassigned_count =
        assignments.iter().filter(|assignment| assignment.member_id.is_none()).count();

    let header_metrics = row![
        styles::small_chip(
            format!("任务 {visible_task_count}/{}", projection.total_task_count),
            styles::priority_color(),
        ),
        matrix_bridge_chip(matrix_bridge),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let header = row![
        column![
            text("人员编排流").size(16).font(styles::bold_font()),
            text(feed_subtitle(filtered_member_name, status_filters))
                .size(11)
                .style(styles::muted_text_style),
        ]
        .spacing(4)
        .width(Length::Fill),
        header_metrics,
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let content = {
        let mut feed = column![].spacing(10).width(Length::Fill);
        if assignments.is_empty() {
            feed = feed.push(empty_state());
        } else {
            for assignment in assignments {
                feed =
                    feed.push(build_assignment_card(assignment, projection, command_queue, now_ms));
            }
        }
        column![
            task_filter_bar(
                filter_member_id,
                filtered_member_name,
                status_filters,
                status_counts,
                visible_task_count,
                visible_unassigned_count,
                now_ms,
            ),
            scrollable(feed)
                .direction(styles::vertical_scrollbar())
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill)
    };

    let body = column![header, build_staff_command_bar(projection, command_input), content]
        .spacing(12)
        .width(Length::Fill)
        .height(Length::Fill);

    container(body)
        .padding(14)
        .width(Length::Fill)
        .height(height)
        .style(styles::center_panel_style)
        .into()
}

fn build_staff_command_bar<'a>(
    projection: &'a TaskBoardStaffProjection,
    command_input: &'a str,
) -> Element<'a, Message> {
    let submit_message = (!command_input.trim().is_empty())
        .then_some(Message::TaskBoard(TaskBoardMessage::SubmitStaffCommandInput));
    let input = text_input("@Orchestrator 执行 T-001", command_input)
        .on_input(|value| Message::TaskBoard(TaskBoardMessage::UpdateStaffCommandInput(value)))
        .on_submit(Message::TaskBoard(TaskBoardMessage::SubmitStaffCommandInput))
        .padding([8, 10])
        .size(12)
        .style(settings_text_input_style)
        .width(Length::Fill);

    let input_row: Element<'_, Message> =
        row![input, action_button(Icon::Terminal, "提交人员编排命令", submit_message),]
            .spacing(8)
            .align_y(Alignment::Center)
            .into();

    let candidates = staff_mention_candidates(projection, command_input);
    let command_input_row: Element<'_, Message> = if candidates.is_empty() {
        input_row
    } else {
        BelowOverlay::new(input_row, build_staff_mention_dropdown(candidates))
            .show(true)
            .gap(4.0)
            .capture_outside_click(false)
            .into()
    };

    let body = column![command_input_row].width(Length::Fill);

    container(body).padding([8, 10]).width(Length::Fill).style(styles::empty_panel_style).into()
}

fn build_staff_mention_dropdown<'a>(
    candidates: Vec<&'a TaskBoardStaffMemberLoad>,
) -> Element<'a, Message> {
    let mut list = column![].spacing(6).width(Length::Fill);
    for member in candidates.into_iter().take(6) {
        list = list.push(staff_mention_candidate_button(member));
    }

    container(list)
        .padding(6)
        .width(Length::Fill)
        .max_width(STAFF_MENTION_DROPDOWN_MAX_WIDTH)
        .style(styles::empty_panel_style)
        .into()
}

fn staff_mention_candidate_button<'a>(
    member: &'a TaskBoardStaffMemberLoad,
) -> Element<'a, Message> {
    let accent = styles::role_color(member.role);
    let status_label = styles::staff_status_label(member.status, member.enabled);
    let load_label = format!("{}/{}", member.load_count, member.capacity);
    let content = row![
        column![
            text(format!("@{}", member.member_name)).size(12).font(styles::medium_font()),
            text(member.member_id.as_str()).size(10).style(styles::muted_text_style),
        ]
        .spacing(2)
        .width(Length::Fill),
        styles::small_chip(styles::role_label(member.role).to_string(), accent),
        styles::small_chip(load_label, styles::priority_color()),
        styles::small_chip(
            status_label.to_string(),
            styles::staff_status_color(member.status, member.enabled)
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    button(content)
        .padding([7, 9])
        .width(Length::Fill)
        .on_press(Message::TaskBoard(TaskBoardMessage::SelectStaffCommandMember {
            member_id: member.member_id.clone(),
        }))
        .style(move |theme, status| {
            styles::assignment_member_button_style(theme, status, false, accent)
        })
        .into()
}

pub(super) fn staff_mention_candidates<'a>(
    projection: &'a TaskBoardStaffProjection,
    command_input: &str,
) -> Vec<&'a TaskBoardStaffMemberLoad> {
    let Some(query) = active_staff_mention_query(command_input) else {
        return Vec::new();
    };
    let query = query.to_ascii_lowercase();
    let mut candidates = projection
        .member_loads
        .iter()
        .filter(|member| member.enabled)
        .filter(|member| {
            query.is_empty()
                || member.member_name.to_ascii_lowercase().contains(&query)
                || member.member_id.to_ascii_lowercase().contains(&query)
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.role
            .key()
            .cmp(right.role.key())
            .then_with(|| left.member_name.cmp(&right.member_name))
            .then_with(|| left.member_id.cmp(&right.member_id))
    });
    candidates
}

fn active_staff_mention_query(command_input: &str) -> Option<&str> {
    let trimmed = command_input.trim_start();
    let at_pos = trimmed.rfind('@')?;
    let after_at = &trimmed[at_pos + '@'.len_utf8()..];
    if after_at.contains(char::is_whitespace) {
        return None;
    }
    Some(after_at)
}

fn matrix_bridge_chip<'a>(matrix_bridge: StaffMatrixBridgeSnapshot) -> Element<'a, Message> {
    let (label, color) = if matrix_bridge.enabled {
        (
            format!(
                "Matrix 已启用 · 游标 {} · 事件 {}",
                matrix_bridge.cursor, matrix_bridge.processed_count
            ),
            Color::from_rgb8(22, 163, 74),
        )
    } else {
        ("本地协同".to_string(), styles::display_only_color())
    };
    button(text(label).size(10))
        .padding([3, 7])
        .style(move |theme, status| styles::chip_button_style(theme, status, color))
        .into()
}

fn filtered_assignments<'a>(
    projection: &'a TaskBoardStaffProjection,
    filter_member_id: Option<&str>,
    status_filters: &[TaskStatus],
) -> Vec<&'a TaskBoardStaffAssignment> {
    let mut assignments = projection
        .lanes
        .iter()
        .flat_map(|lane| lane.assignments.iter())
        .filter(|assignment| {
            filter_member_id
                .is_none_or(|member_id| assignment.member_id.as_deref() == Some(member_id))
                && task_status_filter_matches(status_filters, assignment.task_status)
        })
        .collect::<Vec<_>>();
    assignments.sort_by(|a, b| {
        b.updated_at_ms
            .cmp(&a.updated_at_ms)
            .then_with(|| a.order.cmp(&b.order))
            .then_with(|| a.task_id.cmp(&b.task_id))
    });
    assignments
}

fn task_status_filter_matches(status_filters: &[TaskStatus], status: TaskStatus) -> bool {
    if status_filters.is_empty() {
        return status != TaskStatus::Archived;
    }
    status_filters.contains(&status)
}

fn task_status_counts(
    projection: &TaskBoardStaffProjection,
    filter_member_id: Option<&str>,
) -> Vec<(TaskStatus, usize)> {
    TaskStatus::all()
        .into_iter()
        .map(|status| {
            let count = projection
                .lanes
                .iter()
                .flat_map(|lane| lane.assignments.iter())
                .filter(|assignment| assignment.task_status == status)
                .filter(|assignment| {
                    filter_member_id
                        .is_none_or(|member_id| assignment.member_id.as_deref() == Some(member_id))
                })
                .count();
            (status, count)
        })
        .collect()
}

fn task_status_count(status_counts: &[(TaskStatus, usize)], status: TaskStatus) -> usize {
    status_counts
        .iter()
        .find(|(candidate, _)| *candidate == status)
        .map(|(_, count)| *count)
        .unwrap_or(0)
}

fn non_archived_task_status_count(status_counts: &[(TaskStatus, usize)]) -> usize {
    status_counts
        .iter()
        .filter(|(status, _)| *status != TaskStatus::Archived)
        .map(|(_, count)| *count)
        .sum()
}

fn task_filter_bar<'a>(
    filter_member_id: Option<&'a str>,
    filtered_member_name: Option<&'a str>,
    status_filters: &[TaskStatus],
    status_counts: Vec<(TaskStatus, usize)>,
    visible_task_count: usize,
    visible_unassigned_count: usize,
    now_ms: u64,
) -> Element<'a, Message> {
    let all_count = non_archived_task_status_count(&status_counts);
    let status_buttons = TaskStatus::all().into_iter().fold(
        row![all_status_filter_button(all_count, status_filters.is_empty())]
            .spacing(6)
            .align_y(Alignment::Center),
        |row, status| {
            row.push(status_filter_button(
                status,
                task_status_count(&status_counts, status),
                status_filters.contains(&status),
            ))
        },
    );

    let filter_total_count = if matches!(status_filters, [TaskStatus::Archived]) {
        task_status_count(&status_counts, TaskStatus::Archived)
    } else {
        all_count
    };

    let mut bar = row![
        status_buttons,
        styles::small_chip(
            format!("显示 {visible_task_count}/{filter_total_count}"),
            styles::priority_color()
        ),
        styles::small_chip(format!("未分配 {visible_unassigned_count}"), styles::warning_color()),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if let (Some(member_id), Some(member_name)) = (filter_member_id, filtered_member_name) {
        bar = bar
            .push(styles::small_chip(
                format!("成员 {member_name}"),
                styles::role_color(crate::app::task::staff::TaskBoardStaffRole::Executor),
            ))
            .push(action_button(
                Icon::X,
                "清除成员筛选",
                Some(staff_message(TaskBoardStaffCommand::FilterByMember {
                    command_id: format!("staff-filter-clear-{member_id}-{now_ms}"),
                    member_id: member_id.to_string(),
                })),
            ));
    }

    container(bar.wrap())
        .padding([8, 10])
        .width(Length::Fill)
        .style(styles::empty_panel_style)
        .into()
}

fn all_status_filter_button<'a>(count: usize, selected: bool) -> Element<'a, Message> {
    button(text(format!("全部 {count}")).size(11))
        .padding([6, 9])
        .on_press(Message::TaskBoard(TaskBoardMessage::SelectAllStaffTaskStatuses))
        .style(move |theme, button_status| styles::tab_button_style(theme, button_status, selected))
        .into()
}

fn status_filter_button<'a>(
    status: TaskStatus,
    count: usize,
    selected: bool,
) -> Element<'a, Message> {
    button(text(format!("{} {count}", status.label())).size(11))
        .padding([6, 9])
        .on_press(Message::TaskBoard(TaskBoardMessage::ToggleStaffTaskStatusFilter(status)))
        .style(move |theme, button_status| styles::tab_button_style(theme, button_status, selected))
        .into()
}

fn feed_subtitle(filtered_member_name: Option<&str>, status_filters: &[TaskStatus]) -> String {
    let status_label = staff_status_filter_summary(status_filters);
    match (filtered_member_name, status_filters.is_empty()) {
        (Some(name), true) => format!("按 {name} 筛选当前任务"),
        (Some(name), false) => format!("按 {name} 和 {status_label} 筛选任务"),
        (None, true) => "按最近更新时间展示当前任务".to_string(),
        (None, false) => format!("按 {status_label} 筛选当前任务"),
    }
}

fn staff_status_filter_summary(status_filters: &[TaskStatus]) -> String {
    let labels = status_filters.iter().map(TaskStatus::label).collect::<Vec<_>>();
    match labels.as_slice() {
        [] => "全部类型".to_string(),
        [single] => (*single).to_string(),
        _ => format!("{} 类任务", labels.len()),
    }
}
fn build_assignment_card<'a>(
    assignment: &'a TaskBoardStaffAssignment,
    projection: &'a TaskBoardStaffProjection,
    command_queue: &'a TaskBoardStaffCommandQueue,
    now_ms: u64,
) -> Element<'a, Message> {
    let accent = assignment.role.map(styles::role_color).unwrap_or_else(styles::display_only_color);
    let assignee = assignment.member_name.as_deref().unwrap_or("未分配");
    let role_label = assignment.role.map(styles::role_label).unwrap_or("展示");
    let task_title = task_preview(assignment);
    let updated_label = updated_at_label(now_ms, assignment.updated_at_ms);

    let top_line = row![
        styles::small_chip(assignment.task_status.label().to_string(), accent),
        styles::small_chip(format!("P{}", assignment.priority), styles::priority_color()),
        text(updated_label).size(11).style(styles::muted_text_style),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let member_line = row![
        text(role_label).size(11).style(styles::muted_text_style),
        text("·").size(11).style(styles::muted_text_style),
        text(assignee).size(11).style(styles::muted_text_style),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let mut content = column![
        top_line,
        text(task_title).size(13).width(Length::Fill).font(styles::medium_font()),
        member_line,
    ]
    .spacing(7)
    .width(Length::Fill);

    if let Some(reason) = assignment.block_reason.as_deref() {
        content = content.push(
            container(text(reason).size(11))
                .padding([7, 9])
                .width(Length::Fill)
                .style(styles::warning_panel_style),
        );
    }

    let assignment_queued = command_queue
        .has_active_task_command(&assignment.task_id, TaskBoardStaffCommandGroup::Assignment);
    let start_queued = command_queue
        .has_active_task_command(&assignment.task_id, TaskBoardStaffCommandGroup::Start);
    let pause_resume_queued = command_queue
        .has_active_task_command(&assignment.task_id, TaskBoardStaffCommandGroup::PauseResume);

    let actions = row![
        action_button(
            Icon::Eye,
            "打开任务日志入队",
            assignment
                .actions
                .can_open_logs
                .then(|| staff_message(open_logs_command(assignment, now_ms))),
        ),
        action_button(
            Icon::HandIndex,
            if assignment_queued {
                "派任务或接管已入队"
            } else {
                "派任务或接管入队"
            },
            (!assignment_queued)
                .then(|| assign_or_take_over_message(assignment, projection, now_ms))
                .flatten(),
        ),
        action_button(
            Icon::Terminal,
            if start_queued { "开始任务已入队" } else { "开始任务入队" },
            (!start_queued).then(|| start_message(assignment, now_ms)).flatten(),
        ),
        action_button(
            Icon::EyeSlash,
            if pause_resume_queued { "暂停或继续已入队" } else { "暂停或继续入队" },
            (!pause_resume_queued).then(|| pause_or_resume_message(assignment, now_ms)).flatten(),
        ),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    container(row![content, actions].spacing(12).align_y(Alignment::Center))
        .padding(12)
        .width(Length::Fill)
        .style(move |theme| styles::task_card_style(theme, accent, assignment.blocked))
        .into()
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EventCardContent {
    pub status_label: String,
    pub delivery_label: Option<String>,
    pub meta: String,
    pub message: String,
}

#[cfg(test)]
fn event_card_content(event: &TaskBoardStaffEvent, now_ms: u64) -> EventCardContent {
    match event.kind {
        TaskBoardStaffEventKind::Command => command_event_card_content(event, now_ms),
        TaskBoardStaffEventKind::Message => message_event_card_content(event, now_ms),
    }
}

#[cfg(test)]
pub(super) fn event_card_content_for_tests(
    event: &TaskBoardStaffEvent,
    now_ms: u64,
) -> EventCardContent {
    event_card_content(event, now_ms)
}

#[cfg(test)]
fn command_event_card_content(event: &TaskBoardStaffEvent, now_ms: u64) -> EventCardContent {
    let task_label = event.task_id.as_deref().unwrap_or("全局");
    let member_label = event.member_id.as_deref().unwrap_or("队列");
    let updated_label = updated_at_label(now_ms, event.timestamp_ms);
    let meta = event
        .orchestration
        .as_ref()
        .map(|context| {
            format!(
                "{} · {} · {} · {}",
                context.action_source, context.member_name, context.target, updated_label
            )
        })
        .unwrap_or_else(|| format!("{task_label} · {member_label} · {updated_label}"));

    EventCardContent {
        status_label: event.status.label().to_string(),
        delivery_label: None,
        meta,
        message: event.message.clone(),
    }
}

#[cfg(test)]
fn message_event_card_content(event: &TaskBoardStaffEvent, now_ms: u64) -> EventCardContent {
    let updated_label = updated_at_label(now_ms, event.timestamp_ms);
    let Some(message) = event.staff_message.as_ref() else {
        let task_label = event.task_id.as_deref().unwrap_or("全局");
        let member_label = event.member_id.as_deref().unwrap_or("员工");
        return EventCardContent {
            status_label: event.status.label().to_string(),
            delivery_label: None,
            meta: format!("{task_label} · {member_label} · {updated_label}"),
            message: event.message.clone(),
        };
    };

    EventCardContent {
        status_label: event.status.label().to_string(),
        delivery_label: message.delivery_status.map(|status| status.label().to_string()),
        meta: format!(
            "{} · {} -> {} · {}",
            message.task_id, message.from_member_id, message.to_member_id, updated_label
        ),
        message: message.body.clone(),
    }
}

fn empty_state<'a>() -> Element<'a, Message> {
    container(
        column![
            text("暂无编排任务").size(14).font(styles::bold_font()),
            text("切到人员编排视图后，任务池投影会在这里形成推进流。")
                .size(11)
                .style(styles::muted_text_style),
        ]
        .spacing(6)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(styles::empty_panel_style)
    .into()
}

fn open_logs_command(assignment: &TaskBoardStaffAssignment, now_ms: u64) -> TaskBoardStaffCommand {
    TaskBoardStaffCommand::OpenTaskLogs {
        command_id: command_id("open_logs", assignment, None, now_ms),
        task_id: assignment.task_id.clone(),
    }
}

fn assign_or_take_over_message(
    assignment: &TaskBoardStaffAssignment,
    projection: &TaskBoardStaffProjection,
    now_ms: u64,
) -> Option<Message> {
    if !assignment.actions.can_assign {
        return None;
    }
    if let Some(member_id) = assignment.member_id.clone() {
        return Some(staff_message(TaskBoardStaffCommand::TakeOverTask {
            command_id: command_id("take_over", assignment, Some(&member_id), now_ms),
            task_id: assignment.task_id.clone(),
            member_id,
        }));
    }

    let role = assignment.role?;
    let member_id = projection
        .member_loads
        .iter()
        .find(|member| {
            member.role == role && member.enabled && member.load_count < member.capacity as usize
        })
        .map(|member| member.member_id.clone())?;
    Some(staff_message(TaskBoardStaffCommand::AssignTask {
        command_id: command_id("assign", assignment, Some(&member_id), now_ms),
        task_id: assignment.task_id.clone(),
        member_id,
    }))
}

fn start_message(assignment: &TaskBoardStaffAssignment, now_ms: u64) -> Option<Message> {
    if !assignment.actions.can_start {
        return None;
    }
    let member_id = assignment.member_id.clone()?;
    Some(staff_message(TaskBoardStaffCommand::StartTask {
        command_id: command_id("start", assignment, Some(&member_id), now_ms),
        task_id: assignment.task_id.clone(),
        member_id,
    }))
}

fn pause_or_resume_message(assignment: &TaskBoardStaffAssignment, now_ms: u64) -> Option<Message> {
    let member_id = assignment.member_id.clone()?;
    if assignment.actions.can_pause {
        return Some(staff_message(TaskBoardStaffCommand::PauseTask {
            command_id: command_id("pause", assignment, Some(&member_id), now_ms),
            task_id: assignment.task_id.clone(),
            member_id,
        }));
    }
    if assignment.actions.can_resume {
        return Some(staff_message(TaskBoardStaffCommand::ResumeTask {
            command_id: command_id("resume", assignment, Some(&member_id), now_ms),
            task_id: assignment.task_id.clone(),
            member_id,
        }));
    }
    None
}

fn staff_message(command: TaskBoardStaffCommand) -> Message {
    Message::TaskBoard(TaskBoardMessage::SubmitStaffCommand(command))
}

fn command_id(
    action: &str,
    assignment: &TaskBoardStaffAssignment,
    member_id: Option<&str>,
    now_ms: u64,
) -> String {
    format!("staff-{action}-{}-{}-{now_ms}", assignment.task_id, member_id.unwrap_or("global"))
}

fn action_button<'a>(
    icon: Icon,
    tooltip_label: &'a str,
    message: Option<Message>,
) -> Element<'a, Message> {
    let enabled = message.is_some();
    let icon = styles::icon(icon, 13.0, enabled);
    let button = button(icon)
        .padding(6)
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .style(styles::icon_button_style);
    let button = if let Some(message) = message { button.on_press(message) } else { button };

    styles::tooltip(button.into(), tooltip_label)
}

fn task_preview(assignment: &TaskBoardStaffAssignment) -> String {
    let value = assignment.task_description.trim();
    let text = if value.is_empty() { assignment.task_id.as_str() } else { value };
    let normalized = text.replace(['\n', '\r'], " ");
    let compact = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= TASK_PREVIEW_CHARS {
        return compact;
    }
    let mut clipped = compact.chars().take(TASK_PREVIEW_CHARS).collect::<String>();
    clipped.push('…');
    clipped
}

fn updated_at_label(now_ms: u64, updated_at_ms: u64) -> String {
    let elapsed_ms = now_ms.saturating_sub(updated_at_ms);
    let elapsed_secs = elapsed_ms / 1000;
    if elapsed_secs < 60 {
        return "刚刚更新".to_string();
    }
    let elapsed_minutes = elapsed_secs / 60;
    if elapsed_minutes < 60 {
        return format!("{elapsed_minutes} 分钟前");
    }
    let elapsed_hours = elapsed_minutes / 60;
    if elapsed_hours < 24 {
        return format!("{elapsed_hours} 小时前");
    }
    format!("{} 天前", elapsed_hours / 24)
}

#[cfg(test)]
#[path = "feed_tests.rs"]
mod feed_tests;
