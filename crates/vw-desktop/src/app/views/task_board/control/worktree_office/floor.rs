use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::task::{WorktreePoolSnapshot, WorktreeSlotSnapshot};
use crate::app::{App, Message};

use super::room_scene::build_worktree_pixel_card;

fn is_dark_floor_theme(theme: &Theme) -> bool {
    let base = theme.palette().background;
    base.r + base.g + base.b < 1.5
}

fn floor_text_alpha_style(theme: &Theme, alpha: f32) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(theme.extended_palette().background.base.text.scale_alpha(alpha)),
    }
}

fn build_worktree_floor_strip_style(theme: &Theme) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    let is_dark = is_dark_floor_theme(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.27, 0.23, 0.16, 0.24)
        } else {
            Color::from_rgba(0.90, 0.82, 0.58, 0.34)
        })),
        border: Border {
            width: 1.0,
            color: p.background.strong.color.scale_alpha(if is_dark { 0.4 } else { 0.3 }),
            radius: 2.0.into(),
        },
        text_color: Some(p.background.base.text.scale_alpha(if is_dark { 0.82 } else { 0.75 })),
        ..Default::default()
    }
}

fn build_worktree_floor_texture_fill_style(theme: &Theme) -> iced::widget::container::Style {
    let is_dark = is_dark_floor_theme(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.24, 0.20, 0.14, 0.20)
        } else {
            Color::from_rgba(0.93, 0.85, 0.61, 0.28)
        })),
        ..Default::default()
    }
}

fn build_worktree_corridor_style(theme: &Theme) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    let is_dark = is_dark_floor_theme(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.22, 0.19, 0.14, 0.16)
        } else {
            Color::from_rgba(0.79, 0.73, 0.58, 0.22)
        })),
        border: Border {
            width: 1.0,
            color: p.background.strong.color.scale_alpha(if is_dark { 0.36 } else { 0.3 }),
            radius: 2.0.into(),
        },
        text_color: Some(p.background.base.text),
        ..Default::default()
    }
}

fn build_empty_worktree_section_style(theme: &Theme) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    let is_dark = is_dark_floor_theme(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            p.background.weak.color.scale_alpha(0.24)
        } else {
            p.background.base.color.scale_alpha(0.22)
        })),
        border: Border {
            width: 1.0,
            color: p.background.strong.color.scale_alpha(if is_dark { 0.34 } else { 0.25 }),
            radius: 2.0.into(),
        },
        ..Default::default()
    }
}

fn build_worktree_pixel_section_style(theme: &Theme) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    let is_dark = is_dark_floor_theme(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            p.background.weak.color.scale_alpha(0.22)
        } else {
            p.background.base.color.scale_alpha(0.18)
        })),
        border: Border {
            width: 1.0,
            color: p.background.strong.color.scale_alpha(if is_dark { 0.4 } else { 0.3 }),
            radius: 2.0.into(),
        },
        ..Default::default()
    }
}

fn build_worktree_pixel_floor_style(theme: &Theme) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    let is_dark = is_dark_floor_theme(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.20, 0.17, 0.12, 0.14)
        } else {
            Color::from_rgba(0.94, 0.86, 0.60, 0.24)
        })),
        border: Border {
            width: 1.0,
            color: p.background.strong.color.scale_alpha(if is_dark { 0.36 } else { 0.28 }),
            radius: 2.0.into(),
        },
        ..Default::default()
    }
}

pub(super) struct WorktreeCorridorSummary {
    status: &'static str,
    maintenance_hint: String,
    width: u32,
    busy_flow: bool,
}

