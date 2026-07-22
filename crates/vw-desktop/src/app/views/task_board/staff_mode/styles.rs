//! 员工模式局部样式。

use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{button, container, scrollable, svg, text};
use iced::{Background, Border, Color, Element, Font, Length, Shadow, Theme, Vector};

use crate::app::Message;
use crate::app::assets::{Icon, get_icon};
use crate::app::task::staff::{TaskBoardStaffLevel, TaskBoardStaffRole, TaskBoardStaffStatus};

pub(super) const STAFF_SCROLLBAR_WIDTH: u32 = 4;
pub(super) const STAFF_SCROLLBAR_GUTTER: u32 = 7;

pub(super) fn vertical_scrollbar() -> scrollable::Direction {
    scrollable::Direction::Vertical(
        scrollable::Scrollbar::new()
            .width(STAFF_SCROLLBAR_WIDTH)
            .scroller_width(STAFF_SCROLLBAR_WIDTH)
            .spacing(STAFF_SCROLLBAR_GUTTER - STAFF_SCROLLBAR_WIDTH),
    )
}

pub(super) fn bold_font() -> Font {
    Font { weight: iced::font::Weight::Bold, ..Default::default() }
}

pub(super) fn medium_font() -> Font {
    Font { weight: iced::font::Weight::Medium, ..Default::default() }
}

pub(super) fn root_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(surface_color(theme).scale_alpha(0.58))),
        ..Default::default()
    }
}

pub(super) fn side_panel_style(theme: &Theme) -> container::Style {
    panel_style(theme, 0.74)
}

pub(super) fn center_panel_style(theme: &Theme) -> container::Style {
    panel_style(theme, 0.82)
}

pub(super) fn empty_panel_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weak.color.scale_alpha(0.34))),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.42),
            radius: 8.0.into(),
        },
        text_color: Some(theme.palette().text),
        ..Default::default()
    }
}

pub(super) fn warning_panel_style(theme: &Theme) -> container::Style {
    let color = warning_color();
    container::Style {
        background: Some(Background::Color(color.scale_alpha(if is_dark_theme(theme) {
            0.16
        } else {
            0.10
        }))),
        border: Border { width: 1.0, color: color.scale_alpha(0.32), radius: 7.0.into() },
        text_color: Some(if is_dark_theme(theme) {
            Color::from_rgb8(253, 224, 171)
        } else {
            Color::from_rgb8(146, 64, 14)
        }),
        ..Default::default()
    }
}

pub(super) fn autonomous_toggle_style(theme: &Theme, enabled: bool) -> container::Style {
    let accent = if enabled { priority_color() } else { display_only_color() };
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(if enabled {
            accent.scale_alpha(if is_dark_theme(theme) { 0.18 } else { 0.11 })
        } else {
            palette.background.weak.color.scale_alpha(0.34)
        })),
        border: Border {
            width: 1.0,
            color: accent.scale_alpha(if enabled { 0.46 } else { 0.30 }),
            radius: 8.0.into(),
        },
        text_color: Some(theme.palette().text),
        ..Default::default()
    }
}

pub(super) fn lane_card_style(theme: &Theme, accent: Color, active: bool) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(if active {
            accent.scale_alpha(if is_dark_theme(theme) { 0.16 } else { 0.10 })
        } else {
            palette.background.weak.color.scale_alpha(0.38)
        })),
        border: Border {
            width: 1.0,
            color: if active { accent.scale_alpha(0.42) } else { palette.background.strong.color },
            radius: 8.0.into(),
        },
        text_color: Some(theme.palette().text),
        ..Default::default()
    }
}

pub(super) fn task_card_style(theme: &Theme, accent: Color, blocked: bool) -> container::Style {
    let palette = theme.extended_palette();
    let border_color = if blocked { danger_color() } else { accent };
    container::Style {
        background: Some(Background::Color(
            palette.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                0.56
            } else {
                0.76
            }),
        )),
        border: Border {
            width: 1.0,
            color: border_color.scale_alpha(if blocked { 0.58 } else { 0.32 }),
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark_theme(theme) { 0.18 } else { 0.08 }),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 16.0,
        },
        text_color: Some(theme.palette().text),
        ..Default::default()
    }
}

pub(super) fn member_card_style(theme: &Theme, accent: Color, enabled: bool) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(if enabled {
            palette.background.weak.color.scale_alpha(0.54)
        } else {
            palette.background.weak.color.scale_alpha(0.24)
        })),
        border: Border {
            width: 1.0,
            color: if enabled {
                accent.scale_alpha(0.28)
            } else {
                palette.background.strong.color.scale_alpha(0.44)
            },
            radius: 8.0.into(),
        },
        text_color: Some(theme.palette().text),
        ..Default::default()
    }
}

