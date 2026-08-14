//! 轨迹概览与账本的主题和控件样式。

use iced::widget::scrollable::Direction;
use iced::widget::{button, container, text_input};
use iced::{Background, Border, Color, Font, Theme};

use super::TrajectoryRecordKind;
use super::timeline::TimelineLane;
use crate::app::components::chat_panel::utils::{chat_scroll_direction, is_dark_theme};

pub(crate) const LEDGER_SCROLLBAR_WIDTH: f32 = 4.0;
pub(crate) const DETAILS_SCROLLBAR_WIDTH: f32 = 4.0;

pub(crate) fn ledger_scroll_direction() -> Direction {
    chat_scroll_direction()
}

pub(crate) fn details_scroll_direction() -> Direction {
    chat_scroll_direction()
}

pub(crate) const fn record_preview_font(_preview: &str) -> Font {
    Font::DEFAULT
}

pub(crate) fn role_badge_style(theme: &Theme, kind: TrajectoryRecordKind) -> container::Style {
    let palette = theme.extended_palette();
    let color = match kind {
        TrajectoryRecordKind::System => palette.secondary.base.color,
        TrajectoryRecordKind::User => palette.success.base.color,
        TrajectoryRecordKind::Assistant => palette.primary.base.color,
        TrajectoryRecordKind::Tool => palette.warning.base.color,
    };
    container::Style {
        text_color: Some(color),
        background: Some(Background::Color(color.scale_alpha(if is_dark_theme(theme) {
            0.20
        } else {
            0.12
        }))),
        border: Border { radius: 4.0.into(), ..Border::default() },
        ..container::Style::default()
    }
}

pub(crate) fn ledger_record_row_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
    missing: bool,
) -> button::Style {
    let palette = theme.extended_palette();
    let background = if selected {
        Some(Background::Color(palette.primary.weak.color.scale_alpha(if is_dark_theme(theme) {
            0.34
        } else {
            0.22
        })))
    } else {
        match status {
            button::Status::Hovered => Some(Background::Color(
                palette.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                    0.72
                } else {
                    0.58
                }),
            )),
            button::Status::Pressed => Some(Background::Color(
                palette.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                    0.58
                } else {
                    0.42
                }),
            )),
            button::Status::Active | button::Status::Disabled => None,
        }
    };
    let border = if selected {
        Border {
            width: 1.0,
            color: palette.primary.base.color.scale_alpha(0.70),
            radius: 6.0.into(),
        }
    } else {
        Border { radius: 6.0.into(), ..Border::default() }
    };
    let text_color = if missing {
        palette.danger.base.color.scale_alpha(0.88)
    } else {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.92 } else { 0.86 })
    };

    button::Style { background, border, text_color, ..button::Style::default() }
}

pub(crate) fn group_header_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(theme.palette().text.scale_alpha(if is_dark_theme(theme) {
            0.80
        } else {
            0.72
        })),
        background: Some(Background::Color(
            palette.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                0.48
            } else {
                0.38
            }),
        )),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.48
            } else {
                0.28
            }),
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

pub(crate) fn step_header_text_color(theme: &Theme) -> Color {
    theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.58 } else { 0.52 })
}

pub(crate) fn empty_text_color(theme: &Theme) -> Color {
    theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.54 } else { 0.48 })
}

pub(crate) fn details_panel_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(
            palette.background.base.color.scale_alpha(if is_dark_theme(theme) {
                0.72
            } else {
                0.82
            }),
        )),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.54
            } else {
                0.34
            }),
            radius: 8.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark_theme(theme) { 0.34 } else { 0.18 }),
            offset: iced::Vector::new(-4.0, 3.0),
            blur_radius: 18.0,
        },
        ..container::Style::default()
    }
}

pub(crate) fn trajectory_search_input_style(
    theme: &Theme,
    status: text_input::Status,
) -> text_input::Style {
    let palette = theme.extended_palette();
    let focused = matches!(status, text_input::Status::Focused { .. });
    let disabled = matches!(status, text_input::Status::Disabled);
    text_input::Style {
        background: Background::Color(
            palette.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                0.58
            } else {
                0.72
            }),
        ),
        border: Border {
            width: if focused { 1.5 } else { 1.0 },
            color: if focused {
                palette.primary.base.color
            } else {
                palette.background.strong.color.scale_alpha(0.54)
            },
            radius: 12.0.into(),
        },
        icon: theme.palette().text.scale_alpha(0.62),
        placeholder: theme.palette().text.scale_alpha(0.48),
        value: theme.palette().text.scale_alpha(if disabled { 0.46 } else { 0.92 }),
        selection: theme.palette().primary.scale_alpha(0.22),
    }
}

