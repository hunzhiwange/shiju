//! 系统设置中智能体配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    settings_muted_text_style, settings_section_card as shared_settings_section_card,
    settings_segment_button_style, settings_text_editor_style,
};
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::state::{AgentSettingsEntryKind, DelegateAgentSettingsEntry};
use crate::app::{App, Message};
use iced::widget::{button, column, text, text_editor};
use iced::{Background, Border, Color, Element, Theme};

/// 构建或处理 `section_card` 对应的界面片段与交互数据。
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
pub(super) fn section_card<'a>(
    title: &'a str,
    description: &'a str,
) -> iced::widget::Container<'a, Message> {
    shared_settings_section_card(title, description)
}

/// 构建或处理 `label_col` 对应的界面片段与交互数据。
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
pub(super) fn label_col<'a>(title: &'a str, hint: &'a str) -> iced::widget::Column<'a, Message> {
    column![text(title), text(hint).size(12).style(settings_muted_text_style),].spacing(4)
}

/// 构建或处理 `editor_style` 对应的界面片段与交互数据。
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
pub(super) fn editor_style(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    settings_text_editor_style(theme, status)
}

/// 构建或处理 `is_dark_theme` 对应的界面片段与交互数据。
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
pub(super) fn is_dark_theme(theme: &Theme) -> bool {
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
pub(super) fn agent_sidebar_button_style(
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

/// 构建或处理 `selected_entry` 对应的界面片段与交互数据。
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
pub(super) fn selected_entry(app: &App) -> Option<&DelegateAgentSettingsEntry> {
    app.agents_settings
        .entries
        .iter()
        .find(|entry| entry.key == app.agents_settings.selected_agent)
        .or_else(|| app.agents_settings.entries.first())
}

/// 构建或处理 `entry_kind_label` 对应的界面片段与交互数据。
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
pub(super) fn entry_kind_label(entry: &DelegateAgentSettingsEntry) -> &'static str {
    match entry.kind {
        AgentSettingsEntryKind::Main => "内建 Agent",
        AgentSettingsEntryKind::BuiltinWorker => "内建 Worker",
        AgentSettingsEntryKind::Custom => "自定义",
    }
}

/// 构建或处理 `models_for_provider` 对应的界面片段与交互数据。
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
pub(super) fn models_for_provider(app: &App, provider_id: &str) -> Vec<String> {
    app.agents_settings
        .provider_models
        .iter()
        .find(|provider| provider.id == provider_id)
        .map(|provider| provider.models.iter().map(|model| model.id.clone()).collect())
        .unwrap_or_default()
}

/// 构建或处理 `with_selected_option` 对应的界面片段与交互数据。
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
pub(super) fn with_selected_option(mut options: Vec<String>, selected: &str) -> Vec<String> {
    let selected = selected.trim();
    if !selected.is_empty() && !options.iter().any(|option| option == selected) {
        options.push(selected.to_string());
    }
    options.sort();
    options.dedup();
    options
}

/// 构建或处理 `detail_tab_button` 对应的界面片段与交互数据。
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
pub(super) fn detail_tab_button<'a>(
    label: &'a str,
    key: &'a str,
    active_key: &'a str,
) -> Element<'a, Message> {
    let is_active = key == active_key;
    button(text(label).size(14))
        .padding([10, 18])
        .style(move |theme: &Theme, status| settings_segment_button_style(theme, status, is_active))
        .on_press(Message::Settings(SettingsMessage::Agents(AgentsMessage::DetailTabSelected(
            key.to_string(),
        ))))
        .into()
}