pub(super) fn worktree_corridor_summary(
    snapshot: &WorktreePoolSnapshot,
    maintenance_hidden: bool,
) -> WorktreeCorridorSummary {
    let status = if snapshot.busy_count > 0 {
        "任务流动中"
    } else if snapshot.tainted_count > 0 || snapshot.recycling_count > 0 {
        "维护队列处理中"
    } else {
        "走廊空闲"
    };
    let maintenance_total = snapshot.tainted_count + snapshot.recycling_count + snapshot.dead_count;
    let maintenance_hint = if maintenance_hidden {
        format!(
            "运维修复间已收起\n污染:{} 回收:{} 停机:{}",
            snapshot.tainted_count, snapshot.recycling_count, snapshot.dead_count
        )
    } else {
        format!("运维修复间记录: {maintenance_total}")
    };
    let width = if maintenance_hidden { 190 } else { 120 };

    WorktreeCorridorSummary { status, maintenance_hint, width, busy_flow: snapshot.busy_count > 0 }
}

/// 构建地板装饰条带，用于分隔办公室楼层区域。
fn build_worktree_floor_strip<'a>(label: &'static str) -> Element<'a, Message> {
    container(
        column![
            row![
                text("▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦").size(10),
                Space::new().width(Length::Fill),
                text(label).size(10),
                Space::new().width(Length::Fill),
                text("▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥").size(10),
            ]
            .align_y(Alignment::Center),
            text("▨▩▨▩▨▩▨▩▨▩▨▩▨▩▨▩▨▩▨▩▨▩▨▩")
                .size(9)
                .style(|theme: &Theme| floor_text_alpha_style(theme, 0.38)),
        ]
        .spacing(2),
    )
    .padding([6, 8])
    .width(Length::Fill)
    .style(build_worktree_floor_strip_style)
    .into()
}

/// 构建地板纹理填充区域，用于办公室楼层的顶部和底部装饰。
fn build_worktree_floor_texture_fill<'a>() -> Element<'a, Message> {
    container(
        column![
            text("▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦▥▦")
                .size(10)
                .style(|theme: &Theme| floor_text_alpha_style(theme, 0.3)),
            text("▩▨▩▨▩▨▩▨▩▨▩▨▩▨▩▨▩▨▩▨▩▨▩▨")
                .size(10)
                .style(|theme: &Theme| floor_text_alpha_style(theme, 0.24)),
        ]
        .spacing(1),
    )
    .padding([4, 8])
    .width(Length::Fill)
    .style(build_worktree_floor_texture_fill_style)
    .into()
}

/// 构建 Worktree 走廊组件，显示任务流程和维护状态概览。
fn build_worktree_corridor<'a>(
    snapshot: &'a WorktreePoolSnapshot,
    maintenance_hidden: bool,
) -> Element<'a, Message> {
    let summary = worktree_corridor_summary(snapshot, maintenance_hidden);
    let corridor_steps = if summary.busy_flow {
        row![
            text("入口").size(10),
            text("=>").size(10),
            text("执行").size(10),
            text("=>").size(10),
            text("回收").size(10),
        ]
    } else {
        row![
            text("入口").size(10),
            text(".").size(10),
            text("空闲").size(10),
            text(".").size(10),
            text("归档").size(10),
        ]
    }
    .spacing(6)
    .align_y(Alignment::Center);

    container(
        column![
            text("主走廊").size(11).style(|theme: &Theme| floor_text_alpha_style(theme, 0.82)),
            text(summary.status)
                .size(10)
                .style(|theme: &Theme| floor_text_alpha_style(theme, 0.62)),
            corridor_steps,
            text(format!(
                "活跃:{} 维护:{}",
                snapshot.busy_count,
                snapshot.tainted_count + snapshot.recycling_count
            ))
            .size(10)
            .style(|theme: &Theme| floor_text_alpha_style(theme, 0.56)),
            text(summary.maintenance_hint)
                .size(10)
                .style(|theme: &Theme| floor_text_alpha_style(theme, 0.64)),
            text("│\n│\n│\n│\n│\n│\n│\n↓")
                .size(11)
                .style(|theme: &Theme| { floor_text_alpha_style(theme, 0.5) })
        ]
        .spacing(6)
        .align_x(Alignment::Center),
    )
    .padding([10, 8])
    .width(summary.width)
    .height(Length::Fill)
    .style(build_worktree_corridor_style)
    .into()
}

