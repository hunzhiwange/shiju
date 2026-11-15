use iced::{Color, Theme};
use iced::widget::toggler;

/// 常规设置开关的共享样式。
pub(super) fn settings_toggle_style(theme: &Theme, status: toggler::Status) -> toggler::Style {
    let mut style = toggler::default(theme, status);
    // 保持开关滑块稳定可见，悬停时只改变轨道颜色。
    style.foreground = Color::WHITE.into();
    let palette = theme.extended_palette();
    match status {
        toggler::Status::Hovered { is_toggled: true } => {
            style.background = palette.primary.strong.color.into();
        }
        toggler::Status::Hovered { is_toggled: false } => {
            style.background = palette.background.strongest.color.into();
        }
        toggler::Status::Disabled { .. } => {
            style.foreground = Color::WHITE.scale_alpha(0.8).into();
        }
        toggler::Status::Active { .. } => {}
    }
    style
}

use super::theme::is_dark_theme;
use iced::theme::palette;
use iced::widget::{button, checkbox, pick_list, text_editor, text_input};
use iced::{Background, Border, Shadow, Vector};
pub fn settings_muted_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(if is_dark_theme(theme) {
            theme.palette().text.scale_alpha(0.72)
        } else {
            theme.palette().text.scale_alpha(0.62)
        }),
    }
}
pub fn settings_panel_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            palette.background.base.color.scale_alpha(0.92)
        } else {
            Color::from_rgba8(255, 255, 255, 0.88)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.9)
            } else {
                Color::from_rgba8(15, 23, 42, 0.08)
            },
            radius: 18.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.07 } else { 0.03 }),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 7.0,
        },
        snap: false,
        ..Default::default()
    }
}
pub fn settings_modal_backdrop_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark_theme(theme) {
            Color::from_rgba8(2, 6, 23, 0.70)
        } else {
            Color::from_rgba8(15, 23, 42, 0.30)
        })),
        ..Default::default()
    }
}
pub fn settings_modal_card_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            palette.background.base.color.scale_alpha(0.98)
        } else {
            Color::from_rgba8(255, 255, 255, 0.97)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.96)
            } else {
                theme.palette().primary.scale_alpha(0.10)
            },
            radius: 24.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.34 } else { 0.14 }),
            offset: Vector::new(0.0, 24.0),
            blur_radius: 52.0,
        },
        snap: false,
        ..Default::default()
    }
}
pub fn settings_text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let is_focused = matches!(status, text_input::Status::Focused { .. });
    let is_hovered = matches!(status, text_input::Status::Hovered)
        || matches!(status, text_input::Status::Focused { is_hovered: true });
    let is_disabled = matches!(status, text_input::Status::Disabled);

    let background = if is_disabled {
        extended.background.base.color.scale_alpha(if is_dark { 0.40 } else { 0.60 })
    } else if is_focused {
        if is_dark {
            extended.background.weak.color.scale_alpha(0.96)
        } else {
            Color::WHITE.scale_alpha(0.98)
        }
    } else if is_hovered {
        if is_dark {
            extended.background.base.color.scale_alpha(0.96)
        } else {
            Color::WHITE.scale_alpha(0.94)
        }
    } else if is_dark {
        extended.background.base.color.scale_alpha(0.84)
    } else {
        Color::WHITE.scale_alpha(0.90)
    };

    let border_color = if is_focused {
        palette.primary.scale_alpha(0.84)
    } else if is_hovered {
        extended.background.strong.color.scale_alpha(0.92)
    } else {
        let idle_border = if is_dark {
            extended.background.strong.color.scale_alpha(0.82)
        } else {
            Color::from_rgba8(15, 23, 42, 0.10)
        };
        if is_disabled { idle_border.scale_alpha(0.55) } else { idle_border }
    };

    text_input::Style {
        background: Background::Color(background),
        border: Border { width: 1.0, color: border_color, radius: 14.0.into() },
        icon: palette.text.scale_alpha(0.65),
        placeholder: palette.text.scale_alpha(0.50),
        value: if is_disabled { palette.text.scale_alpha(0.55) } else { palette.text },
        selection: palette.primary.scale_alpha(0.20),
    }
}
pub fn settings_text_editor_style(
    theme: &Theme,
    status: text_editor::Status,
) -> text_editor::Style {
    let input = settings_text_input_style(
        theme,
        match status {
            text_editor::Status::Active => text_input::Status::Active,
            text_editor::Status::Hovered => text_input::Status::Hovered,
            text_editor::Status::Focused { is_hovered } => {
                text_input::Status::Focused { is_hovered }
            }
            text_editor::Status::Disabled => text_input::Status::Disabled,
        },
    );
    text_editor::Style {
        background: input.background,
        border: input.border,
        placeholder: input.placeholder,
        value: input.value,
        selection: input.selection,
    }
}