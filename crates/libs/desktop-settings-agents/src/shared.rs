pub use desktop_ui_controls::system_settings_common::{
    settings_text_editor_style as editor_style,
};
use iced::widget::Button;
use crate::message::Event;
pub fn agent_tab_button(label: &str, selected: bool, message: Event) -> Button<'_, Event> {
    desktop_ui_controls::system_settings_common::settings_tab_button(label, selected, Some(message))
}

use iced::{Theme, Background, Color, Border};
use crate::state::{DelegateAgentSettingsEntry, AgentSettingsEntryKind};
pub fn is_dark_theme(theme: &Theme) -> bool {
    let background = theme.palette().background;
    background.r + background.g + background.b < 1.5
}

/// 构建或处理 `agent_sidebar_button_style` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn agent_sidebar_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    is_selected: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    let background = if is_selected {
        Some(Background::Color(if is_dark {
            theme.palette().primary.scale_alpha(0.14)
        } else {
            theme.palette().primary.scale_alpha(0.07)
        }))
    } else {
        match status {
            iced::widget::button::Status::Hovered => Some(Background::Color(if is_dark {
                palette.background.weak.color.scale_alpha(0.64)
            } else {
                Color::WHITE.scale_alpha(0.74)
            })),
            iced::widget::button::Status::Pressed => Some(Background::Color(if is_dark {
                palette.background.strong.color.scale_alpha(0.84)
            } else {
                palette.background.weak.color.scale_alpha(0.90)
            })),
            _ => Some(Background::Color(if is_dark {
                palette.background.base.color.scale_alpha(0.32)
            } else {
                Color::WHITE.scale_alpha(0.50)
            })),
        }
    };

    let border_color = if is_selected {
        theme.palette().primary.scale_alpha(if is_dark { 0.30 } else { 0.20 })
    } else if matches!(
        status,
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
    ) {
        if is_dark {
            palette.background.strong.color.scale_alpha(0.84)
        } else {
            Color::from_rgba8(15, 23, 42, 0.08)
        }
    } else if is_dark {
        palette.background.strong.color.scale_alpha(0.54)
    } else {
        Color::from_rgba8(15, 23, 42, 0.06)
    };

    iced::widget::button::Style {
        background,
        text_color: theme.palette().text,
        border: Border { width: 1.0, color: border_color, radius: 14.0.into() },
        ..Default::default()
    }
}

pub fn entry_kind_label(entry: &DelegateAgentSettingsEntry) -> &'static str {
    match entry.kind {
        AgentSettingsEntryKind::Main => "内建 Agent",
        AgentSettingsEntryKind::BuiltinWorker => "内建 Worker",
        AgentSettingsEntryKind::Custom if entry.builtin => "官方 Agent",
        AgentSettingsEntryKind::Custom => "自定义",
    }
}
