//! 任务看板面板样式集合，统一维护按钮、弹层和容器的视觉规则。

use iced::widget::{button, container, text, text_editor, text_input};
use iced::{Background, Border, Color, Theme};


use desktop_ui_controls::system_settings_common::{
    settings_muted_text_style, settings_panel_style,
    settings_text_editor_style,
    settings_text_input_style,
};
use desktop_task_model::SubTaskStatus;

/// 构建或更新 input label 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn input_label<'a, Message: 'a>(label: &'a str) -> iced::Element<'a, Message> {
    text(label).size(12).style(settings_muted_text_style).into()
}

/// 构建或更新 input style 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    settings_text_input_style(theme, status)
}

/// 构建或更新 editor style 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn editor_style(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    settings_text_editor_style(theme, status)
}

pub fn selector_trigger_style(
    theme: &Theme,
    status: button::Status,
    open: bool,
) -> button::Style {
    crate::selector_styles::selector_trigger_style(theme, status, open)
}

pub fn selector_option_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    crate::selector_styles::selector_option_style(theme, status, selected)
}

pub fn popover_style(theme: &Theme) -> container::Style {
    crate::selector_styles::popover_style(theme)
}

/// 构建或更新 tooltip dark style 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn tooltip_dark_style(_theme: &Theme) -> container::Style {
    crate::selector_styles::tooltip_dark_style(_theme)
}

/// 构建或更新 square icon button style 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn square_icon_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    enabled: bool,
) -> iced::widget::button::Style {
    crate::selector_styles::square_icon_button_style(theme, status, enabled)
}

/// 构建或更新 subtask card style 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn subtask_card_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(
            if theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5
            {
                p.background.weak.color.scale_alpha(0.18)
            } else {
                Color::WHITE.scale_alpha(0.82)
            },
        )),
        border: Border {
            width: 1.0,
            color: p.background.strong.color.scale_alpha(0.72),
            radius: 16.0.into(),
        },
        ..Default::default()
    }
}

/// 构建或更新 subtask badge style 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn subtask_badge_style(
    theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let p = theme.extended_palette();
    iced::widget::button::Style {
        background: Some(Background::Color(p.background.weak.color.scale_alpha(0.55))),
        border: Border {
            width: 1.0,
            color: p.background.strong.color.scale_alpha(0.78),
            radius: 999.0.into(),
        },
        text_color: theme.palette().text,
        ..Default::default()
    }
}

/// 构建子任务状态图标文本。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn subtask_status_icon(status: SubTaskStatus, now_ms: u64) -> &'static str {
    match status {
        SubTaskStatus::Pending => ".",
        SubTaskStatus::Running => match (now_ms / 250) % 4 {
            0 => "◐",
            1 => "◓",
            2 => "◑",
            _ => "◒",
        },
        SubTaskStatus::Completed => "✓",
        SubTaskStatus::Failed => "x",
    }
}

/// 构建子任务状态标签。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn subtask_status_label(status: SubTaskStatus) -> &'static str {
    match status {
        SubTaskStatus::Pending => "待执行",
        SubTaskStatus::Running => "执行中",
        SubTaskStatus::Completed => "执行成功",
        SubTaskStatus::Failed => "执行失败",
    }
}

fn subtask_status_colors(theme: &Theme, status: SubTaskStatus) -> (Color, Color, Color) {
    let p = theme.extended_palette();
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    match status {
        SubTaskStatus::Pending => {
            let bg = if is_dark {
                p.background.weak.color.scale_alpha(0.18)
            } else {
                Color::from_rgba8(243, 244, 246, 0.92)
            };
            let border = if is_dark {
                p.background.strong.color.scale_alpha(0.72)
            } else {
                Color::from_rgb8(209, 213, 219)
            };
            let text = if is_dark { p.background.base.text } else { Color::from_rgb8(75, 85, 99) };
            (bg, border, text)
        }
        SubTaskStatus::Running => {
            let bg = if is_dark {
                Color::from_rgba8(168, 85, 247, 0.22)
            } else {
                Color::from_rgb8(243, 232, 255)
            };
            let border = Color::from_rgb8(192, 132, 252);
            let text = if is_dark {
                Color::from_rgb8(216, 180, 254)
            } else {
                Color::from_rgb8(126, 34, 206)
            };
            (bg, border, text)
        }
        SubTaskStatus::Completed => {
            let bg = if is_dark {
                Color::from_rgba8(34, 197, 94, 0.18)
            } else {
                Color::from_rgb8(220, 252, 231)
            };
            let border = Color::from_rgb8(74, 222, 128);
            let text = if is_dark {
                Color::from_rgb8(187, 247, 208)
            } else {
                Color::from_rgb8(22, 101, 52)
            };
            (bg, border, text)
        }
        SubTaskStatus::Failed => {
            let bg = if is_dark {
                Color::from_rgba8(239, 68, 68, 0.18)
            } else {
                Color::from_rgb8(254, 226, 226)
            };
            let border = Color::from_rgb8(248, 113, 113);
            let text = if is_dark {
                Color::from_rgb8(254, 202, 202)
            } else {
                Color::from_rgb8(185, 28, 28)
            };
            (bg, border, text)
        }
    }
}

/// 构建子任务状态文字样式。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn subtask_status_text_style(theme: &Theme, status: SubTaskStatus) -> text::Style {
    let (_, _, text_color) = subtask_status_colors(theme, status);
    text::Style { color: Some(text_color) }
}

/// 构建子任务状态徽章样式。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn subtask_status_badge_style(
    theme: &Theme,
    status: button::Status,
    subtask_status: SubTaskStatus,
) -> button::Style {
    let (mut background, border, text_color) = subtask_status_colors(theme, subtask_status);
    if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        background = background.scale_alpha(0.86);
    }
    button::Style {
        background: Some(Background::Color(background)),
        border: Border { width: 1.0, color: border, radius: 999.0.into() },
        text_color,
        ..Default::default()
    }
}

/// 构建子任务状态标签容器样式。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn subtask_status_pill_style(theme: &Theme, status: SubTaskStatus) -> container::Style {
    let (background, border, text_color) = subtask_status_colors(theme, status);
    container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(text_color),
        border: Border { width: 1.0, color: border, radius: 999.0.into() },
        ..Default::default()
    }
}

/// 构建或更新 disabled arrow button style 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn disabled_arrow_button_style(theme: &Theme) -> iced::widget::button::Style {
    let p = theme.extended_palette();
    iced::widget::button::Style {
        background: None,
        border: Border { radius: 4.0.into(), width: 0.0, color: Color::TRANSPARENT },
        text_color: p.background.weak.text,
        ..Default::default()
    }
}

/// 构建或更新 panel container style 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn panel_container_style(theme: &Theme) -> container::Style {
    let mut style = settings_panel_style(theme);
    style.border.radius = Border { radius: 22.0.into(), ..style.border }.radius;
    style
}

#[cfg(test)]
#[path = "styles_tests.rs"]
mod styles_tests;
