use iced::{Background, Border, Color, Theme, widget::container};

#[derive(Clone, Copy)]
pub(super) enum Tone {
    Neutral,
    Success,
    Warning,
}

pub(super) fn dark(theme: &Theme) -> bool {
    let c = theme.palette().background;
    c.r + c.g + c.b < 1.5
}

pub(super) fn card(theme: &Theme, selected: bool) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(if dark(theme) {
            palette.background.weak.color.scale_alpha(0.42)
        } else {
            Color::WHITE.scale_alpha(0.88)
        })),
        border: Border {
            radius: 14.0.into(),
            width: 1.0,
            color: if selected {
                theme.palette().primary
            } else if dark(theme) {
                theme.palette().text.scale_alpha(0.10)
            } else {
                Color::from_rgba8(20, 30, 50, 0.09)
            },
        },
        ..Default::default()
    }
}

pub(super) fn badge(theme: &Theme, tone: Tone) -> container::Style {
    let color = match tone {
        Tone::Neutral => theme.palette().text.scale_alpha(0.66),
        Tone::Success if dark(theme) => Color::from_rgb8(117, 221, 176),
        Tone::Success => Color::from_rgb8(22, 109, 75),
        Tone::Warning if dark(theme) => Color::from_rgb8(239, 196, 114),
        Tone::Warning => Color::from_rgb8(139, 83, 14),
    };
    container::Style {
        text_color: Some(color),
        background: Some(color.scale_alpha(0.10).into()),
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub(super) fn icon_tile(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.palette().text.scale_alpha(0.045).into()),
        border: Border { radius: 12.0.into(), ..Default::default() },
        ..Default::default()
    }
}
