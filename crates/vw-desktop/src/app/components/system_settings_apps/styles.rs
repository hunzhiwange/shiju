use crate::app::components::system_settings_common::settings_panel_style;
use iced::{Border, Color, Theme, widget::container};

#[derive(Clone, Copy)]
pub(super) enum Tone {
    Neutral,
    Success,
    Warning,
}

pub(super) fn card(theme: &Theme, selected: bool) -> container::Style {
    let mut style = settings_panel_style(theme);
    if selected {
        style.border.color = theme.palette().primary;
    }
    style
}

pub(super) fn badge(theme: &Theme, tone: Tone) -> container::Style {
    let color = match tone {
        Tone::Neutral => theme.palette().text.scale_alpha(0.66),
        Tone::Success if theme.extended_palette().is_dark => Color::from_rgb8(117, 221, 176),
        Tone::Success => Color::from_rgb8(22, 109, 75),
        Tone::Warning if theme.extended_palette().is_dark => Color::from_rgb8(239, 196, 114),
        Tone::Warning => Color::from_rgb8(139, 83, 14),
    };
    container::Style {
        text_color: Some(color),
        background: Some(color.scale_alpha(0.10).into()),
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    }
}
