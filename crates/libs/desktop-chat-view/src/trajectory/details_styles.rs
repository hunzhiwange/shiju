use iced::widget::{button, container, text_input};
use iced::widget::scrollable::Direction;
use iced::{Background, Border, Color, Font, Theme};
use super::model::TrajectoryRecordKind;
use super::json_tree::JsonTreeTokenKind;
use desktop_chat_tools::card_style::{chat_scroll_direction, is_dark_theme};
pub const DETAILS_SCROLLBAR_WIDTH: f32 = 4.0;
pub fn json_tree_token_color(theme: &Theme, kind: JsonTreeTokenKind) -> Color {
    let dark = is_dark_theme(theme);
    match kind {
        JsonTreeTokenKind::Punctuation => theme.palette().text.scale_alpha(0.82),
        JsonTreeTokenKind::Key => {
            if dark {
                Color::from_rgb8(196, 148, 255)
            } else {
                Color::from_rgb8(112, 66, 170)
            }
        }
        JsonTreeTokenKind::String => {
            if dark {
                Color::from_rgb8(244, 139, 152)
            } else {
                Color::from_rgb8(174, 48, 66)
            }
        }
        JsonTreeTokenKind::Number => {
            if dark {
                Color::from_rgb8(113, 190, 255)
            } else {
                Color::from_rgb8(26, 106, 174)
            }
        }
        JsonTreeTokenKind::Boolean => {
            if dark {
                Color::from_rgb8(240, 178, 100)
            } else {
                Color::from_rgb8(164, 92, 20)
            }
        }
        JsonTreeTokenKind::Null => theme.palette().text.scale_alpha(0.54),
    }
}

pub fn json_tree_action_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::Style {
        text_color: theme.palette().text.scale_alpha(0.66),
        ..button::Style::default()
    };
    if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        style.background = Some(Background::Color(
            theme.extended_palette().background.strong.color.scale_alpha(0.32),
        ));
    }
    style
}

pub fn details_scroll_direction() -> Direction {
    chat_scroll_direction()
}

pub fn role_badge_style(theme: &Theme, kind: TrajectoryRecordKind) -> container::Style {
    let palette = theme.extended_palette();
    let color = match kind {
        TrajectoryRecordKind::System => palette.secondary.base.color,
        TrajectoryRecordKind::User => palette.success.base.color,
        TrajectoryRecordKind::Context => palette.success.weak.color,
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

pub fn empty_text_color(theme: &Theme) -> Color {
    theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.54 } else { 0.48 })
}

pub fn details_panel_style(theme: &Theme) -> container::Style {
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

pub fn trajectory_search_input_style(
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

pub fn details_close_button_style(theme: &Theme, status: button::Status) -> button::Style {
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

pub fn details_section_style(theme: &Theme) -> container::Style {
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

pub fn tool_definition_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(
            palette.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                0.22
            } else {
                0.16
            }),
        )),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.42
            } else {
                0.26
            }),
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn tool_definition_header_style(theme: &Theme, status: button::Status) -> button::Style {
    let background =
        matches!(status, button::Status::Hovered | button::Status::Pressed).then(|| {
            Background::Color(
                theme
                    .extended_palette()
                    .background
                    .strong
                    .color
                    .scale_alpha(if is_dark_theme(theme) { 0.32 } else { 0.20 }),
            )
        });
    button::Style {
        background,
        text_color: theme.palette().text,
        border: Border { radius: 6.0.into(), ..Border::default() },
        ..button::Style::default()
    }
}

pub fn details_tab_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let palette = theme.extended_palette();
    let background = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
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
        text_color: theme.palette().text.scale_alpha(if selected { 1.0 } else { 0.68 }),
        border: Border { radius: 4.0.into(), ..Border::default() },
        ..button::Style::default()
    }
}

pub fn details_tab_underline_style(theme: &Theme, selected: bool) -> container::Style {
    container::Style {
        background: selected
            .then(|| Background::Color(theme.extended_palette().primary.base.color)),
        border: Border { radius: 1.0.into(), ..Border::default() },
        ..container::Style::default()
    }
}

pub fn details_label_color(theme: &Theme) -> Color {
    theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.62 } else { 0.56 })
}
