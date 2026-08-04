use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

pub(crate) const SCROLLBAR_WIDTH: u32 = 4;

pub(super) fn tab_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let palette = theme.extended_palette();
    let active = if selected {
        palette.primary.strong.color
    } else {
        palette.background.weak.color
    };
    let background = match status {
        button::Status::Hovered => palette.background.strong.color,
        _ => active,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: if selected { palette.primary.strong.text } else { theme.palette().text },
        border: Border { radius: 8.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub(super) fn quiet_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered => palette.background.strong.color,
        _ => palette.background.weak.color,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: theme.palette().text,
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 7.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn card_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        text_color: Some(theme.palette().text),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if palette.is_dark { 0.22 } else { 0.08 }),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        ..Default::default()
    }
}

pub(super) fn badge_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.primary.weak.color)),
        text_color: Some(palette.primary.weak.text),
        border: Border { radius: 10.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub(super) fn status_style(theme: &Theme, ready: bool) -> container::Style {
    let palette = theme.extended_palette();
    let pair = if ready { palette.success.weak } else { palette.danger.weak };
    container::Style {
        background: Some(Background::Color(pair.color)),
        text_color: Some(pair.text),
        border: Border { radius: 999.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub(super) fn muted_text_style(theme: &Theme) -> text::Style {
    text::Style { color: Some(theme.extended_palette().background.strong.text) }
}
