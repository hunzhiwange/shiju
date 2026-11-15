use super::*;

pub fn floating_panel_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        border: Border { width: 1.0, color: palette.background.weak.color, radius: 12.0.into() },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(0.12),
            offset: Vector::new(0.0, 10.0),
            blur_radius: 22.0,
        },
        ..Default::default()
    }
}

pub fn root_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark_theme(theme) {
            palette.background.base.color.scale_alpha(0.98)
        } else {
            Color::from_rgba8(244, 247, 252, 1.0)
        })),
        ..Default::default()
    }
}

pub fn modal_card_style(theme: &Theme) -> iced::widget::container::Style {
    settings_modal_card_style(theme)
}

pub fn value_card_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            palette.background.base.color.scale_alpha(0.62)
        } else {
            Color::WHITE.scale_alpha(0.90)
        })),
        border: Border {
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.72)
            } else {
                Color::from_rgba8(15, 23, 42, 0.06)
            },
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.04 } else { 0.02 }),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        ..Default::default()
    }
}
