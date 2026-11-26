use iced::{Background, Border, Color, Theme};
use desktop_ui_controls::system_settings_common::settings_panel_style;

pub(super) fn status_chip_style(theme: &Theme, tone: Color) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(if tone.a > 0.0 {
            tone.scale_alpha(0.14)
        } else {
            palette.background.weak.color
        })),
        border: Border {
            width: 1.0,
            color: if tone.a > 0.0 {
                tone.scale_alpha(0.30)
            } else {
                palette.background.strong.color.scale_alpha(0.68)
            },
            radius: 999.0.into(),
        },
        text_color: Some(if tone.a > 0.0 { tone } else { theme.palette().text.scale_alpha(0.72) }),
        ..Default::default()
    }
}

pub(super) fn rounded_settings_panel_style(theme: &Theme, radius: f32) -> iced::widget::container::Style {
    let mut style = settings_panel_style(theme);
    style.border.radius = radius.into();
    style
}

pub(super) fn top_controls_panel_style(theme: &Theme) -> iced::widget::container::Style {
    rounded_settings_panel_style(theme, 24.0)
}

pub(super) fn control_outer_panel_style(theme: &Theme) -> iced::widget::container::Style {
    rounded_settings_panel_style(theme, 22.0)
}

pub(super) fn worktree_status_panel_style(theme: &Theme) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(p.background.weak.color.scale_alpha(0.35))),
        border: Border {
            width: 1.0,
            color: p.background.strong.color.scale_alpha(0.4),
            radius: 8.0.into(),
        },
        text_color: Some(Color::from_rgba(0.95, 0.97, 1.0, 0.92)),
        ..Default::default()
    }
}