/// 构建 Worktree 像素区域，显示一组槽位卡片的网格布局。
fn build_worktree_pixel_section<'a>(
    app: &'a App,
    title: &'static str,
    subtitle: &'static str,
    slots: Vec<&'a WorktreeSlotSnapshot>,
    cards_per_row: usize,
) -> Element<'a, Message> {
    let cards_per_row = cards_per_row.max(1);
    let header = column![
        text(title).size(12).style(|theme: &Theme| floor_text_alpha_style(theme, 1.0)),
        text(subtitle).size(11).style(|theme: &Theme| floor_text_alpha_style(theme, 0.72)),
    ]
    .spacing(4)
    .width(Length::Fill);

    let mut room_rows = column![].spacing(22).width(Length::Fill);

    if slots.is_empty() {
        room_rows = room_rows.push(
            container(
                text("当前区域暂无工位")
                    .size(11)
                    .style(|theme: &Theme| floor_text_alpha_style(theme, 0.62)),
            )
            .padding([8, 10])
            .width(Length::Fill)
            .style(build_empty_worktree_section_style),
        );
    } else {
        for chunk in slots.chunks(cards_per_row) {
            let mut office_row = row![].spacing(10).width(Length::Fill);
            for slot in chunk {
                office_row = office_row.push(build_worktree_pixel_card(app, slot));
            }
            for _ in chunk.len()..cards_per_row {
                office_row = office_row.push(container(text(" ")).width(Length::FillPortion(1)));
            }
            room_rows = room_rows.push(office_row);
        }
    }
    container(column![header, room_rows].spacing(12).width(Length::Fill))
        .padding([8, 8])
        .width(Length::Fill)
        .style(build_worktree_pixel_section_style)
        .into()
}

/// 构建 Worktree 像素楼层，组合活跃区和维护区的完整布局。
pub(super) fn build_worktree_pixel_floor<'a>(
    app: &'a App,
    snapshot: &'a WorktreePoolSnapshot,
    active_slots: Vec<&'a WorktreeSlotSnapshot>,
    maintenance_slots: Vec<&'a WorktreeSlotSnapshot>,
) -> Element<'a, Message> {
    let has_maintenance_rooms = !maintenance_slots.is_empty();
    let use_single_column = app.window_size.0 < 1360.0;
    let active_cards_per_row = if use_single_column { 1 } else { 2 };

    let active_section = container(build_worktree_pixel_section(
        app,
        "任务办公区",
        "空闲槽位可立即接入任务，繁忙槽位持续处理中",
        active_slots,
        active_cards_per_row,
    ))
    .width(Length::Fill);

    let corridor = build_worktree_corridor(snapshot, !has_maintenance_rooms);

    let maintenance_section = container(build_worktree_pixel_section(
        app,
        "运维修复间",
        "处理污染、回收和停机工位",
        maintenance_slots,
        1,
    ))
    .width(Length::Fill);

    let room_line: Element<'a, Message> = if use_single_column {
        let mut stacked_rooms = column![active_section, corridor].spacing(42).width(Length::Fill);
        if has_maintenance_rooms {
            stacked_rooms = stacked_rooms.push(maintenance_section);
        }
        stacked_rooms.into()
    } else {
        let mut wide_rooms = row![
            active_section.width(Length::FillPortion(if has_maintenance_rooms { 5 } else { 7 })),
            corridor,
        ]
        .spacing(14)
        .width(Length::Fill)
        .align_y(Alignment::Start);
        if has_maintenance_rooms {
            wide_rooms = wide_rooms.push(maintenance_section.width(Length::FillPortion(2)));
        }
        wide_rooms.into()
    };

    container(
        column![
            build_worktree_floor_texture_fill(),
            build_worktree_floor_strip("房间地板"),
            room_line,
            build_worktree_floor_strip("工位走道"),
            build_worktree_floor_texture_fill(),
        ]
        .spacing(8)
        .width(Length::Fill),
    )
    .padding([8, 8])
    .width(Length::Fill)
    .style(build_worktree_pixel_floor_style)
    .into()
}

#[cfg(test)]
#[path = "floor_tests.rs"]
mod floor_tests;
