//! 人员编排视图右侧成员名册与负载。

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::app::Message;
use crate::app::assets::Icon;
use crate::app::message::{TaskBoardMessage, ViewMessage};
use crate::app::task::staff::{
    TaskBoardStaffCommand, TaskBoardStaffMemberLoad, TaskBoardStaffProjection, TaskBoardStaffRole,
};
use crate::app::time;

use super::styles;

const ROSTER_BOTTOM_SPACER: f32 = 24.0;

pub(super) fn build_roster_panel<'a>(
    projection: &'a TaskBoardStaffProjection,
    selected_member_id: Option<&'a str>,
    width: Length,
    height: Length,
) -> Element<'a, Message> {
    let header = row![
        column![
            text("成员名册").size(15).font(styles::bold_font()),
            text(format!(
                "{} 名成员 · {} 个阻塞任务",
                projection.member_loads.len(),
                projection.blocked_task_count
            ))
            .size(11)
            .style(styles::muted_text_style),
        ]
        .spacing(4)
        .width(Length::Fill),
        action_button(
            Icon::ArrowRepeat,
            "刷新人员编排配置",
            Some(Message::TaskBoard(TaskBoardMessage::ReloadStaffConfig)),
        ),
        action_button(
            Icon::GearWideConnected,
            "打开工作流编排",
            Some(Message::View(ViewMessage::OpenWorkflowManagerStageActions)),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut groups = column![].spacing(12).width(Length::Fill);
    let now_ms = time::now_ms();
    for role in TaskBoardStaffRole::all() {
        let members =
            projection.member_loads.iter().filter(|member| member.role == role).collect::<Vec<_>>();
        if !members.is_empty() {
            groups = groups.push(build_role_group(role, &members, selected_member_id, now_ms));
        }
    }
    if !projection.member_loads.is_empty() {
        groups = groups.push(Space::new().height(Length::Fixed(ROSTER_BOTTOM_SPACER)));
    }

    let body = column![
        header,
        scrollable(groups)
            .direction(styles::vertical_scrollbar())
            .width(Length::Fill)
            .height(Length::Fill),
    ]
    .spacing(12)
    .width(Length::Fill)
    .height(Length::Fill);

    container(body).padding(12).width(width).height(height).style(styles::side_panel_style).into()
}

fn build_role_group<'a>(
    role: TaskBoardStaffRole,
    members: &[&'a TaskBoardStaffMemberLoad],
    selected_member_id: Option<&'a str>,
    now_ms: u64,
) -> Element<'a, Message> {
    let accent = styles::role_color(role);
    let mut group = column![
        row![
            text(styles::role_label(role)).size(11).font(styles::bold_font()).width(Length::Fill),
            styles::count_chip(members.len().to_string(), accent),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
    ]
    .spacing(8)
    .width(Length::Fill);

    for member in members {
        let selected = selected_member_id == Some(member.member_id.as_str());
        group = group.push(build_member_card(member, accent, selected, now_ms));
    }

    group.into()
}

fn build_member_card<'a>(
    member: &'a TaskBoardStaffMemberLoad,
    accent: iced::Color,
    selected: bool,
    now_ms: u64,
) -> Element<'a, Message> {
    let status_color = styles::staff_status_color(member.status, member.enabled);
    let capacity_label = format!("容量 {}/{}", member.load_count, member.capacity);
    let task_label = if member.task_ids.is_empty() {
        "当前无任务".to_string()
    } else {
        format!("任务 {}", member.task_ids.join(", "))
    };
    let binding = member
        .acp_agent
        .as_deref()
        .or(member.delegate_agent.as_deref())
        .or_else(|| member.executor_backend.as_ref().map(|backend| backend.label()))
        .unwrap_or("未绑定 agent");

    let name_row = row![
        styles::status_dot(status_color),
        text(&member.member_name)
            .size(13)
            .font(styles::medium_font())
            .width(Length::Fill)
            .wrapping(Wrapping::WordOrGlyph),
        styles::small_chip(capacity_label, accent),
        action_button(
            Icon::Search,
            if selected { "取消成员筛选" } else { "仅筛选该成员任务" },
            member.enabled.then(|| filter_message(member, now_ms)),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut detail = column![
        name_row,
        row![
            text(styles::staff_status_label(member.status, member.enabled))
                .size(11)
                .style(styles::muted_text_style),
            text("·").size(11).style(styles::muted_text_style),
            text(binding)
                .size(11)
                .style(styles::muted_text_style)
                .width(Length::Fill)
                .wrapping(Wrapping::WordOrGlyph),
        ]
        .spacing(4)
        .width(Length::Fill)
        .align_y(Alignment::Center),
        text(task_label)
            .size(11)
            .style(styles::muted_text_style)
            .width(Length::Fill)
            .wrapping(Wrapping::WordOrGlyph),
    ]
    .spacing(7)
    .width(Length::Fill);

    if member.over_capacity {
        detail = detail.push(
            container(text("负载超过容量").size(11))
                .padding([6, 8])
                .style(styles::warning_panel_style),
        );
    } else if !member.blocked_task_ids.is_empty() {
        detail = detail.push(
            container(text(format!("阻塞 {}", member.blocked_task_ids.len())).size(11))
                .padding([6, 8])
                .style(styles::warning_panel_style),
        );
    }

    container(detail)
        .padding(10)
        .width(Length::Fill)
        .style(move |theme| styles::member_card_style(theme, accent, member.enabled))
        .into()
}

fn filter_message(member: &TaskBoardStaffMemberLoad, now_ms: u64) -> Message {
    Message::TaskBoard(TaskBoardMessage::SubmitStaffCommand(
        TaskBoardStaffCommand::FilterByMember {
            command_id: format!("staff-filter-{}-{now_ms}", member.member_id),
            member_id: member.member_id.clone(),
        },
    ))
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

#[cfg(test)]
#[path = "roster_tests.rs"]
mod roster_tests;
