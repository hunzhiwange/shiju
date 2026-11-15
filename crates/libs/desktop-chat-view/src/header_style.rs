use iced::widget::button;
use iced::{Background, Border, Color, Theme};
use crate::presentation::is_dark_theme;
pub fn round_icon_button_style(
    theme: &Theme,
    status: button::Status,
    enabled: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let idle_bg = if is_dark {
        Color::from_rgba8(24, 25, 29, 0.92)
    } else {
        Color::from_rgba8(247, 248, 250, 1.0)
    };
    let hover_bg = if is_dark {
        Color::from_rgba8(31, 33, 38, 0.96)
    } else {
        Color::from_rgba8(241, 243, 246, 1.0)
    };
    let pressed_bg = if is_dark {
        Color::from_rgba8(36, 38, 44, 0.98)
    } else {
        Color::from_rgba8(232, 236, 241, 1.0)
    };
    let background = match status {
        button::Status::Pressed => pressed_bg,
        button::Status::Hovered => hover_bg,
        _ => idle_bg,
    };

    button::Style {
        background: Some(Background::Color(background)),
        border: Border { radius: 999.0.into(), width: 0.0, color: Color::TRANSPARENT },
        text_color: if enabled {
            theme.palette().text.scale_alpha(if is_dark { 0.92 } else { 0.88 })
        } else {
            theme.palette().text.scale_alpha(if is_dark { 0.36 } else { 0.42 })
        },
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

pub fn header_surface_style(
    _theme: &Theme,
    chrome: Option<Color>,
) -> iced::widget::container::Style {
    // 默认透明；显式/合成的 chat_header_bg 提供分区透明度色层。
    iced::widget::container::Style {
        background: chrome.map(Background::Color),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
        ..Default::default()
    }
}

pub fn theme_is_dark(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

pub fn chat_mode_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    active: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(
        status,
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
    );
    let background = if active {
        palette.primary.weak.color.scale_alpha(if theme_is_dark(theme) { 0.38 } else { 0.24 })
    } else if hovered {
        palette.background.strong.color.scale_alpha(if theme_is_dark(theme) { 0.42 } else { 0.28 })
    } else {
        Color::TRANSPARENT
    };
    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        border: Border {
            width: 1.0,
            color: if active {
                palette.primary.base.color.scale_alpha(0.66)
            } else {
                palette.background.strong.color.scale_alpha(0.42)
            },
            radius: 5.0.into(),
        },
        text_color: if active {
            if theme_is_dark(theme) {
                theme.palette().text.scale_alpha(0.96)
            } else {
                palette.primary.base.color
            }
        } else {
            theme.palette().text.scale_alpha(0.72)
        },
        ..Default::default()
    }
}

pub fn return_to_parent_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let is_dark = theme_is_dark(theme);
    let p = theme.extended_palette();
    let (bg, border, fg) = match status {
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed => {
            if is_dark {
                (
                    p.primary.base.color.scale_alpha(0.22),
                    p.primary.base.color.scale_alpha(0.40),
                    theme.palette().text.scale_alpha(0.96),
                )
            } else {
                (
                    p.primary.base.color.scale_alpha(0.14),
                    p.primary.base.color.scale_alpha(0.28),
                    p.primary.base.color.scale_alpha(0.90),
                )
            }
        }
        _ => {
            if is_dark {
                (
                    p.primary.base.color.scale_alpha(0.14),
                    p.primary.base.color.scale_alpha(0.28),
                    theme.palette().text.scale_alpha(0.92),
                )
            } else {
                (
                    p.primary.base.color.scale_alpha(0.08),
                    p.primary.base.color.scale_alpha(0.18),
                    p.primary.base.color.scale_alpha(0.82),
                )
            }
        }
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        border: Border { width: 1.0, color: border, radius: 999.0.into() },
        text_color: fg,
        ..Default::default()
    }
}

pub fn session_workspace_badge_style(
    theme: &Theme,
    is_primary: bool,
) -> iced::widget::container::Style {
    let is_dark = theme_is_dark(theme);
    let p = theme.extended_palette();
    let (bg, border, fg) = if is_primary {
        if is_dark {
            (
                Color::from_rgba8(24, 25, 29, 0.96),
                Color::from_rgba8(45, 48, 54, 0.94),
                theme.palette().text.scale_alpha(0.92),
            )
        } else {
            (
                Color::from_rgb8(247, 248, 250),
                Color::from_rgb8(226, 231, 237),
                Color::from_rgb8(75, 85, 99),
            )
        }
    } else if is_dark {
        (
            p.primary.base.color.scale_alpha(0.14),
            p.primary.base.color.scale_alpha(0.28),
            p.primary.base.text.scale_alpha(0.92),
        )
    } else {
        (
            p.primary.base.color.scale_alpha(0.08),
            p.primary.base.color.scale_alpha(0.18),
            p.primary.base.color.scale_alpha(0.82),
        )
    };
    iced::widget::container::Style {
        background: Some(Background::Color(bg)),
        border: Border { width: 1.0, color: border, radius: 999.0.into() },
        text_color: Some(fg),
        ..Default::default()
    }
}

pub fn colored_text_style(color: Color) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(color) }
}

pub fn running_status_badge_style(color: Color) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(color.scale_alpha(0.16))),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
        text_color: Some(color),
        ..Default::default()
    }
}

pub fn queued_status_badge_style() -> iced::widget::container::Style {
    let color = Color::from_rgb8(245, 158, 11);
    iced::widget::container::Style {
        background: Some(Background::Color(color.scale_alpha(0.16))),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
        text_color: Some(color),
        ..Default::default()
    }
}

pub fn session_actions_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let is_dark = theme_is_dark(theme);
    let bg = match status {
        iced::widget::button::Status::Hovered => {
            if is_dark {
                Color::from_rgba8(31, 33, 38, 0.96)
            } else {
                Color::from_rgba8(241, 243, 246, 1.0)
            }
        }
        iced::widget::button::Status::Pressed => {
            if is_dark {
                Color::from_rgba8(36, 38, 44, 0.98)
            } else {
                Color::from_rgba8(232, 236, 241, 1.0)
            }
        }
        _ => Color::TRANSPARENT,
    };

    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: if is_dark {
                Color::from_rgba8(45, 48, 54, 0.72)
            } else {
                Color::from_rgba8(226, 231, 237, 0.92)
            },
        },
        text_color: theme.palette().text.scale_alpha(0.92),
        ..Default::default()
    }
}

pub fn session_actions_popover_style(theme: &Theme) -> iced::widget::container::Style {
    let is_dark = theme_is_dark(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba8(20, 21, 24, 0.985)
        } else {
            Color::from_rgba8(252, 252, 253, 1.0)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                Color::from_rgba8(44, 47, 53, 0.96)
            } else {
                Color::from_rgba8(226, 231, 237, 1.0)
            },
            radius: 12.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.20 } else { 0.08 }),
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        ..Default::default()
    }
}
