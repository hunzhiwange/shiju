use iced::widget::{column, container, text};
use iced::{Background, Border, Element, Length, Theme};

use crate::app::Message;
use crate::app::task::{TaskStatus, WorktreeState};

use super::super::helpers::{task_status_tag_colors, worktree_state_palette};

/// 构建 Worktree 状态统计条带，显示特定状态的槽位数量。
pub(super) fn build_worktree_summary_strip<'a>(
    label: &'static str,
    value: usize,
    state: WorktreeState,
) -> Element<'a, Message> {
    container(
        column![
            text(label).size(10).style(move |theme: &Theme| worktree_label_style(theme, state)),
            text(value.to_string())
                .size(14)
                .style(move |theme: &Theme| worktree_value_style(theme, state)),
        ]
        .spacing(2),
    )
    .padding([6, 10])
    .width(Length::FillPortion(1))
    .style(move |theme: &Theme| worktree_container_style(theme, state))
    .into()
}

/// 构建任务状态统计条带，显示特定状态的任务数量。
pub(super) fn build_task_status_summary_strip<'a>(
    label: &'static str,
    value: usize,
    status: TaskStatus,
) -> Element<'a, Message> {
    container(
        column![
            text(label).size(8).style(move |_theme: &Theme| task_status_label_style(status)),
            text(value.to_string())
                .size(11)
                .style(move |_theme: &Theme| task_status_value_style(status)),
        ]
        .spacing(1),
    )
    .padding([3, 5])
    .width(Length::FillPortion(1))
    .style(move |_theme: &Theme| task_status_container_style(status))
    .into()
}

fn worktree_label_style(theme: &Theme, state: WorktreeState) -> iced::widget::text::Style {
    let (_, text_color, _) = worktree_state_palette(theme, state);
    iced::widget::text::Style { color: Some(text_color.scale_alpha(0.72)) }
}

fn worktree_value_style(theme: &Theme, state: WorktreeState) -> iced::widget::text::Style {
    let (_, text_color, _) = worktree_state_palette(theme, state);
    iced::widget::text::Style { color: Some(text_color) }
}

fn worktree_container_style(theme: &Theme, state: WorktreeState) -> iced::widget::container::Style {
    let (bg, _, border_color) = worktree_state_palette(theme, state);
    iced::widget::container::Style {
        background: Some(Background::Color(bg.scale_alpha(0.9))),
        border: Border { width: 1.0, color: border_color.scale_alpha(0.8), radius: 2.0.into() },
        ..Default::default()
    }
}

fn task_status_label_style(status: TaskStatus) -> iced::widget::text::Style {
    let (text_color, _) = task_status_tag_colors(status);
    iced::widget::text::Style { color: Some(text_color.scale_alpha(0.84)) }
}

fn task_status_value_style(status: TaskStatus) -> iced::widget::text::Style {
    let (text_color, _) = task_status_tag_colors(status);
    iced::widget::text::Style { color: Some(text_color) }
}

fn task_status_container_style(status: TaskStatus) -> iced::widget::container::Style {
    let (text_color, bg_color) = task_status_tag_colors(status);
    iced::widget::container::Style {
        background: Some(Background::Color(bg_color.scale_alpha(0.92))),
        border: Border { width: 1.0, color: text_color.scale_alpha(0.42), radius: 2.0.into() },
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "summary_tests.rs"]
mod summary_tests;
