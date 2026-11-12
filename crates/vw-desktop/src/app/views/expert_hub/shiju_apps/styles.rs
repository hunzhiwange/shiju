//! 提供识矩应用页面在明暗主题下的组件样式。

use iced::widget::{button, container, text};
use iced::{Background, Border, Theme};

/// 识矩应用页面统一使用的滚动条宽度。
pub(crate) const SCROLLBAR_WIDTH: u32 = 4;
/// 滚动内容两侧预留与滚动条等宽的空间。
pub(crate) const SCROLLABLE_SIDE_PADDING: u16 = SCROLLBAR_WIDTH as u16;

/// 根据选中与悬停状态生成明暗主题均可读的页签样式。
pub(super) fn tab_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let palette = theme.extended_palette();
    let active =
        if selected { palette.primary.strong.color } else { palette.background.weak.color };
    let background = match status {
        button::Status::Hovered => palette.background.strong.color,
        _ => active,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: if selected { palette.primary.strong.text } else { theme.palette().text },
        border: Border { radius: 8.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub(super) fn quiet_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered => palette.background.strong.color,
        _ => palette.background.weak.color,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: theme.palette().text,
        border: Border { color: palette.background.strong.color, width: 1.0, radius: 7.0.into() },
        ..Default::default()
    }
}

pub(super) fn card_style(theme: &Theme) -> container::Style {
    super::super::styles::card_style(theme)
}

pub(super) fn badge_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.primary.weak.color)),
        text_color: Some(palette.primary.weak.text),
        border: Border { radius: 10.0.into(), ..Default::default() },
        ..Default::default()
    }
}

/// 用成功色或弱化色区分应用是否运行就绪。
pub(super) fn status_style(theme: &Theme, ready: bool) -> container::Style {
    let palette = theme.extended_palette();
    let pair = if ready { palette.success.weak } else { palette.danger.weak };
    container::Style {
        background: Some(Background::Color(pair.color)),
        text_color: Some(pair.text),
        border: Border { radius: 999.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub(super) fn muted_text_style(theme: &Theme) -> text::Style {
    text::Style { color: Some(theme.extended_palette().background.strong.text) }
}

pub(super) fn binding_field_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        text_color: Some(theme.palette().text),
        border: Border { color: palette.background.strong.color, width: 1.0, radius: 9.0.into() },
        ..Default::default()
    }
}

/// 根据密钥绑定状态是否出错选择反馈容器颜色。
pub(super) fn binding_state_style(theme: &Theme, error: bool) -> container::Style {
    let palette = theme.extended_palette();
    let pair = if error { palette.danger.weak } else { palette.primary.weak };
    container::Style {
        background: Some(Background::Color(pair.color)),
        text_color: Some(pair.text),
        border: Border { radius: 999.0.into(), ..Default::default() },
        ..Default::default()
    }
}
