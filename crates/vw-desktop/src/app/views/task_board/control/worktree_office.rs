//! Worktree 像素办公室视图。
//!
//! 本模块专注于将 Worktree 池快照渲染为像素办公室场景，
//! 包含单工位房间、走廊、楼层布局以及统计摘要。

mod floor;
mod room_scene;
mod summary;

use iced::widget::{column, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::task::{TaskStatus, WorktreePoolSnapshot, WorktreeSlotSnapshot, WorktreeState};
use crate::app::{App, Message};

use self::floor::build_worktree_pixel_floor;
use self::summary::{build_task_status_summary_strip, build_worktree_summary_strip};

fn worktree_office_style(theme: &Theme) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    let base = theme.palette().background;
    let is_dark = base.r + base.g + base.b < 1.5;
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            p.background.weak.color.scale_alpha(0.16)
        } else {
            Color::from_rgba(0.95, 0.90, 0.76, 0.10)
        })),
        border: Border {
            width: 1.0,
            color: p.background.strong.color.scale_alpha(if is_dark { 0.42 } else { 0.35 }),
            radius: 2.0.into(),
        },
        ..Default::default()
    }
}

fn active_worktree_slots(snapshot: &WorktreePoolSnapshot) -> Vec<&WorktreeSlotSnapshot> {
    snapshot
        .slots
        .iter()
        .filter(|slot| matches!(slot.state, WorktreeState::Idle | WorktreeState::Busy))
        .collect()
}

fn maintenance_worktree_slots(snapshot: &WorktreePoolSnapshot) -> Vec<&WorktreeSlotSnapshot> {
    snapshot
        .slots
        .iter()
        .filter(|slot| {
            matches!(
                slot.state,
                WorktreeState::Tainted | WorktreeState::Recycling | WorktreeState::Dead
            )
        })
        .collect()
}

fn visible_task_status_count(app: &App, status: TaskStatus) -> usize {
    app.task_board_tasks
        .iter()
        .filter(|task| !task.deleted && !task.archived && task.status == status)
        .count()
}

/// 构建 Worktree 像素办公室，展示完整的办公室可视化界面。
pub(super) fn build_worktree_pixel_office<'a>(
    app: &'a App,
    snapshot: &'a WorktreePoolSnapshot,
) -> Element<'a, Message> {
    let active_slots = active_worktree_slots(snapshot);
    let maintenance_slots = maintenance_worktree_slots(snapshot);

    let summary_row = row![
        build_worktree_summary_strip("空闲工位", snapshot.idle_count, WorktreeState::Idle),
        build_worktree_summary_strip("执行中", snapshot.busy_count, WorktreeState::Busy),
        build_worktree_summary_strip("待处理污染", snapshot.tainted_count, WorktreeState::Tainted),
        build_worktree_summary_strip(
            "回收槽位",
            snapshot.recycling_count,
            WorktreeState::Recycling
        ),
        build_worktree_summary_strip("停机槽位", snapshot.dead_count, WorktreeState::Dead),
    ]
    .spacing(8)
    .width(Length::Fill);

    let task_status_summary = row![
        build_task_status_summary_strip(
            "任务池",
            visible_task_status_count(app, TaskStatus::Pool),
            TaskStatus::Pool
        ),
        build_task_status_summary_strip(
            "待执行",
            visible_task_status_count(app, TaskStatus::Pending),
            TaskStatus::Pending,
        ),
        build_task_status_summary_strip(
            "拆分",
            visible_task_status_count(app, TaskStatus::Planning),
            TaskStatus::Planning,
        ),
        build_task_status_summary_strip(
            "执行中",
            visible_task_status_count(app, TaskStatus::Running),
            TaskStatus::Running,
        ),
        build_task_status_summary_strip(
            "暂停",
            visible_task_status_count(app, TaskStatus::Paused),
            TaskStatus::Paused
        ),
        build_task_status_summary_strip(
            "失败",
            visible_task_status_count(app, TaskStatus::Failed),
            TaskStatus::Failed
        ),
        build_task_status_summary_strip(
            "完成",
            visible_task_status_count(app, TaskStatus::CodeComplete),
            TaskStatus::CodeComplete,
        ),
        build_task_status_summary_strip(
            "审核",
            visible_task_status_count(app, TaskStatus::CodeReview),
            TaskStatus::CodeReview,
        ),
        build_task_status_summary_strip(
            "合并",
            visible_task_status_count(app, TaskStatus::PrSubmitted),
            TaskStatus::PrSubmitted,
        ),
        build_task_status_summary_strip(
            "已完成",
            visible_task_status_count(app, TaskStatus::Completed),
            TaskStatus::Completed,
        ),
    ]
    .spacing(4)
    .width(Length::Fill);

    container(
        container(
            column![
                row![
                    text("像素办公室").size(12).style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(theme.extended_palette().background.base.text),
                    }),
                    text("每个工位都展示槽位状态、房间布置与关键道具").size(11).style(
                        |theme: &Theme| {
                            iced::widget::text::Style {
                                color: Some(
                                    theme.extended_palette().background.base.text.scale_alpha(0.72),
                                ),
                            }
                        }
                    )
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                summary_row,
                task_status_summary,
                build_worktree_pixel_floor(app, snapshot, active_slots, maintenance_slots)
            ]
            .spacing(8)
            .width(Length::Fill),
        )
        .padding([8, 8])
        .width(Length::Fill)
        .style(worktree_office_style),
    )
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
#[path = "worktree_office_tests.rs"]
mod worktree_office_tests;