pub(crate) fn details_close_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered => Some(Background::Color(
            palette.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.50
            } else {
                0.34
            }),
        )),
        button::Status::Pressed => Some(Background::Color(
            palette.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.68
            } else {
                0.48
            }),
        )),
        button::Status::Active | button::Status::Disabled => Some(Background::Color(
            palette.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                0.42
            } else {
                0.30
            }),
        )),
    };
    button::Style {
        background,
        text_color: theme.palette().text,
        border: Border { radius: 999.0.into(), ..Border::default() },
        ..button::Style::default()
    }
}

pub(crate) fn details_section_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(
            palette.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                0.36
            } else {
                0.28
            }),
        )),
        border: Border { radius: 6.0.into(), ..Border::default() },
        ..container::Style::default()
    }
}

pub(crate) fn details_tooltip_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(Color::WHITE),
        background: Some(Background::Color(Color::from_rgba8(18, 18, 20, 0.96))),
        border: Border { width: 1.0, color: Color::WHITE.scale_alpha(0.10), radius: 8.0.into() },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.32),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        },
        ..container::Style::default()
    }
}

pub(crate) fn details_tab_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let palette = theme.extended_palette();
    let background = if selected {
        Some(Background::Color(palette.primary.weak.color.scale_alpha(if is_dark_theme(theme) {
            0.42
        } else {
            0.28
        })))
    } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        Some(Background::Color(
            palette.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.38
            } else {
                0.24
            }),
        ))
    } else {
        None
    };
    button::Style {
        background,
        text_color: theme.palette().text,
        border: Border {
            width: if selected { 1.0 } else { 0.0 },
            color: palette.primary.base.color.scale_alpha(if selected { 0.72 } else { 0.0 }),
            radius: 5.0.into(),
        },
        ..button::Style::default()
    }
}

pub(crate) fn details_label_color(theme: &Theme) -> Color {
    theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.62 } else { 0.56 })
}

pub(crate) fn timeline_block_style(
    theme: &Theme,
    status: button::Status,
    lane: TimelineLane,
    selected: bool,
) -> button::Style {
    let color = timeline_lane_color(theme, lane);
    let alpha = if selected {
        if is_dark_theme(theme) { 0.48 } else { 0.30 }
    } else {
        match status {
            button::Status::Hovered => {
                if is_dark_theme(theme) {
                    0.38
                } else {
                    0.24
                }
            }
            button::Status::Pressed => {
                if is_dark_theme(theme) {
                    0.54
                } else {
                    0.34
                }
            }
            button::Status::Active | button::Status::Disabled => {
                if is_dark_theme(theme) {
                    0.28
                } else {
                    0.18
                }
            }
        }
    };
    button::Style {
        background: Some(Background::Color(color.scale_alpha(alpha))),
        text_color: theme.palette().text,
        border: Border {
            width: if selected { 1.0 } else { 0.0 },
            color: color.scale_alpha(if selected { 0.92 } else { 0.0 }),
            radius: 4.0.into(),
        },
        ..button::Style::default()
    }
}

pub(crate) fn timeline_panel_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(
            palette.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                0.32
            } else {
                0.24
            }),
        )),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.44
            } else {
                0.28
            }),
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

pub(crate) fn timeline_track_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(
            palette.background.base.color.scale_alpha(if is_dark_theme(theme) {
                0.52
            } else {
                0.62
            }),
        )),
        border: Border { radius: 4.0.into(), ..Border::default() },
        ..container::Style::default()
    }
}

pub(crate) fn timeline_stat_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(
            palette.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.24
            } else {
                0.18
            }),
        )),
        border: Border { radius: 5.0.into(), ..Border::default() },
        ..container::Style::default()
    }
}

pub(crate) fn timeline_lane_text_color(theme: &Theme, lane: TimelineLane) -> Color {
    timeline_lane_color(theme, lane).scale_alpha(if is_dark_theme(theme) { 0.96 } else { 0.88 })
}

pub(crate) fn timeline_stat_label_color(theme: &Theme) -> Color {
    theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.60 } else { 0.54 })
}

pub(crate) fn timeline_stat_value_color(theme: &Theme) -> Color {
    theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.94 } else { 0.88 })
}

fn timeline_lane_color(theme: &Theme, lane: TimelineLane) -> Color {
    let palette = theme.extended_palette();
    match lane {
        TimelineLane::Input => palette.success.base.color,
        TimelineLane::Model => palette.primary.base.color,
        TimelineLane::Tools => palette.warning.base.color,
    }
}
