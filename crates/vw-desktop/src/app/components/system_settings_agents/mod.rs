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
    rounded_action_btn_style, settings_error_banner, settings_muted_text_style,
    settings_page_intro, settings_panel_style, settings_pick_list_menu_style,
    settings_pick_list_style, settings_segment_button_style, settings_text_input_style,
    settings_value_badge,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::message::settings::{AgentConfigTabMessage, AgentsMessage, SettingsMessage};
use crate::app::state::{
    AGENT_DETAIL_BASIC_TAB, AGENT_DETAIL_IDENTITY_TAB, AGENT_DETAIL_SKILLS_TAB,
    AGENT_DETAIL_TOOLS_TAB, AgentConfigTab,
};
use crate::app::{App, Message};
use iced::widget::{
    button, column, container, pick_list, row, scrollable,
    scrollable::{Direction, Scrollbar},
    text, text_input,
};
use iced::{Alignment, Element, Length, Theme};

pub(super) const AGENT_CONFIG_TABS: &[SettingsTabDescriptor<AgentConfigTab>] = &[
    SettingsTabDescriptor::new(
        AgentConfigTab::Agents,
        "智能体",
        &[
            "Agent 工作台",
            "新增智能体",
            "模型",
            "工具",
            "身份",
            "提供商",
            "API 密钥",
            "最大深度",
            "上下文压缩",
            "并行工具",
        ],
    ),
    SettingsTabDescriptor::new(
        AgentConfigTab::Channels,
        "通道",
        &["channels", "message", "CLI", "外部消息", "Telegram", "Slack", "飞书"],
    ),
    SettingsTabDescriptor::new(
        AgentConfigTab::Runtime,
        "运行时",
        &["runtime", "native", "docker", "wasm", "隔离", "工作区挂载"],
    ),
    SettingsTabDescriptor::new(
        AgentConfigTab::Acp,
        "ACP",
        &[
            "agent client protocol",
            "后端",
            "初始化",
            "codex",
            "claude",
            "gemini",
            "copilot",
            "pi",
            "npx",
        ],
    ),
    SettingsTabDescriptor::new(
        AgentConfigTab::Mcp,
        "MCP",
        &["model context protocol", "mcpServers", "server"],
    ),
    SettingsTabDescriptor::new(AgentConfigTab::Skills, "技能", &["skills", "catalog"]),
    SettingsTabDescriptor::new(
        AgentConfigTab::Plugins,
        "插件",
        &["plugin", "plugins", "插件目录", "启用"],
    ),
    SettingsTabDescriptor::new(
        AgentConfigTab::Reliability,
        "可靠性",
        &["reliability", "retry", "Provider 重试次数", "退避", "频道重连", "调度器重试"],
    ),
    SettingsTabDescriptor::new(
        AgentConfigTab::DialogueFlow,
        "对话流",
        &["dialogue flow", "permission", "时间线", "推理摘要", "reasoning summary", "edit patch"],
    ),
    SettingsTabDescriptor::new(
        AgentConfigTab::QuickMessages,
        "快捷消息",
        &["quick messages", "chat", "输入框", "Git 操作", "创建新会话", "自定义消息"],
    ),
    SettingsTabDescriptor::new(
        AgentConfigTab::Heartbeat,
        "心跳配置",
        &["heartbeat", "monitor", "follow up", "执行间隔", "兜底消息", "目标通道", "接收者"],
    ),
];

pub(super) fn agent_project_options(app: &App) -> Vec<String> {
    let mut options = Vec::new();
    for path in app.project_path.iter().chain(app.recent_projects.iter()) {
        let path = path.trim();
        if !path.is_empty() && !options.iter().any(|option: &String| option.as_str() == path) {
            options.push(path.to_string());
        }
    }
    options
}

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
    let active_tab = app.agents_settings.active_config_tab;
    let tab_button = |tab, label| {
        let is_active = tab == active_tab;
        button(text(label).size(13))
            .on_press(Message::Settings(SettingsMessage::AgentConfigTab(
                AgentConfigTabMessage::Selected(tab),
            )))
            .padding([8, 14])
            .style(move |theme: &Theme, status| {
                settings_segment_button_style(theme, status, is_active)
            })
    };
    let tabs = AGENT_CONFIG_TABS
        .iter()
        .fold(row![].spacing(8).align_y(Alignment::Center), |row, descriptor| {
            row.push(tab_button(descriptor.tab, descriptor.label))
        })
        .wrap();

    let content = match active_tab {
        AgentConfigTab::Agents => agents_view(app),
        AgentConfigTab::Channels => {
            scrollable(container(crate::app::components::system_settings_channels::view(app)))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill)
                .into()
        }
        AgentConfigTab::Runtime => {
            scrollable(container(crate::app::components::system_settings_runtime::view(app)))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill)
                .into()
        }
        AgentConfigTab::Acp => scrollable(
            container(crate::app::components::system_settings_acp::view(app))
                .padding(iced::Padding::default().right(12.0)),
        )
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill)
        .into(),
        AgentConfigTab::Mcp => {
            scrollable(container(crate::app::components::system_settings_mcp::view(app)))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill)
                .into()
        }
        AgentConfigTab::Skills => {
            scrollable(container(crate::app::components::system_settings_skills::view(app)))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill)
                .into()
        }
        AgentConfigTab::Plugins => scrollable(container(
            crate::app::components::system_settings_skills::plugins::view(app),
        ))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill)
        .into(),
        AgentConfigTab::Reliability => {
            scrollable(container(crate::app::components::system_settings_reliability::view(app)))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill)
                .into()
        }
        AgentConfigTab::DialogueFlow => {
            scrollable(container(crate::app::components::system_settings_dialogue_flow::view(app)))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill)
                .into()
        }
        AgentConfigTab::QuickMessages => {
            scrollable(
                container(crate::app::components::system_settings_quick_messages::view(app))
                    .padding(iced::Padding { left: 8.0, right: 16.0, ..Default::default() }),
            )
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill)
                .into()
        }
        AgentConfigTab::Heartbeat => {
            scrollable(container(crate::app::components::system_settings_heartbeat::view(app)))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill)
                .into()
        }
    };

    let mut page = column![tabs].spacing(16).width(Length::Fill).height(Length::Fill);
    if let Some(error) = active_tab_save_error(app) {
        page = page.push(settings_error_banner(error));
    }
    page.push(content).into()
}

