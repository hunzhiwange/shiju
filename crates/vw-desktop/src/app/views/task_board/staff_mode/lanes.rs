//! 员工模式左侧工作流阶段列表。

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::app::Message;
use crate::app::message::TaskBoardMessage;
use crate::app::task::TaskStatus;
use crate::app::task::staff::{TaskBoardStaffConfig, TaskBoardStaffLane, TaskBoardStaffProjection};

use super::styles;

pub(super) fn build_lanes_panel<'a>(
    projection: &'a TaskBoardStaffProjection,
    config: &'a TaskBoardStaffConfig,
    load_error: Option<&'a str>,
    status_filters: &'a [TaskStatus],
    width: Length,
    height: Length,
) -> Element<'a, Message> {
    let header = column![
        text("工作流阶段").size(15).font(styles::bold_font()),
        text(format!(
            "任务 {} · 阻塞 {} · 未分配 {}",
            projection.total_task_count,
            projection.blocked_task_count,
            projection.unassigned.len()
        ))
        .size(11)
        .style(styles::muted_text_style)
    ]
    .spacing(4);

    let mut body = column![header].spacing(10).width(Length::Fill);
    if let Some(error) = load_error {
        body = body.push(
            container(text(format!("员工配置加载失败：{error}")).size(11))
                .padding([8, 10])
                .width(Length::Fill)
                .style(styles::warning_panel_style),
        );
    }

    let mut stage_list = column![].spacing(8).width(Length::Fill);
    for lane in &projection.lanes {
        stage_list =
            stage_list.push(build_lane_item(lane, config, status_filters == [lane.status]));
    }

    body = body.push(
        scrollable(stage_list)
            .direction(styles::vertical_scrollbar())
            .width(Length::Fill)
            .height(Length::Fill),
    );

    container(body).padding(12).width(width).height(height).style(styles::side_panel_style).into()
}

fn build_lane_item<'a>(
    lane: &'a TaskBoardStaffLane,
    config: &'a TaskBoardStaffConfig,
    selected: bool,
) -> Element<'a, Message> {
    let accent = lane.role.map(styles::role_color).unwrap_or_else(styles::display_only_color);
    let role_label = lane.role.map(styles::role_label).unwrap_or("展示");
    let state_label = if lane.display_only { "只读" } else { role_label };
    let stage_assignment =
        config.stage_assignments.iter().find(|assignment| assignment.status == lane.status);

    let header = row![
        container(text(lane.status.label()).size(12).font(styles::bold_font())).width(Length::Fill),
        styles::count_chip(lane.task_count.to_string(), accent)
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut meta = row![
        styles::small_chip(state_label.to_string(), accent),
        styles::small_chip(format!("阻塞 {}", lane.blocked_count), styles::danger_color()),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    if lane.blocked_count == 0 {
        meta = row![styles::small_chip(state_label.to_string(), accent)]
            .spacing(6)
            .align_y(Alignment::Center);
    }
    let mut content = column![header, meta].spacing(8);
    if let Some(assignment) = stage_assignment {
        let primary_count = usize::from(assignment.primary_member_id.is_some());
        let ownership = row![
            styles::small_chip(format!("主责 {primary_count}"), styles::priority_color(),),
            styles::small_chip(
                format!("候补 {}", assignment.backup_member_ids.len()),
                styles::staff_level_color(crate::app::task::staff::TaskBoardStaffLevel::Executor),
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        let mut dispatch = row![].spacing(6).align_y(Alignment::Center);
        if !assignment.manager_member_ids.is_empty() {
            dispatch = dispatch.push(styles::small_chip(
                format!("管理 {}", assignment.manager_member_ids.len()),
                styles::warning_color(),
            ));
        }
        let autonomous_label = if assignment.autonomous { "自主开" } else { "手动" };
        dispatch = dispatch.push(styles::small_chip(
            autonomous_label.to_string(),
            if assignment.autonomous {
                styles::priority_color()
            } else {
                styles::display_only_color()
            },
        ));
        content = content.push(ownership).push(dispatch);
    }

    let card = container(content).padding([10, 11]).width(Length::Fill).style(move |theme| {
        styles::lane_card_style(theme, accent, selected || lane.task_count > 0)
    });

    button(card)
        .padding(0)
        .width(Length::Fill)
        .on_press(Message::TaskBoard(TaskBoardMessage::SelectStaffTaskStatusTab(lane.status)))
        .style(move |theme, status| styles::tab_button_style(theme, status, selected))
        .into()
}

#[cfg(test)]
#[path = "lanes_tests.rs"]
mod lanes_tests;
