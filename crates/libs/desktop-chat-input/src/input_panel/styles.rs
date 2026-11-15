use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme};
fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

pub fn square_icon_button_style(
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
    let idle_border = if is_dark {
        Color::from_rgba8(45, 48, 54, 0.95)
    } else {
        Color::from_rgba8(226, 231, 237, 1.0)
    };
    let active_border = if is_dark {
        Color::from_rgba8(63, 67, 75, 0.95)
    } else {
        Color::from_rgba8(210, 216, 224, 1.0)
    };
    let (background, border_color) = match status {
        button::Status::Pressed => (pressed_bg, active_border),
        button::Status::Hovered => (hover_bg, active_border),
        _ => (idle_bg, idle_border),
    };
    button::Style {
        background: Some(Background::Color(background)),
        border: Border { radius: 10.0.into(), width: 1.0, color: border_color },
        text_color: if enabled {
            theme.palette().text.scale_alpha(if is_dark { 0.92 } else { 0.88 })
        } else {
            theme.palette().text.scale_alpha(if is_dark { 0.36 } else { 0.42 })
        },
        ..Default::default()
    }
}

pub fn selector_label_font() -> iced::Font {
    iced::Font { weight: iced::font::Weight::Medium, ..Default::default() }
}

pub fn selector_text_color(theme: &Theme, highlighted: bool) -> Color {
    if highlighted {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.97 } else { 0.95 })
    } else {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.92 } else { 0.88 })
    }
}

pub fn selector_chevron_color(theme: &Theme, highlighted: bool) -> Color {
    if highlighted {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.72 } else { 0.68 })
    } else {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.58 } else { 0.62 })
    }
}

pub fn selector_pill_button_style(
    theme: &Theme,
    status: button::Status,
    highlighted: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let idle_bg = if highlighted {
        if is_dark {
            Color::from_rgba8(28, 30, 35, 0.97)
        } else {
            Color::from_rgba8(244, 246, 249, 1.0)
        }
    } else if is_dark {
        Color::from_rgba8(24, 25, 29, 0.92)
    } else {
        Color::from_rgba8(248, 249, 251, 1.0)
    };
    let hover_bg = if highlighted {
        if is_dark {
            Color::from_rgba8(34, 36, 42, 0.98)
        } else {
            Color::from_rgba8(238, 241, 245, 1.0)
        }
    } else if is_dark {
        Color::from_rgba8(31, 33, 38, 0.96)
    } else {
        Color::from_rgba8(241, 243, 246, 1.0)
    };
    let pressed_bg = if highlighted {
        if is_dark {
            Color::from_rgba8(38, 40, 46, 1.0)
        } else {
            Color::from_rgba8(232, 236, 241, 1.0)
        }
    } else if is_dark {
        Color::from_rgba8(36, 38, 44, 0.98)
    } else {
        Color::from_rgba8(233, 236, 241, 1.0)
    };
    let idle_border = if highlighted {
        if is_dark {
            Color::from_rgba8(70, 74, 83, 0.96)
        } else {
            Color::from_rgba8(206, 213, 223, 1.0)
        }
    } else if is_dark {
        Color::from_rgba8(45, 48, 54, 0.92)
    } else {
        Color::from_rgba8(226, 231, 237, 1.0)
    };
    let active_border = if highlighted {
        if is_dark {
            Color::from_rgba8(84, 88, 98, 0.98)
        } else {
            Color::from_rgba8(194, 202, 214, 1.0)
        }
    } else if is_dark {
        Color::from_rgba8(63, 67, 75, 0.95)
    } else {
        Color::from_rgba8(210, 216, 224, 1.0)
    };
    let (background, border_color) = match status {
        button::Status::Pressed => (pressed_bg, active_border),
        button::Status::Hovered => (hover_bg, active_border),
        _ => (idle_bg, idle_border),
    };

    button::Style {
        background: Some(Background::Color(background)),
        border: Border { radius: 12.0.into(), width: 1.0, color: border_color },
        text_color: selector_text_color(theme, highlighted),
        shadow: if highlighted {
            iced::Shadow {
                color: Color::BLACK.scale_alpha(if is_dark { 0.10 } else { 0.035 }),
                offset: iced::Vector::new(0.0, 6.0),
                blur_radius: 16.0,
            }
        } else {
            iced::Shadow::default()
        },
        ..Default::default()
    }
}

pub fn selectable_list_button_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let hovered = matches!(status, button::Status::Hovered);
    let pressed = matches!(status, button::Status::Pressed);
    let neutral_hover = if is_dark {
        Color::from_rgba8(31, 33, 38, 0.94)
    } else {
        Color::from_rgba8(241, 243, 246, 1.0)
    };
    let selected_bg = Color::from_rgba(
        theme.palette().primary.r,
        theme.palette().primary.g,
        theme.palette().primary.b,
        if is_dark { 0.15 } else { 0.10 },
    );
    let selected_hover_bg = Color::from_rgba(
        theme.palette().primary.r,
        theme.palette().primary.g,
        theme.palette().primary.b,
        if is_dark { 0.19 } else { 0.13 },
    );
    let background = if pressed {
        Some(Background::Color(if selected { selected_hover_bg } else { neutral_hover }))
    } else if hovered {
        Some(Background::Color(if selected { selected_hover_bg } else { neutral_hover }))
    } else if selected {
        Some(Background::Color(selected_bg))
    } else {
        None
    };
    let border_color = if selected {
        theme.palette().primary.scale_alpha(if is_dark { 0.42 } else { 0.24 })
    } else if hovered || pressed {
        if is_dark {
            Color::from_rgba8(58, 61, 69, 0.92)
        } else {
            Color::from_rgba8(218, 223, 230, 1.0)
        }
    } else {
        Color::TRANSPARENT
    };

    button::Style {
        background,
        border: Border {
            radius: 8.0.into(),
            width: if selected || hovered || pressed { 1.0 } else { 0.0 },
            color: border_color,
        },
        text_color: if selected {
            selector_text_color(theme, true)
        } else {
            theme.palette().text.scale_alpha(if is_dark { 0.90 } else { 0.88 })
        },
        ..Default::default()
    }
}

pub fn popover_style(theme: &Theme) -> container::Style {
    let is_dark = is_dark_theme(theme);
    let background = if is_dark {
        Color::from_rgba8(20, 21, 24, 0.985)
    } else {
        Color::from_rgba8(252, 252, 253, 0.985)
    };
    let border = if is_dark {
        Color::from_rgba8(44, 47, 53, 0.96)
    } else {
        Color::from_rgba8(226, 231, 237, 1.0)
    };
    container::Style {
        background: Some(Background::Color(background)),
        border: Border { radius: 14.0.into(), width: 1.0, color: border },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.22 } else { 0.08 }),
            offset: iced::Vector::new(0.0, 10.0),
            blur_radius: 28.0,
        },
        ..Default::default()
    }
}
