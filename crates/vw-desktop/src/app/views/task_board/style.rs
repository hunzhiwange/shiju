use desktop_ui_controls::system_settings_common::settings_segment_button_style;
use iced::widget::{button, container};
use iced::{Border, Color, Theme};

pub(super) fn rail_surface(theme: &Theme) -> container::Style {
    surface(theme, 0.055)
}

pub(super) fn sidebar_surface(theme: &Theme) -> container::Style {
    surface(theme, 0.025)
}

pub(super) fn sidebar_view_surface(theme: &Theme, selected: bool, hovered: bool) -> container::Style {
    let ink = theme.palette().text;
    container::Style {
        background: if selected {
            Some(ink.scale_alpha(0.085).into())
        } else if hovered {
            Some(ink.scale_alpha(0.05).into())
        } else {
            None
        },
        border: iced::border::rounded(8),
        ..Default::default()
    }
}

pub(super) fn filter_chip_surface(theme: &Theme) -> container::Style {
    let mut style = surface(theme, 0.012);
    style.border =
        Border { color: theme.palette().text.scale_alpha(0.09), width: 1.0, radius: 7.0.into() };
    style
}

pub(super) fn filter_bar_surface(theme: &Theme) -> container::Style {
    let mut style = surface(theme, 0.033);
    style.border.radius = 9.0.into();
    style
}

pub(super) fn filter_group_surface(theme: &Theme) -> container::Style {
    let mut style = surface(theme, 0.032);
    style.border = Border {
        color: theme.palette().text.scale_alpha(0.10),
        width: 1.0,
        radius: 9.0.into(),
    };
    style
}

fn surface(theme: &Theme, tint: f32) -> container::Style {
    let palette = theme.palette();
    let background = palette.background;
    let ink = palette.text;
    container::Style {
        background: Some(
            Color::from_rgb(
                background.r + (ink.r - background.r) * tint,
                background.g + (ink.g - background.g) * tint,
                background.b + (ink.b - background.b) * tint,
            )
            .into(),
        ),
        ..Default::default()
    }
}

pub(super) fn navigation_button(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let mut style = settings_segment_button_style(theme, status, selected);
    let palette = theme.palette();
    style.background = match status {
        button::Status::Pressed => Some(palette.text.scale_alpha(0.10).into()),
        button::Status::Hovered if !selected => Some(palette.text.scale_alpha(0.06).into()),
        _ if selected => Some(palette.primary.scale_alpha(0.15).into()),
        _ => None,
    };
    style.text_color = palette.text.scale_alpha(if selected { 1.0 } else { 0.76 });
    style.border = Border { radius: 8.0.into(), ..Default::default() };
    style
}
