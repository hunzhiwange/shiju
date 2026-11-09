//! 系统设置中智能体配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::shared::section_card;
use crate::app::components::number_input::NumberInput;
use crate::app::components::system_settings_common::{
    settings_checkbox_style, settings_divider, settings_flexible_row_with_help,
    settings_group_card, settings_group_heading_with_help, settings_muted_text_style,
    settings_text_input,
};
use crate::app::i18n::{I18n, settings_extensions::text as tr};
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::state::DelegateAgentSettingsEntry;
use crate::app::{App, Message};
use iced::widget::{checkbox, column, row, slider, text};
use iced::{Alignment, Element, Length};

pub(super) fn shows_registration_controls(entry: &DelegateAgentSettingsEntry) -> bool {
    entry.can_disable
}

pub(super) fn shows_delegation_controls(entry: &DelegateAgentSettingsEntry) -> bool {
    !entry.role_controls_locked
}

/// 构建或处理 `view` 对应的界面片段与交互数据。
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
pub(super) fn view<'a>(
    app: &'a App,
    entry: &'a DelegateAgentSettingsEntry,
    _provider_options: Vec<String>,
) -> Element<'a, Message> {
    let native = match entry.runtime {
        vw_config_types::agent::AgentRuntime::Native => true,
        vw_config_types::agent::AgentRuntime::Acp { .. } => false,
    };
    let i18n = I18n::new(app.desktop_locale);
    let field_with_help = |label, description, control: Element<'a, Message>| {
        settings_flexible_row_with_help(i18n, &app.app_theme, label, description, 320.0, control)
            .padding([6, 0])
    };
    let api_key_row = field_with_help(
        tr(app.desktop_locale, "API 密钥"),
        tr(app.desktop_locale, "可选覆盖该代理的提供商凭证。"),
        settings_text_input(tr(app.desktop_locale, "可选"), &entry.api_key_input)
            .secure(true)
            .on_input({
                let key = entry.key.clone();
                move |value| {
                    Message::Settings(SettingsMessage::Agents(AgentsMessage::ApiKeyChanged(
                        key.clone(),
                        value,
                    )))
                }
            })
            .width(Length::Fill)
            .into(),
    );

    let temperature_slider = slider(0.0..=2.0, entry.temperature, {
        let key = entry.key.clone();
        move |value| {
            Message::Settings(SettingsMessage::Agents(AgentsMessage::TemperatureChanged(
                key.clone(),
                (value * 100.0).round() / 100.0,
            )))
        }
    });

    let temperature_row = field_with_help(
        tr(app.desktop_locale, "温度"),
        tr(app.desktop_locale, "控制生成发散度。"),
        column![
            temperature_slider,
            row![
                text(format!("{:.2}", entry.temperature)).size(12).style(settings_muted_text_style),
                text(tr(app.desktop_locale, "范围 0.0–2.0"))
                    .size(12)
                    .style(settings_muted_text_style),
            ]
            .spacing(8)
            .wrap(),
        ]
        .spacing(8)
        .width(Length::Fill)
        .into(),
    );

    let enabled_row = field_with_help(
        tr(app.desktop_locale, "启用"),
        tr(app.desktop_locale, "关闭后保留配置。"),
        column![
            checkbox(entry.enabled)
                .label(tr(app.desktop_locale, "启用该智能体"))
                .on_toggle({
                    let key = entry.key.clone();
                    move |value| {
                        Message::Settings(SettingsMessage::Agents(AgentsMessage::CatalogToggle(
                            key.clone(),
                            value,
                        )))
                    }
                })
                .style(settings_checkbox_style),
            text(tr(app.desktop_locale, "关闭后不参与注册。"))
                .size(12)
                .style(settings_muted_text_style),
        ]
        .spacing(6)
        .align_x(Alignment::End)
        .into(),
    );

    let registration_section = shows_registration_controls(entry).then(|| {
        column![
            settings_group_heading_with_help(
                i18n,
                &app.app_theme,
                tr(app.desktop_locale, "可用性"),
                tr(app.desktop_locale, "控制该智能体是否作为可用 Agent 暴露。")
            ),
            settings_group_card(enabled_row).padding([14, 16]),
        ]
        .spacing(8)
    });

    let allow_as_subagent_row = field_with_help(
        tr(app.desktop_locale, "作为子智能体"),
        tr(app.desktop_locale, "只有开启后，该 Agent 才会出现在 AgentTool 的可选列表中。"),
        column![
            checkbox(entry.allow_as_subagent)
                .label(tr(app.desktop_locale, "允许作为子智能体"))
                .on_toggle({
                    let key = entry.key.clone();
                    move |value| {
                        Message::Settings(SettingsMessage::Agents(
                            AgentsMessage::AllowAsSubagentToggled(key.clone(), value),
                        ))
                    }
                })
                .style(settings_checkbox_style),
            text(tr(app.desktop_locale, "关闭后不可被委托。"))
                .size(12)
                .style(settings_muted_text_style),
        ]
        .spacing(6)
        .align_x(Alignment::End)
        .into(),
    );

    let max_parallel_subagents_row = field_with_help(
        tr(app.desktop_locale, "子智能体数量"),
        tr(app.desktop_locale, "该 Agent 每轮最多同时启动的子智能体数量。"),
        NumberInput::new(entry.max_parallel_subagents as f32, 1.0, 32.0, 1.0, 0, 1.0, {
            let key = entry.key.clone();
            move |value| {
                Message::Settings(SettingsMessage::Agents(
                    AgentsMessage::MaxParallelSubagentsChanged(key.clone(), value.round() as u32),
                ))
            }
        })
        .settings_style()
        .into(),
    );

    let max_depth_row = field_with_help(
        tr(app.desktop_locale, "最大深度"),
        tr(app.desktop_locale, "委托嵌套深度上限，防止无限递归。"),
        NumberInput::new(entry.max_depth as f32, 1.0, 32.0, 1.0, 0, 1.0, {
            let key = entry.key.clone();
            move |value| {
                Message::Settings(SettingsMessage::Agents(AgentsMessage::MaxDepthChanged(
                    key.clone(),
                    value.round() as u32,
                )))
            }
        })
        .settings_style()
        .into(),
    );

    let compact_context_row = field_with_help(
        tr(app.desktop_locale, "上下文压缩"),
        tr(app.desktop_locale, "适合小模型，压缩该智能体的上下文体积。"),
        checkbox(entry.compact_context)
            .label(tr(app.desktop_locale, "启用上下文压缩"))
            .on_toggle({
                let key = entry.key.clone();
                move |value| {
                    Message::Settings(SettingsMessage::Agents(
                        AgentsMessage::CompactContextToggled(key.clone(), value),
                    ))
                }
            })
            .style(settings_checkbox_style)
            .into(),
    );

    let max_history_messages_row = field_with_help(
        tr(app.desktop_locale, "历史消息上限"),
        tr(app.desktop_locale, "该智能体保留给模型的会话历史消息上限。"),
        NumberInput::new(entry.max_history_messages as f32, 1.0, 1000.0, 1.0, 0, 1.0, {
            let key = entry.key.clone();
            move |value| {
                Message::Settings(SettingsMessage::Agents(
                    AgentsMessage::MaxHistoryMessagesChanged(key.clone(), value.round() as u32),
                ))
            }
        })
        .settings_style()
        .into(),
    );

    let parallel_tools_row = field_with_help(
        tr(app.desktop_locale, "并行工具"),
        tr(app.desktop_locale, "该智能体是否允许同一轮并行执行多个工具。"),
        checkbox(entry.parallel_tools)
            .label(tr(app.desktop_locale, "启用并行工具执行"))
            .on_toggle({
                let key = entry.key.clone();
                move |value| {
                    Message::Settings(SettingsMessage::Agents(AgentsMessage::ParallelToolsToggled(
                        key.clone(),
                        value,
                    )))
                }
            })
            .style(settings_checkbox_style)
            .into(),
    );

    let max_iterations_row = field_with_help(
        tr(app.desktop_locale, "最大迭代次数"),
        tr(app.desktop_locale, "智能体模式下最多运行多少轮工具循环。"),
        NumberInput::new(entry.max_iterations as f32, 1.0, u32::MAX as f32, 1.0, 0, 1.0, {
            let key = entry.key.clone();
            move |value| {
                Message::Settings(SettingsMessage::Agents(AgentsMessage::MaxIterationsChanged(
                    key.clone(),
                    value.round() as u32,
                )))
            }
        })
        .settings_style()
        .into(),
    );

    let agentic_row = field_with_help(
        tr(app.desktop_locale, "智能体模式"),
        tr(app.desktop_locale, "启用后允许该代理进入多轮工具调用循环。"),
        checkbox(entry.agentic)
            .label(tr(app.desktop_locale, "启用智能体模式"))
            .on_toggle({
                let key = entry.key.clone();
                move |value| {
                    Message::Settings(SettingsMessage::Agents(AgentsMessage::AgenticToggled(
                        key.clone(),
                        value,
                    )))
                }
            })
            .style(settings_checkbox_style)
            .into(),
    );

    let delegation_section: Element<'_, Message> = if shows_delegation_controls(entry) {
        column![
            settings_group_heading_with_help(
                i18n,
                &app.app_theme,
                tr(app.desktop_locale, "子智能体委托"),
                tr(
                    app.desktop_locale,
                    "控制该 Agent 是否可被平台委托；原生引擎还可配置发起委托的并发上限。"
                )
            ),
            settings_group_card(
                column![
                    allow_as_subagent_row,
                    native.then(settings_divider),
                    native.then_some(max_parallel_subagents_row),
                    native.then(settings_divider),
                    native.then_some(max_depth_row),
                ]
                .spacing(12),
            )
            .padding([14, 16]),
        ]
        .spacing(8)
        .into()
    } else {
        section_card(
            tr(app.desktop_locale, "角色边界"),
            tr(
                app.desktop_locale,
                "委托资格、并发上限和递归深度由内置角色目录固定，不能在此放宽。",
            ),
        )
        .into()
    };

    let runtime_section = native.then(|| {
        settings_group_card(
            column![
                api_key_row,
                settings_divider(),
                temperature_row,
                settings_divider(),
                max_iterations_row,
                settings_divider(),
                agentic_row,
                settings_divider(),
                compact_context_row,
                settings_divider(),
                max_history_messages_row,
                settings_divider(),
                parallel_tools_row,
            ]
            .spacing(12),
        )
        .padding([14, 16])
    });

    column![
        super::basic_profile::view(app, entry),
        super::defaults::view(app, entry),
        registration_section,
        delegation_section,
        runtime_section,
    ]
    .spacing(16)
    .into()
}

#[cfg(test)]
#[path = "basic_ui_refresh_16_tests.rs"]
mod ui_refresh_16_tests;