pub(super) fn icon_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered => palette.background.strong.color.scale_alpha(0.72),
        button::Status::Pressed => palette.primary.base.color.scale_alpha(0.26),
        button::Status::Disabled => palette.background.weak.color.scale_alpha(0.26),
        button::Status::Active => palette.background.weak.color.scale_alpha(0.58),
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: if matches!(status, button::Status::Disabled) {
            palette.background.weak.text.scale_alpha(0.42)
        } else {
            theme.palette().text
        },
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.54),
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn tab_button_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let palette = theme.extended_palette();
    let background = if selected {
        theme.palette().primary.scale_alpha(if is_dark_theme(theme) { 0.26 } else { 0.14 })
    } else {
        match status {
            button::Status::Hovered => palette.background.strong.color.scale_alpha(0.62),
            button::Status::Pressed => theme.palette().primary.scale_alpha(0.18),
            button::Status::Disabled => palette.background.weak.color.scale_alpha(0.22),
            button::Status::Active => palette.background.weak.color.scale_alpha(0.46),
        }
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: if selected {
            if is_dark_theme(theme) { Color::WHITE } else { theme.palette().primary }
        } else {
            theme.palette().text
        },
        border: Border {
            width: 1.0,
            color: if selected {
                theme.palette().primary.scale_alpha(0.42)
            } else {
                palette.background.strong.color.scale_alpha(0.48)
            },
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn assignment_member_button_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
    accent: Color,
) -> button::Style {
    let palette = theme.extended_palette();
    let background = if selected {
        accent.scale_alpha(if is_dark_theme(theme) { 0.24 } else { 0.16 })
    } else {
        match status {
            button::Status::Hovered => palette.background.strong.color.scale_alpha(0.62),
            button::Status::Pressed => accent.scale_alpha(0.18),
            button::Status::Disabled => palette.background.weak.color.scale_alpha(0.22),
            button::Status::Active => palette.background.weak.color.scale_alpha(0.42),
        }
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: if selected {
            if is_dark_theme(theme) {
                mix_color(accent, Color::WHITE, 0.24)
            } else {
                mix_color(accent, Color::BLACK, 0.18)
            }
        } else {
            theme.palette().text
        },
        border: Border {
            width: 1.0,
            color: if selected {
                accent.scale_alpha(0.56)
            } else {
                palette.background.strong.color.scale_alpha(0.48)
            },
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn muted_text_style(theme: &Theme) -> text::Style {
    text::Style { color: Some(theme.palette().text.scale_alpha(0.62)) }
}

pub(super) fn role_label(role: TaskBoardStaffRole) -> &'static str {
    match role {
        TaskBoardStaffRole::Orchestrator => "指挥调度",
        TaskBoardStaffRole::Planner => "规划员工",
        TaskBoardStaffRole::Executor => "执行员工",
        TaskBoardStaffRole::Reviewer => "审核员工",
        TaskBoardStaffRole::Merger => "合并员工",
        TaskBoardStaffRole::Observer => "观察员",
    }
}

pub(super) fn staff_status_label(status: TaskBoardStaffStatus, enabled: bool) -> &'static str {
    if !enabled {
        return "已停用";
    }
    match status {
        TaskBoardStaffStatus::Online => "在线",
        TaskBoardStaffStatus::Working => "工作中",
        TaskBoardStaffStatus::Blocked => "阻塞",
        TaskBoardStaffStatus::Offline => "离线",
    }
}

pub(super) fn staff_level_color(level: TaskBoardStaffLevel) -> Color {
    match level {
        TaskBoardStaffLevel::Principal => Color::from_rgb8(59, 130, 246),
        TaskBoardStaffLevel::Manager => Color::from_rgb8(245, 158, 11),
        TaskBoardStaffLevel::Executor => Color::from_rgb8(16, 185, 129),
        TaskBoardStaffLevel::Member => display_only_color(),
    }
}

pub(super) fn role_color(role: TaskBoardStaffRole) -> Color {
    match role {
        TaskBoardStaffRole::Orchestrator => Color::from_rgb8(59, 130, 246),
        TaskBoardStaffRole::Planner => Color::from_rgb8(124, 58, 237),
        TaskBoardStaffRole::Executor => Color::from_rgb8(16, 185, 129),
        TaskBoardStaffRole::Reviewer => Color::from_rgb8(245, 158, 11),
        TaskBoardStaffRole::Merger => Color::from_rgb8(6, 182, 212),
        TaskBoardStaffRole::Observer => Color::from_rgb8(148, 163, 184),
    }
}

pub(super) fn staff_status_color(status: TaskBoardStaffStatus, enabled: bool) -> Color {
    if !enabled {
        return Color::from_rgb8(100, 116, 139);
    }
    match status {
        TaskBoardStaffStatus::Online => Color::from_rgb8(34, 197, 94),
        TaskBoardStaffStatus::Working => Color::from_rgb8(59, 130, 246),
        TaskBoardStaffStatus::Blocked => danger_color(),
        TaskBoardStaffStatus::Offline => Color::from_rgb8(100, 116, 139),
    }
}

pub(super) fn display_only_color() -> Color {
    Color::from_rgb8(148, 163, 184)
}

pub(super) fn priority_color() -> Color {
    Color::from_rgb8(99, 102, 241)
}

pub(super) fn warning_color() -> Color {
    Color::from_rgb8(217, 119, 6)
}

pub(super) fn danger_color() -> Color {
    Color::from_rgb8(220, 38, 38)
}

pub(super) fn count_chip<'a>(label: String, accent: Color) -> Element<'a, Message> {
    container(text(label).size(11).font(bold_font()))
        .padding([2, 7])
        .style(move |theme| chip_style(theme, accent, true))
        .into()
}

pub(super) fn small_chip<'a>(label: String, accent: Color) -> Element<'a, Message> {
    container(text(label).size(10))
        .padding([3, 7])
        .style(move |theme| chip_style(theme, accent, false))
        .into()
}

pub(super) fn chip_button_style(
    theme: &Theme,
    status: button::Status,
    accent: Color,
) -> button::Style {
    let alpha = match status {
        button::Status::Hovered => 0.22,
        button::Status::Pressed => 0.28,
        button::Status::Disabled => 0.08,
        button::Status::Active => 0.14,
    };
    button::Style {
        background: Some(Background::Color(accent.scale_alpha(alpha))),
        text_color: if is_dark_theme(theme) {
            mix_color(accent, Color::WHITE, 0.18)
        } else {
            mix_color(accent, Color::BLACK, 0.12)
        },
        border: Border { width: 1.0, color: accent.scale_alpha(0.34), radius: 999.0.into() },
        ..Default::default()
    }
}

pub(super) fn status_dot<'a>(color: Color) -> Element<'a, Message> {
    container(iced::widget::Space::new().width(Length::Fixed(7.0)).height(Length::Fixed(7.0)))
        .width(Length::Fixed(9.0))
        .height(Length::Fixed(9.0))
        .style(move |_theme| container::Style {
            background: Some(Background::Color(color)),
            border: Border { radius: 999.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

pub(super) fn icon<'a>(icon: Icon, size: f32, enabled: bool) -> svg::Svg<'a, Theme> {
    svg::Svg::<Theme>::new(get_icon(icon))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |theme: &Theme, _status| svg::Style {
            color: Some(if enabled {
                theme.palette().text
            } else {
                theme.palette().text.scale_alpha(0.36)
            }),
        })
}

pub(super) fn tooltip<'a>(content: Element<'a, Message>, label: &'a str) -> Element<'a, Message> {
    Tooltip::new(content, tooltip_content(label), TooltipPosition::Top).gap(6.0).into()
}

fn tooltip_content<'a>(label: &'a str) -> Element<'a, Message> {
    container(text(label).size(12).style(|_theme| text::Style { color: Some(Color::WHITE) }))
        .padding([6, 8])
        .style(|_theme| container::Style {
            background: Some(Background::Color(Color::from_rgba8(12, 13, 15, 0.97))),
            border: Border {
                width: 1.0,
                color: Color::from_rgba8(255, 255, 255, 0.10),
                radius: 8.0.into(),
            },
            text_color: Some(Color::WHITE),
            ..Default::default()
        })
        .into()
}

fn panel_style(theme: &Theme, alpha: f32) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(surface_color(theme).scale_alpha(alpha))),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.54),
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark_theme(theme) { 0.20 } else { 0.08 }),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 22.0,
        },
        text_color: Some(theme.palette().text),
        ..Default::default()
    }
}

fn chip_style(theme: &Theme, accent: Color, strong: bool) -> container::Style {
    let alpha = if strong { 0.20 } else { 0.14 };
    container::Style {
        background: Some(Background::Color(accent.scale_alpha(alpha))),
        border: Border { width: 1.0, color: accent.scale_alpha(0.34), radius: 999.0.into() },
        text_color: Some(if is_dark_theme(theme) {
            mix_color(accent, Color::WHITE, 0.18)
        } else {
            mix_color(accent, Color::BLACK, 0.12)
        }),
        ..Default::default()
    }
}

fn mix_color(color: Color, target: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color {
        r: color.r + (target.r - color.r) * amount,
        g: color.g + (target.g - color.g) * amount,
        b: color.b + (target.b - color.b) * amount,
        a: color.a,
    }
}

fn surface_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        Color::from_rgb8(23, 27, 34)
    } else {
        Color::from_rgb8(248, 250, 252)
    }
}

fn is_dark_theme(theme: &Theme) -> bool {
    let bg = theme.palette().background;
    bg.r + bg.g + bg.b < 1.5
}

#[cfg(test)]
#[path = "styles_tests.rs"]
mod styles_tests;
