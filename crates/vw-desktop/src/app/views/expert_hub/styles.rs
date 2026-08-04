use iced::widget::{button, container, text_input};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

fn is_dark(theme: &Theme) -> bool {
    let background = theme.palette().background;
    background.r + background.g + background.b < 1.5
}

pub(super) fn card_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(if is_dark(theme) {
            palette.background.weak.color
        } else {
            Color::WHITE
        })),
        text_color: Some(theme.palette().text),
        border: Border {
            color: palette.background.strong.color.scale_alpha(if is_dark(theme) {
                0.72
            } else {
                0.28
            }),
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark(theme) { 0.12 } else { 0.04 }),
            offset: Vector::new(0.0, 7.0),
            blur_radius: 20.0,
        },
        ..Default::default()
    }
}

pub(super) fn featured_card_style(theme: &Theme) -> container::Style {
    let mut style = card_style(theme);
    let primary = theme.palette().primary;
    style.border.color = primary.scale_alpha(if is_dark(theme) { 0.55 } else { 0.28 });
    style
}

pub(super) fn detail_panel_style(theme: &Theme) -> container::Style {
    card_style(theme)
}

pub(super) fn inset_panel_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        text_color: Some(palette.background.weak.text),
        border: Border {
            color: palette.background.strong.color.scale_alpha(0.44),
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn meta_badge_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.strong.color.scale_alpha(0.58))),
        text_color: Some(palette.background.strong.text),
        border: Border { radius: 999.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub(super) fn muted_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.64)) }
}

pub(super) fn search_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.extended_palette();
    let focused = matches!(status, text_input::Status::Focused { .. });
    let disabled = matches!(status, text_input::Status::Disabled);
    text_input::Style {
        background: Background::Color(if is_dark(theme) {
            palette.background.weak.color
        } else {
            Color::WHITE
        }),
        border: Border {
            color: if focused {
                palette.primary.base.color
            } else {
                palette.background.strong.color.scale_alpha(0.64)
            },
            width: if focused { 1.5 } else { 1.0 },
            radius: 12.0.into(),
        },
        icon: theme.palette().text.scale_alpha(0.64),
        placeholder: theme.palette().text.scale_alpha(0.50),
        value: theme.palette().text.scale_alpha(if disabled { 0.48 } else { 1.0 }),
        selection: theme.palette().primary.scale_alpha(0.22),
    }
}

pub(super) fn skill_badge_style(theme: &Theme) -> container::Style {
    let primary = theme.palette().primary;
    container::Style {
        text_color: Some(primary),
        background: Some(Background::Color(primary.scale_alpha(if is_dark(theme) {
            0.18
        } else {
            0.10
        }))),
        border: Border { radius: 999.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub(super) fn divider_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(
            theme.extended_palette().background.strong.color.scale_alpha(0.42),
        )),
        ..Default::default()
    }
}

pub(super) fn selected_tab_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let palette = theme.extended_palette();
    let pair = if selected {
        match status {
            button::Status::Hovered => palette.primary.base,
            button::Status::Pressed => palette.primary.strong,
            _ => palette.primary.strong,
        }
    } else {
        match status {
            button::Status::Hovered | button::Status::Pressed => palette.background.strong,
            _ => palette.background.weak,
        }
    };
    button::Style {
        background: Some(pair.color.into()),
        text_color: if selected { Color::WHITE } else { pair.text },
        border: Border {
            color: if selected {
                palette.primary.base.color
            } else {
                palette.background.strong.color
            },
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn action_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::primary(theme, status);
    style.text_color = Color::WHITE;
    style.border.radius = 8.0.into();
    style
}

pub(super) fn quiet_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let pair = match status {
        button::Status::Hovered | button::Status::Pressed => palette.background.strong,
        _ => palette.background.weak,
    };
    button::Style {
        background: Some(pair.color.into()),
        text_color: pair.text,
        border: Border {
            color: palette.background.strong.color.scale_alpha(0.48),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn pill_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::primary(theme, status);
    style.border.radius = 999.0.into();
    style
}
