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
    let mut style = super::styles::selectable_list_button_style(theme, status, is_action);
    style.border.radius = 9.0.into();
    style
}

/// 快捷消息分组入口样式。
pub fn quick_message_group_button_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let mut style = super::styles::selector_pill_button_style(theme, status, active);
    if style.background.is_none() {
        style.background = Some(Background::Color(theme.palette().text.scale_alpha(0.055)));
    }
    style
}

pub use super::styles::selectable_list_button_style as shiju_skill_button_style;

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
            width: if enabled { 1.0 } else { Default::default() },
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

pub use super::styles::popover_style as quick_message_menu_style;

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
