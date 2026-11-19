use desktop_ui_controls::system_settings_common::settings_panel_style;
use iced::{Border, Color, Theme, widget::container};

#[derive(Clone, Copy)]
pub(super) enum Tone {
    Neutral,
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

/// Group rows share the outer card border, retaining a subtle selection highlight.
pub(super) fn list_row(theme: &Theme, selected: bool) -> container::Style {
    container::Style {
        background: selected.then(|| theme.palette().primary.scale_alpha(0.08).into()),
        ..Default::default()
    }
}

pub(super) fn detail_surface(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.palette().text.scale_alpha(0.035).into()),
        ..Default::default()
    }
}