fn active_tab_save_error(app: &App) -> Option<&str> {
    match app.agents_settings.active_config_tab {
        AgentConfigTab::Agents => app.agents_settings.save_error.as_deref(),
        AgentConfigTab::Runtime => app.runtime_settings.save_error.as_deref(),
        AgentConfigTab::Acp => app.acp_settings.save_error.as_deref(),
        AgentConfigTab::Reliability => app.reliability_settings.save_error.as_deref(),
        AgentConfigTab::Heartbeat => app.heartbeat_settings.save_error.as_deref(),
        AgentConfigTab::Channels
        | AgentConfigTab::Mcp
        | AgentConfigTab::Skills
        | AgentConfigTab::Plugins
        | AgentConfigTab::DialogueFlow
        | AgentConfigTab::QuickMessages => None,
    }
}

fn agents_view(app: &App) -> Element<'_, Message> {
    let settings = &app.agents_settings;
    if !settings.config_ready {
        return column![
            settings_page_intro(
                "智能体配置",
                "按项目分别管理内建与自定义智能体的模型、身份文件、工具、技能和运行参数。",
            ),
            section_card(
                "配置尚未就绪",
                "为防止启动默认值覆盖已保存配置，加载成功前此页面保持只读。",
            ),
            text("正在等待 Gateway 返回当前作用域的智能体配置；完成前不会使用默认值保存。")
                .size(12)
                .style(settings_muted_text_style),
            button(text("重新加载").size(12))
                .padding([8, 14])
                .on_press(Message::Settings(SettingsMessage::Agents(AgentsMessage::Refresh)))
                .style(rounded_action_btn_style),
        ]
        .spacing(14)
        .width(Length::Fill)
        .into();
    }

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
                        text(&entry.key).size(12).style(move |theme: &Theme| {
                            iced::widget::text::Style {
                                color: Some(if is_selected {
                                    theme.palette().text.scale_alpha(0.72)
                                } else {
                                    theme.palette().text.scale_alpha(0.62)
                                }),
                            }
                        }),
                        text(&entry.description).size(12).style(move |theme: &Theme| {
                            iced::widget::text::Style {
                                color: Some(if is_selected {
                                    theme.palette().text.scale_alpha(0.72)
                                } else {
                                    theme.palette().text.scale_alpha(0.62)
                                }),
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
        "智能体配置",
        "按项目分别管理内建与自定义智能体的模型、身份文件、工具、技能和运行参数。",
    );
    let project_options = agent_project_options(app);
    let selected_project = settings.project_path.clone().or_else(|| app.project_path.clone());
    let project_switcher: Element<'_, Message> = if project_options.is_empty() {
        text("尚未打开项目；当前编辑全局智能体配置。")
            .size(12)
            .style(settings_muted_text_style)
            .into()
    } else {
        row![
            text("项目目录").size(13).width(Length::Fixed(84.0)),
            pick_list(project_options, selected_project, |path| {
                Message::Settings(SettingsMessage::Agents(AgentsMessage::ProjectSelected(path)))
            })
            .placeholder("选择项目目录")
            .padding([9, 12])
            .text_size(12)
            .width(Length::Fill)
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style),
            settings_value_badge(if settings.loading { "切换中" } else { "项目独立保存" }),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    };
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
            "项目配置作用域",
            "切换项目会恢复该项目的智能体配置；修改保存到项目配置数据库。",
        ),
        project_switcher,
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
