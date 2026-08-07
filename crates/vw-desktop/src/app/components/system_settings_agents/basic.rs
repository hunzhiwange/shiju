//! 系统设置中智能体配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::shared::{label_col, models_for_provider, section_card, with_selected_option};
use crate::app::components::number_input::NumberInput;
use crate::app::components::system_settings_common::{
    settings_checkbox_style, settings_pick_list_menu_style, settings_pick_list_style,
    settings_text_input_style,
};
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::state::DelegateAgentSettingsEntry;
use crate::app::{App, Message};
use iced::widget::{checkbox, column, pick_list, row, slider, text, text_input};
use iced::{Alignment, Element, Length, Theme};

pub(super) fn is_main_agent(entry: &DelegateAgentSettingsEntry) -> bool {
    entry.key == "main"
}

pub(super) fn shows_registration_controls(entry: &DelegateAgentSettingsEntry) -> bool {
    !is_main_agent(entry)
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
    provider_options: Vec<String>,
) -> Element<'a, Message> {
    let provider_choices = with_selected_option(provider_options, &entry.provider);
    let model_choices =
        with_selected_option(models_for_provider(app, &entry.provider), &entry.model);

    let provider_row = row![
        label_col("提供商", "选择该代理调用的模型提供商。"),
        pick_list(
            provider_choices,
            (!entry.provider.trim().is_empty()).then_some(entry.provider.clone()),
            {
                let key = entry.key.clone();
                move |value| {
                    Message::Settings(SettingsMessage::Agents(AgentsMessage::ProviderChanged(
                        key.clone(),
                        value,
                    )))
                }
            }
        )
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fill),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let model_row = row![
        label_col("模型", "从当前提供商的可用模型中选择。"),
        pick_list(
            model_choices,
            (!entry.model.trim().is_empty()).then_some(entry.model.clone()),
            {
                let key = entry.key.clone();
                move |value| {
                    Message::Settings(SettingsMessage::Agents(AgentsMessage::ModelChanged(
                        key.clone(),
                        value,
                    )))
                }
            }
        )
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fill),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let api_key_row = row![
        label_col("API 密钥", "可选覆盖该代理的提供商凭证。"),
        text_input("可选", &entry.api_key_input)
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
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let temperature_slider = slider(0.0..=2.0, entry.temperature, {
        let key = entry.key.clone();
        move |value| {
            Message::Settings(SettingsMessage::Agents(AgentsMessage::TemperatureChanged(
                key.clone(),
                (value * 100.0).round() / 100.0,
            )))
        }
    });

    let temperature_row = row![
        label_col("温度", "控制生成发散度，范围 0.0 - 2.0。"),
        column![
            temperature_slider,
            text(format!("{:.2}", entry.temperature)).size(12).style(|theme: &Theme| {
                iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.65)) }
            }),
        ]
        .spacing(8)
        .width(Length::Fill),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let is_main = is_main_agent(entry);

    let enabled_row = row![
        label_col("启用", "关闭后保留配置，但该智能体不会作为可用项参与注册。"),
        checkbox(entry.enabled)
            .label("启用该智能体")
            .on_toggle({
                let key = entry.key.clone();
                move |value| {
                    Message::Settings(SettingsMessage::Agents(AgentsMessage::EnabledToggled(
                        key.clone(),
                        value,
                    )))
                }
            })
            .style(settings_checkbox_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let registration_rows = {
        let mut rows = Vec::<Element<'_, Message>>::new();
        if shows_registration_controls(entry) {
            rows.push(enabled_row.into());
        }
        rows
    };
    let registration_section: Element<'_, Message> = if registration_rows.is_empty() {
        text("").into()
    } else {
        column![
            section_card("可用性", "控制该智能体是否作为可用 Agent 暴露。"),
            column(registration_rows).spacing(14),
        ]
        .spacing(14)
        .into()
    };

    let allow_as_subagent_row = row![
        label_col("作为子智能体", "只有开启后，该 Agent 才会出现在 AgentTool 的可选列表中。"),
        checkbox(entry.allow_as_subagent)
            .label("允许作为子智能体")
            .on_toggle({
                let key = entry.key.clone();
                move |value| {
                    Message::Settings(SettingsMessage::Agents(
                        AgentsMessage::AllowAsSubagentToggled(key.clone(), value),
                    ))
                }
            })
            .style(settings_checkbox_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let max_parallel_subagents_row = row![
        label_col("子智能体数量", "该 Agent 每轮最多同时启动的子智能体数量。"),
        NumberInput::new(entry.max_parallel_subagents as f32, 1.0, 32.0, 1.0, 0, 1.0, {
            let key = entry.key.clone();
            move |value| {
                Message::Settings(SettingsMessage::Agents(
                    AgentsMessage::MaxParallelSubagentsChanged(key.clone(), value.round() as u32),
                ))
            }
        },)
        .settings_style(),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let delegation_section = column![
        section_card("子智能体委托", "控制该 Agent 是否可被委托，以及它发起委托时的并发上限。"),
        column![allow_as_subagent_row, max_parallel_subagents_row].spacing(14),
    ]
    .spacing(14);

    let max_depth_row = row![
        label_col("最大深度", "委托嵌套深度上限，防止无限递归。"),
        NumberInput::new(entry.max_depth as f32, 1.0, 32.0, 1.0, 0, 1.0, {
            let key = entry.key.clone();
            move |value| {
                Message::Settings(SettingsMessage::Agents(AgentsMessage::MaxDepthChanged(
                    key.clone(),
                    value.round() as u32,
                )))
            }
        })
        .settings_style(),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let compact_context_row = row![
        label_col("上下文压缩", "适合小模型，压缩主 Agent 的上下文体积。"),
        checkbox(entry.compact_context)
            .label("启用上下文压缩")
            .on_toggle({
                let key = entry.key.clone();
                move |value| {
                    Message::Settings(SettingsMessage::Agents(
                        AgentsMessage::CompactContextToggled(key.clone(), value),
                    ))
                }
            })
            .style(settings_checkbox_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let max_tool_iterations_row = row![
        label_col("最大迭代次数", "主 Agent 单次请求中允许的最大工具循环轮次。"),
        NumberInput::new(entry.max_tool_iterations as f32, 1.0, u32::MAX as f32, 1.0, 0, 1.0, {
            let key = entry.key.clone();
            move |value| {
                Message::Settings(SettingsMessage::Agents(AgentsMessage::MaxToolIterationsChanged(
                    key.clone(),
                    value.round() as u32,
                )))
            }
        })
        .settings_style(),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let max_history_messages_row = row![
        label_col("历史消息上限", "主 Agent 保留给模型的会话历史消息上限。"),
        NumberInput::new(entry.max_history_messages as f32, 1.0, 1000.0, 1.0, 0, 1.0, {
            let key = entry.key.clone();
            move |value| {
                Message::Settings(SettingsMessage::Agents(
                    AgentsMessage::MaxHistoryMessagesChanged(key.clone(), value.round() as u32),
                ))
            }
        })
        .settings_style(),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let parallel_tools_row = row![
        label_col("并行工具", "主 Agent 是否允许同一轮并行执行多个工具。"),
        checkbox(entry.parallel_tools)
            .label("启用并行工具执行")
            .on_toggle({
                let key = entry.key.clone();
                move |value| {
                    Message::Settings(SettingsMessage::Agents(AgentsMessage::ParallelToolsToggled(
                        key.clone(),
                        value,
                    )))
                }
            })
            .style(settings_checkbox_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let tool_dispatcher_row = row![
        label_col("工具调度器", "主 Agent 的工具分发策略，默认 auto。"),
        text_input("auto", &entry.tool_dispatcher)
            .on_input({
                let key = entry.key.clone();
                move |value| {
                    Message::Settings(SettingsMessage::Agents(
                        AgentsMessage::ToolDispatcherChanged(key.clone(), value),
                    ))
                }
            })
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let max_iterations_row = row![
        label_col("最大迭代次数", "智能体模式下最多运行多少轮工具循环。"),
        NumberInput::new(entry.max_iterations as f32, 1.0, u32::MAX as f32, 1.0, 0, 1.0, {
            let key = entry.key.clone();
            move |value| {
                Message::Settings(SettingsMessage::Agents(AgentsMessage::MaxIterationsChanged(
                    key.clone(),
                    value.round() as u32,
                )))
            }
        })
        .settings_style(),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let agentic_row = row![
        label_col("智能体模式", "启用后允许该代理进入多轮工具调用循环。"),
        checkbox(entry.agentic)
            .label("启用智能体模式")
            .on_toggle({
                let key = entry.key.clone();
                move |value| {
                    Message::Settings(SettingsMessage::Agents(AgentsMessage::AgenticToggled(
                        key.clone(),
                        value,
                    )))
                }
            })
            .style(settings_checkbox_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let main_runtime_section: Element<'_, Message> = if is_main {
        column![
            section_card(
                "主 Agent 运行时",
                "这些参数原本在单独的 Agent 页面中，现在统一并入 Main Agent。",
            ),
            compact_context_row,
            max_tool_iterations_row,
            max_history_messages_row,
            parallel_tools_row,
            tool_dispatcher_row,
        ]
        .spacing(14)
        .into()
    } else {
        column![].into()
    };

    column![
        section_card(&entry.label, "为当前 Agent 单独设置 provider、model 与基础运行参数。",),
        registration_section,
        delegation_section,
        provider_row,
        model_row,
        api_key_row,
        temperature_row,
        max_depth_row,
        max_iterations_row,
        agentic_row,
        main_runtime_section,
    ]
    .spacing(14)
    .into()
}
