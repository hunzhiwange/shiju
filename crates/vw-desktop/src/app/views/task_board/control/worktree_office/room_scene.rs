use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};

use crate::app::message::TaskBoardMessage;
use crate::app::task::{TaskStatus, WorktreeSlotSnapshot, WorktreeState};
use crate::app::{App, Message};

use super::super::super::common::{button_style_danger, button_style_secondary};
use super::super::helpers::{
    format_duration_ms, running_dots, slot_task, task_status_tag_colors,
    worktree_log_preview_lines, worktree_room_props, worktree_state_actor, worktree_state_hint,
    worktree_state_label, worktree_state_palette,
};

/// 构建 Worktree 房间场景的可视化 UI 组件。
fn build_worktree_room_scene<'a>(
    app: &'a App,
    slot: &'a WorktreeSlotSnapshot,
) -> Element<'a, Message> {
    let (base_actor_icon, actor_state) = worktree_state_actor(slot.state);
    let now_ms = crate::app::time::now_ms();
    let owner = slot.leased_task_id.as_deref().unwrap_or("未分配任务");
    let task = slot_task(app, slot);
    let detail = match slot.state {
        WorktreeState::Tainted => slot.taint_reason.as_deref().unwrap_or("待清理"),
        WorktreeState::Busy => owner,
        WorktreeState::Recycling => "整理中",
        WorktreeState::Dead => "离线",
        WorktreeState::Idle => "可立即执行",
    };
    let occupancy_label = match slot.state {
        WorktreeState::Busy => "有人",
        WorktreeState::Idle => "空座",
        WorktreeState::Tainted => "封锁",
        WorktreeState::Recycling => "整理",
        WorktreeState::Dead => "停机",
    };
    let props = worktree_room_props(slot.state);
    let is_tight = app.window_size.0 < 1520.0;
    let is_compact = app.window_size.0 < 1320.0;
    let ceiling_line = if is_compact {
        "╱──────────────────────────────╲"
    } else if is_tight {
        "╱────────────────────────────────────────╲"
    } else {
        "╱────────────────────────────────────────────────────╲"
    };
    let desk_edge = if is_compact {
        "╭──────────╮──────────────╭──────────╮"
    } else if is_tight {
        "╭────────────╮──────────────────╭────────────╮"
    } else {
        "╭────────────────╮──────────────────────────╭────────────────╮"
    };
    let floor_edge = if is_compact {
        "╘════════════════════════════════════════╛"
    } else if is_tight {
        "╘══════════════════════════════════════════════════════╛"
    } else {
        "╘════════════════════════════════════════════════════════════════╛"
    };
    let floor_texture = if is_compact {
        "░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░"
    } else if is_tight {
        "░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░"
    } else {
        "░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░"
    };
    let is_busy = matches!(slot.state, WorktreeState::Busy);
    let busy_phase = (now_ms / 240).is_multiple_of(2);
    let actor_icon = if is_busy {
        if busy_phase { "👨‍💻" } else { "🧑‍💻" }
    } else {
        base_actor_icon
    };
    let busy_face = match slot.state {
        WorktreeState::Idle => "🙂",
        WorktreeState::Busy => "😄",
        WorktreeState::Tainted => "😵",
        WorktreeState::Recycling => "🫥",
        WorktreeState::Dead => "😴",
    };
    let actor_bounce = if is_busy && busy_phase { 1.5 } else { 0.0 };
    let keyboard_nudge = 4.0;
    let chair_overlap = -11.0;
    let desk_keyboard = if is_busy { if busy_phase { "⌨️" } else { "⌨" } } else { "⌨️" };
    let animated_actor_state = if is_busy {
        let terminal_dots = match ((now_ms / 350) % 4) as u8 {
            0 => "",
            1 => ".",
            2 => "..",
            _ => "...",
        };
        format!("{}{}", actor_state, terminal_dots)
    } else {
        actor_state.to_string()
    };
    let task_id_label = if is_busy { format!("任务 {owner}") } else { owner.to_string() };
    let task_line: Element<'a, Message> = if let Some(task) = task {
        let (status_color, status_bg_color) = task_status_tag_colors(task.status);
        let status_text = if task.status == TaskStatus::Running {
            format!("{} {}", task.status.label(), running_dots(now_ms))
        } else {
            task.status.label().to_string()
        };
        let mut task_row = row![
            text(task_id_label).size(11).style(move |theme: &Theme| {
                let (_, text_color, _) = worktree_state_palette(theme, slot.state);
                iced::widget::text::Style { color: Some(text_color.scale_alpha(0.76)) }
            }),
            container(text(status_text).size(9)).padding([1, 6]).style(move |_theme: &Theme| {
                iced::widget::container::Style {
                    background: Some(Background::Color(status_bg_color)),
                    border: Border { radius: 999.0.into(), ..Default::default() },
                    text_color: Some(status_color),
                    ..Default::default()
                }
            }),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        if let Some(duration_ms) = task.display_execution_duration_ms(now_ms) {
            let duration_label = format_duration_ms(duration_ms);
            task_row = task_row.push(text(duration_label).size(10).style(move |theme: &Theme| {
                let (_, text_color, _) = worktree_state_palette(theme, slot.state);
                iced::widget::text::Style { color: Some(text_color.scale_alpha(0.68)) }
            }));
        }
        task_row.into()
    } else {
        text(task_id_label)
            .size(11)
            .style(move |theme: &Theme| {
                let (_, text_color, _) = worktree_state_palette(theme, slot.state);
                iced::widget::text::Style { color: Some(text_color.scale_alpha(0.76)) }
            })
            .into()
    };
    let log_preview_lines =
        if is_busy { worktree_log_preview_lines(app, slot, now_ms) } else { Vec::new() };

    let title_meta = column![
        row![
            text(animated_actor_state).size(12).style(move |theme: &Theme| {
                let (_, text_color, _) = worktree_state_palette(theme, slot.state);
                iced::widget::text::Style { color: Some(text_color) }
            }),
            text(busy_face).size(12).style(move |theme: &Theme| {
                let (_, text_color, _) = worktree_state_palette(theme, slot.state);
                iced::widget::text::Style { color: Some(text_color.scale_alpha(0.92)) }
            }),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        text(format!("工位 {}", slot.id)).size(11).style(move |theme: &Theme| {
            let (_, text_color, _) = worktree_state_palette(theme, slot.state);
            iced::widget::text::Style { color: Some(text_color.scale_alpha(0.76)) }
        }),
        text(format!("分支 {}", slot.branch)).size(11).style(move |theme: &Theme| {
            let (_, text_color, _) = worktree_state_palette(theme, slot.state);
            iced::widget::text::Style { color: Some(text_color.scale_alpha(0.82)) }
        }),
        task_line,
    ]
    .spacing(3)
    .align_x(Alignment::End);

    let status_footer = row![
        column![
            text(detail).size(11).style(move |theme: &Theme| {
                let (_, text_color, _) = worktree_state_palette(theme, slot.state);
                iced::widget::text::Style { color: Some(text_color.scale_alpha(0.76)) }
            }),
            text(worktree_state_hint(slot.state)).size(10).style(move |theme: &Theme| {
                let (_, text_color, _) = worktree_state_palette(theme, slot.state);
                iced::widget::text::Style { color: Some(text_color.scale_alpha(0.62)) }
            }),
        ]
        .spacing(3),
        Space::new().width(Length::Fill),
        container(text(occupancy_label).size(10)).padding([2, 6]).style(move |theme: &Theme| {
            let (_, text_color, border_color) = worktree_state_palette(theme, slot.state);
            iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgba(
                    border_color.r,
                    border_color.g,
                    border_color.b,
                    0.12,
                ))),
                border: Border {
                    width: 1.0,
                    color: border_color.scale_alpha(0.55),
                    radius: 2.0.into(),
                },
                text_color: Some(text_color),
                ..Default::default()
            }
        }),
        text(format!("{} {}", props[0], props[1])).size(11).style(move |theme: &Theme| {
            let (_, text_color, _) = worktree_state_palette(theme, slot.state);
            iced::widget::text::Style { color: Some(text_color.scale_alpha(0.72)) }
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let log_bubble: Element<'a, Message> = if is_busy {
        let total = log_preview_lines.len();
        let bubble = log_preview_lines.into_iter().enumerate().fold(
            column![].spacing(1),
            |column, (index, line)| {
                let alpha = if index + 1 == total {
                    0.96
                } else if index + 2 == total {
                    0.78
                } else {
                    0.60
                };
                column.push(text(line).size(9).style(move |theme: &Theme| {
                    iced::widget::text::Style {
                        color: Some(
                            theme.extended_palette().background.base.text.scale_alpha(alpha),
                        ),
                    }
                }))
            },
        );
        container(bubble)
            .padding(Padding { top: 4.0, right: 12.0, bottom: 10.0, left: 12.0 })
            .width(Length::Fixed(320.0))
            .height(Length::Fixed(64.0))
            .style(|theme: &Theme| {
                let p = theme.extended_palette();
                let base = theme.palette().background;
                let is_dark = base.r + base.g + base.b < 1.5;
                iced::widget::container::Style {
                    background: Some(Background::Color(if is_dark {
                        Color::from_rgba(0.13, 0.15, 0.18, 0.94)
                    } else {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.94)
                    })),
                    border: Border {
                        width: 1.0,
                        color: p.background.strong.color.scale_alpha(if is_dark {
                            0.28
                        } else {
                            0.16
                        }),
                        radius: 10.0.into(),
                    },
                    text_color: Some(p.background.base.text),
                    ..Default::default()
                }
            })
            .into()
    } else {
        Space::new().width(Length::Fixed(0.0)).into()
    };

    let office_scene = container(
        column![
            row![
                text("🪟").size(16),
                text("🪟").size(16),
                text("╱").size(12).style(move |_theme: &Theme| iced::widget::text::Style {
                    color: Some(Color::from_rgba(0.82, 0.74, 0.50, 0.40)),
                }),
                Space::new().width(Length::Fill),
                title_meta,
            ]
            .spacing(6)
            .align_y(Alignment::Start),
            row![
                Space::new().width(Length::Fixed(2.0)),
                text(ceiling_line).size(14).style(move |theme: &Theme| {
                    let base = theme.palette().background;
                    let is_dark = base.r + base.g + base.b < 1.5;
                    iced::widget::text::Style {
                        color: Some(if is_dark {
                            Color::from_rgba(0.62, 0.56, 0.42, 0.68)
                        } else {
                            Color::from_rgba(0.92, 0.86, 0.66, 0.88)
                        }),
                    }
                }),
                Space::new().width(Length::Fill),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            row![
                Space::new().width(Length::Fixed(2.0)),
                text("🖥️").size(39),
                text("💻").size(34),
                text(desk_keyboard).size(if is_busy && busy_phase { 26 } else { 23 }).style(
                    move |theme: &Theme| {
                        let (_, text_color, _) = worktree_state_palette(theme, slot.state);
                        iced::widget::text::Style {
                            color: Some(if is_busy {
                                text_color.scale_alpha(if busy_phase { 0.98 } else { 0.62 })
                            } else {
                                text_color.scale_alpha(0.78)
                            }),
                        }
                    }
                ),
                Space::new().width(Length::Fixed(if is_busy && busy_phase { 2.0 } else { 6.0 })),
                Space::new().width(Length::Fill),
                text("🪴").size(18),
            ]
            .spacing(5)
            .align_y(Alignment::Center),
            row![
                Space::new().width(Length::Fixed(4.0)),
                text("🪑").size(40),
                Space::new().width(Length::Fixed(chair_overlap)),
                container(
                    column![
                        Space::new().height(Length::Fixed(actor_bounce)),
                        text(actor_icon).size(62)
                    ]
                    .spacing(0)
                    .align_x(Alignment::Center),
                )
                .width(Length::Fixed(64.0)),
                Space::new().width(Length::Fixed(keyboard_nudge)),
                Space::new().width(Length::Fixed(6.0)),
                log_bubble,
                Space::new().width(Length::Fill),
                text("🖱️").size(17),
            ]
            .spacing(3)
            .align_y(Alignment::End),
            row![
                Space::new().width(Length::Fixed(28.0)),
                Space::new().width(Length::Fill),
                text("╭──────────────────────╮").size(14).style(move |_theme: &Theme| {
                    iced::widget::text::Style {
                        color: Some(Color::from_rgba(0.74, 0.56, 0.32, 0.40)),
                    }
                }),
                Space::new().width(Length::Fill),
            ]
            .align_y(Alignment::Center),
            row![
                Space::new().width(Length::Fixed(0.0)),
                text(desk_edge).size(15).style(move |_theme: &Theme| iced::widget::text::Style {
                    color: Some(Color::from_rgba(0.70, 0.50, 0.30, 0.52)),
                }),
            ]
            .align_y(Alignment::Center),
            row![text(floor_edge).size(18).style(move |_theme: &Theme| {
                iced::widget::text::Style { color: Some(Color::from_rgba(0.46, 0.30, 0.18, 0.92)) }
            }),]
            .align_y(Alignment::Center),
            row![text(floor_texture).size(10).style(move |_theme: &Theme| {
                iced::widget::text::Style { color: Some(Color::from_rgba(0.72, 0.60, 0.36, 0.18)) }
            }),]
            .align_y(Alignment::Center),
            status_footer,
        ]
        .spacing(1)
        .width(Length::Fill),
    )
    .padding([10, 14])
    .style(move |theme: &Theme| {
        let (_, _, border_color) = worktree_state_palette(theme, slot.state);
        let base = theme.palette().background;
        let is_dark = base.r + base.g + base.b < 1.5;
        iced::widget::container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(0.11, 0.12, 0.15, 0.98)
            } else {
                Color::from_rgba(0.995, 0.965, 0.84, 0.99)
            })),
            border: Border { width: 0.0, color: border_color.scale_alpha(0.0), radius: 2.0.into() },
            ..Default::default()
        }
    });

    container(office_scene)
        .padding([6, 6])
        .width(Length::Fill)
        .style(move |theme: &Theme| {
            let (_, _, border_color) = worktree_state_palette(theme, slot.state);
            let base = theme.palette().background;
            let is_dark = base.r + base.g + base.b < 1.5;
            iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgba(
                    border_color.r,
                    border_color.g,
                    border_color.b,
                    if is_dark { 0.12 } else { 0.08 },
                ))),
                border: Border {
                    width: 0.0,
                    color: border_color.scale_alpha(0.0),
                    radius: 2.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

/// 生成 Worktree 房间门牌号。
fn worktree_room_plate(slot_id: &str, state: WorktreeState) -> String {
    let zone = match state {
        WorktreeState::Idle | WorktreeState::Busy => "A",
        WorktreeState::Tainted => "Q",
        WorktreeState::Recycling => "R",
        WorktreeState::Dead => "X",
    };
    format!("{}-{}", zone, slot_id)
}

/// 构建 Worktree 像素卡片，显示单个槽位的完整信息。
pub(super) fn build_worktree_pixel_card<'a>(
    app: &'a App,
    slot: &'a WorktreeSlotSnapshot,
) -> Element<'a, Message> {
    let room_plate = worktree_room_plate(&slot.id, slot.state);
    let recycle_btn = button(text("回收").size(10))
        .on_press(Message::TaskBoard(TaskBoardMessage::RecycleWorktreeSlotPressed {
            slot_id: slot.id.clone(),
        }))
        .padding([2, 8])
        .style(if matches!(slot.state, WorktreeState::Tainted | WorktreeState::Dead) {
            button_style_danger
        } else {
            button_style_secondary
        });

    container(
        column![
            row![
                text(room_plate).size(12).style(move |theme: &Theme| {
                    let (_, text_color, _) = worktree_state_palette(theme, slot.state);
                    iced::widget::text::Style { color: Some(text_color) }
                }),
                text(worktree_state_label(slot.state)).size(11).style(move |theme: &Theme| {
                    let (_, text_color, _) = worktree_state_palette(theme, slot.state);
                    iced::widget::text::Style { color: Some(text_color.scale_alpha(0.76)) }
                }),
                Space::new().width(Length::Fill),
                recycle_btn,
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            build_worktree_room_scene(app, slot),
        ]
        .spacing(3),
    )
    .padding([8, 8])
    .width(Length::FillPortion(1))
    .height(Length::Fixed(268.0))
    .style(move |theme: &Theme| {
        let (bg, text_color, border_color) = worktree_state_palette(theme, slot.state);
        iced::widget::container::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                width: if app.task_board_worktree_panel_expanded
                    || slot.state == WorktreeState::Busy
                {
                    2.0
                } else {
                    1.0
                },
                color: border_color,
                radius: 2.0.into(),
            },
            text_color: Some(text_color),
            ..Default::default()
        }
    })
    .into()
}

#[cfg(test)]
#[path = "room_scene_tests.rs"]
mod room_scene_tests;
