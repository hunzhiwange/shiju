use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme};
fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

/// 快捷消息条中的紧凑按钮样式。
pub fn quick_message_button_style(
    theme: &Theme,
    status: button::Status,
    is_action: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let palette = theme.extended_palette();
    let neutral =
        if is_dark { palette.background.weak.color } else { Color::from_rgba8(246, 248, 251, 1.0) };
    let action = if is_dark {
        palette.primary.weak.color.scale_alpha(0.72)
    } else {
        palette.primary.weak.color.scale_alpha(0.58)
    };
    let base = if is_action { action } else { neutral };
    let background = match status {
        button::Status::Hovered => base.scale_alpha(if is_dark { 0.98 } else { 0.94 }),
        button::Status::Pressed => {
            if is_action {
                palette.primary.base.color.scale_alpha(if is_dark { 0.55 } else { 0.35 })
            } else {
                palette.background.strong.color.scale_alpha(if is_dark { 0.82 } else { 0.48 })
            }
        }
        _ => base.scale_alpha(if is_dark { 0.82 } else { 0.78 }),
    };

    button::Style {
        background: Some(Background::Color(background)),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: if is_action {
                palette.primary.base.color.scale_alpha(if is_dark { 0.58 } else { 0.38 })
            } else {
                palette.background.strong.color.scale_alpha(if is_dark { 0.88 } else { 0.52 })
            },
        },
        text_color: theme.palette().text.scale_alpha(if is_dark { 0.94 } else { 0.88 }),
        ..Default::default()
    }
}

/// 快捷消息分组入口样式。
pub fn quick_message_group_button_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let palette = theme.extended_palette();
    let emphasized = active || matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: emphasized
            .then(|| Background::Color(palette.background.weak.color.scale_alpha(0.82))),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if emphasized { 0.78 } else { 0.5 }),
        },
        text_color: theme.palette().text.scale_alpha(0.9),
        ..Default::default()
    }
}

/// 识矩技能菜单中的单选行样式。
pub fn shiju_skill_button_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let palette = theme.extended_palette();
    let pointed = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let background = if selected {
        Some(Background::Color(theme.palette().primary.scale_alpha(if pointed {
            if is_dark { 0.20 } else { 0.14 }
        } else if is_dark {
            0.16
        } else {
            0.10
        })))
    } else if pointed {
        Some(Background::Color(palette.background.weak.color.scale_alpha(if is_dark {
            0.92
        } else {
            0.78
        })))
    } else {
        None
    };
    let outlined = selected || pointed;

    button::Style {
        background,
        border: Border {
            radius: 7.0.into(),
            width: if outlined { 1.0 } else { 0.0 },
            color: if selected {
                theme.palette().primary.scale_alpha(if is_dark { 0.46 } else { 0.28 })
            } else if pointed {
                palette.background.strong.color.scale_alpha(if is_dark { 0.80 } else { 0.55 })
            } else {
                Color::TRANSPARENT
            },
        },
        text_color: theme.palette().text.scale_alpha(if is_dark { 0.92 } else { 0.88 }),
        ..Default::default()
    }
}

/// 识矩版本差异导航按钮样式。
pub fn shiju_upgrade_button_style(
    theme: &Theme,
    status: button::Status,
    enabled: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let palette = theme.extended_palette();
    let pointed = enabled && matches!(status, button::Status::Hovered | button::Status::Pressed);
    let background = enabled.then(|| {
        Background::Color(palette.primary.weak.color.scale_alpha(if pointed {
            if is_dark { 0.82 } else { 0.62 }
        } else if is_dark {
            0.66
        } else {
            0.48
        }))
    });

    button::Style {
        background,
        border: Border {
            radius: 7.0.into(),
            width: enabled.then_some(1.0).unwrap_or_default(),
            color: if enabled {
                palette.primary.base.color.scale_alpha(if is_dark { 0.58 } else { 0.38 })
            } else {
                Color::TRANSPARENT
            },
        },
        text_color: theme.palette().text.scale_alpha(if enabled {
            0.94
        } else if is_dark {
            0.36
        } else {
            0.42
        }),
        ..Default::default()
    }
}

/// 快捷消息悬停菜单表面。
pub fn quick_message_menu_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.86),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.2),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

/// 创建新会话动作标记。
pub fn quick_message_action_badge_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(palette.primary.base.color),
        background: Some(Background::Color(palette.primary.weak.color.scale_alpha(0.48))),
        border: Border { radius: 999.0.into(), ..Default::default() },
        ..Default::default()
    }
}
