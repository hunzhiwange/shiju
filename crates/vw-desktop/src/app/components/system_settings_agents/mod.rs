//! 系统设置中智能体配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

mod basic;
mod identity;
mod shared;
mod skills;
mod tools;

#[cfg(test)]
#[path = "basic_tests.rs"]
mod basic_tests;
#[cfg(test)]
#[path = "identity_tests.rs"]
mod identity_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "shared_tests.rs"]
mod shared_tests;
#[cfg(test)]
#[path = "skills_tests.rs"]
mod skills_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "tools_tests.rs"]
mod tools_tests;

use self::shared::{
    agent_sidebar_button_style, detail_tab_button, entry_kind_label, section_card, selected_entry,
};
use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_muted_text_style, settings_page_intro, settings_panel_style,
    settings_text_input_style, settings_value_badge,
};
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::state::{
    AGENT_DETAIL_BASIC_TAB, AGENT_DETAIL_IDENTITY_TAB, AGENT_DETAIL_SKILLS_TAB,
    AGENT_DETAIL_TOOLS_TAB,
};
use crate::app::{App, Message};
use iced::widget::{
    button, column, container, row, scrollable,
    scrollable::{Direction, Scrollbar},
    text, text_input,
};
use iced::{Alignment, Element, Length, Theme};

/// 构建或处理 `view` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn view(app: &App) -> Element<'_, Message> {
    let settings = &app.agents_settings;
    let provider_options =
        settings.providers.iter().map(|provider| provider.id.clone()).collect::<Vec<_>>();
    let sidebar_items = settings
        .entries
        .iter()
        .map(|entry| {
            let is_selected = entry.key == settings.selected_agent;
            let meta = if entry.key == "main" {
                entry_kind_label(entry).to_string()
            } else if entry.enabled {
                format!("{} | 已启用", entry_kind_label(entry))
            } else {
                format!("{} | 已停用", entry_kind_label(entry))
            };
            let model_text = if entry.model.trim().is_empty() {
                "未选择 model".to_string()
            } else {
                entry.model.clone()
            };
            let button = iced::widget::button(
                container(
                    column![
                        text(&entry.label).size(14).style(move |theme: &Theme| {
                            iced::widget::text::Style {
                                color: Some(theme.palette().text.scale_alpha(if is_selected {
                                    0.98
                                } else {
                                    0.92
                                })),
                            }
                        }),
                        text(meta).size(12).style(move |theme: &Theme| {
                            iced::widget::text::Style {
                                color: Some(if is_selected {
                                    theme.palette().text.scale_alpha(0.72)
                                } else {
                                    theme.palette().text.scale_alpha(0.62)
                                }),
                            }
                        }),
                        text(model_text).size(12).style(move |theme: &Theme| {
                            iced::widget::text::Style {
                                color: Some(if is_selected {
                                    theme.palette().text.scale_alpha(0.68)
                                } else {
                                    theme.palette().text.scale_alpha(0.56)
                                }),
                            }
                        }),
                    ]
                    .spacing(4)
                    .width(Length::Fill),
                )
                .padding([10, 12])
                .width(Length::Fill),
            )
            .width(Length::Fill)
            .on_press(Message::Settings(SettingsMessage::Agents(AgentsMessage::SelectAgent(
                entry.key.clone(),
            ))))
            .style(move |theme: &Theme, status| {
                agent_sidebar_button_style(theme, status, is_selected)
            });
            container(button).width(Length::Fill).into()
        })
        .collect::<Vec<Element<'_, Message>>>();

    let title = settings_page_intro(
        "委托代理配置",
        "集中管理主 Agent、内建 Worker 与自定义 Agent 的模型、身份文件与工具白名单。",
    );
    let add_agent_bar = row![
        text_input("新增智能体 key", &settings.new_agent_key_input)
            .on_input(|value| {
                Message::Settings(SettingsMessage::Agents(AgentsMessage::AddAgentKeyChanged(value)))
            })
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
        button(text("新增").size(12))
            .padding([8, 14])
            .on_press(Message::Settings(SettingsMessage::Agents(AgentsMessage::AddAgentRequested,)))
            .style(rounded_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let detail: Element<'_, Message> = if let Some(entry) = selected_entry(app) {
        let detail_tabs = row![
            detail_tab_button("基础", AGENT_DETAIL_BASIC_TAB, &settings.active_detail_tab),
            detail_tab_button("身份", AGENT_DETAIL_IDENTITY_TAB, &settings.active_detail_tab),
            detail_tab_button("工具", AGENT_DETAIL_TOOLS_TAB, &settings.active_detail_tab),
            detail_tab_button("技能", AGENT_DETAIL_SKILLS_TAB, &settings.active_detail_tab),
        ]
        .spacing(10)
        .wrap();

        let detail_content: Element<'_, Message> = match settings.active_detail_tab.as_str() {
            AGENT_DETAIL_IDENTITY_TAB => identity::view(app, entry),
            AGENT_DETAIL_TOOLS_TAB => tools::view(app, entry),
            AGENT_DETAIL_SKILLS_TAB => skills::view(app, entry),
            _ => basic::view(app, entry, provider_options.clone()),
        };

        column![detail_tabs, detail_content].spacing(14).into()
    } else {
        container(text("未找到代理配置")).into()
    };

    let mut content = column![
        title,
        section_card(
            "Agent 工作台",
            "左侧选择 Agent，右侧集中编辑基础参数、身份文件、工具与技能能力。",
        ),
        row![
            container(
                column![
                    add_agent_bar,
                    row![
                        settings_value_badge(format!("{} 个代理", settings.entries.len())),
                        settings_value_badge(if settings.loading {
                            "同步中"
                        } else {
                            "已就绪"
                        }),
                    ]
                    .spacing(10)
                    .wrap(),
                    scrollable(
                        container(column(sidebar_items).spacing(8))
                            .padding(iced::Padding::default().right(10.0))
                    )
                    .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                    .height(Length::Fill),
                ]
                .spacing(12)
                .height(Length::Fill)
            )
            .padding([18, 18])
            .width(Length::Fixed(228.0))
            .height(Length::Fill)
            .style(settings_panel_style),
            container(
                scrollable(container(detail).padding(iced::Padding::default().right(12.0)))
                    .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                    .height(Length::Fill)
            )
            .padding([18, 20])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(settings_panel_style),
        ]
        .spacing(20)
        .height(Length::Fill),
    ]
    .spacing(14)
    .width(Length::Fill)
    .height(Length::Fill);

    if settings.loading {
        content = content.push(
            text("正在刷新提供商 / 模型 / 工具列表 ...").size(12).style(settings_muted_text_style),
        );
    }

    content.into()
}
