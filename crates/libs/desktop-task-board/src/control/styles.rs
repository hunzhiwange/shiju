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

pub(super) fn faded_base_text_style(theme: &Theme, alpha: f32) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(theme.extended_palette().background.base.text.scale_alpha(alpha)),
    }
}

pub(super) fn worktree_title_text_style(theme: &Theme) -> iced::widget::text::Style {
    faded_base_text_style(theme, 0.8)
}

pub(super) fn worktree_summary_text_style(theme: &Theme) -> iced::widget::text::Style {
    faded_base_text_style(theme, 0.86)
}

pub(super) fn worktree_refreshing_text_style(theme: &Theme) -> iced::widget::text::Style {
    faded_base_text_style(theme, 0.72)
}

pub(super) fn worktree_action_text_style(theme: &Theme) -> iced::widget::text::Style {
    faded_base_text_style(theme, 0.82)
}

pub(super) fn worktree_slot_title_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.extended_palette().background.base.text) }
}

pub(super) fn worktree_slot_path_text_style(theme: &Theme) -> iced::widget::text::Style {
    faded_base_text_style(theme, 0.84)
}

pub(super) fn worktree_slot_taint_text_style(theme: &Theme) -> iced::widget::text::Style {
    faded_base_text_style(theme, 0.76)
}

pub(super) fn worktree_loading_text_style(theme: &Theme) -> iced::widget::text::Style {
    faded_base_text_style(theme, 0.78)
}

pub(super) fn confirm_hint_text_style(_theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(Color::from_rgb8(153, 27, 27)) }
}

pub(super) fn confirm_panel_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgba(
            254.0 / 255.0,
            226.0 / 255.0,
            226.0 / 255.0,
            0.92,
        ))),
        border: Border { width: 1.0, color: Color::from_rgb8(248, 113, 113), radius: 6.0.into() },
        ..Default::default()
    }
}

pub(super) fn worktree_action_panel_style(theme: &Theme) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(p.background.base.color.scale_alpha(0.22))),
        border: Border {
            width: 1.0,
            color: p.background.strong.color.scale_alpha(0.24),
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn worktree_slot_row_style(theme: &Theme) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(p.background.base.color.scale_alpha(0.35))),
        border: Border {
            width: 1.0,
            color: p.background.strong.color.scale_alpha(0.32),
            radius: 6.0.into(),
        },
        ..Default::default()
    }
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
