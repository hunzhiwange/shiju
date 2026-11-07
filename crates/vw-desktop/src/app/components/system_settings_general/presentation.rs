//! 常规设置标签的选中与悬停样式。

use crate::app::components::system_settings_common::rounded_action_btn_style;
use iced::widget::button;
use iced::{Border, Theme};

pub(super) fn tab_style(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    let palette = theme.extended_palette();
    style.border = Border { radius: 10.0.into(), ..Default::default() };
    style.background = if active || matches!(status, button::Status::Pressed) {
        Some(palette.background.strong.color.scale_alpha(0.5).into())
    } else if matches!(status, button::Status::Hovered) {
        Some(palette.background.weak.color.into())
    } else {
        None
    };
    style.text_color = if active { theme.palette().text } else { palette.background.strong.text };
    style
}
