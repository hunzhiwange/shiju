//! 系统设置中智能体配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    settings_section_card as shared_settings_section_card, settings_tab_button,
    settings_text_editor_style,
};
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::state::DelegateAgentSettingsEntry;
use crate::app::{App, Message};
use iced::widget::{Button, text_editor};
use iced::{Element, Theme};

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

#[cfg(test)]
pub(super) use desktop_settings_agents::shared::{
    is_dark_theme, agent_sidebar_button_style, entry_kind_label,
};

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
    app.agents_settings.profile_modal.as_ref().or_else(|| {
        app.agents_settings
            .entries
            .iter()
            .find(|entry| entry.key == app.agents_settings.selected_agent)
            .or_else(|| app.agents_settings.entries.first())
    })
}

pub(super) fn agent_tab_button(
    label: &str,
    selected: bool,
    message: Message,
) -> Button<'_, Message> {
    settings_tab_button(label, selected, Some(message))
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
    agent_tab_button(
        label,
        key == active_key,
        Message::Settings(SettingsMessage::Agents(AgentsMessage::DetailTabSelected(
            key.to_string(),
        ))),
    )
    .into()
}
