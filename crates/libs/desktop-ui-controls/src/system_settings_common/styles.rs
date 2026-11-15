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
