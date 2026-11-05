//! 系统设置中智能体配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

mod basic;
mod basic_profile;
mod catalog;
#[cfg(test)]
mod catalog_tests;
pub(crate) mod defaults;
mod identity;
mod mcp;
mod profile_modal;
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

use self::shared::section_card;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, rounded_action_btn_style,
    settings_error_banner, settings_muted_text_style, settings_page_intro,
    settings_segment_button_style, settings_text_input_style,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::message::settings::{AgentConfigTabMessage, AgentsMessage, SettingsMessage};
use crate::app::state::AgentConfigTab;
use crate::app::{App, Message};
use iced::widget::{
    button, column, container, row, scrollable,
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
            row.push(tab_button(descriptor.tab, tr(app.desktop_locale, descriptor.label)))
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
        AgentConfigTab::Acp => crate::app::components::system_settings_acp::view(app),
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
        AgentConfigTab::QuickMessages => scrollable(
            container(crate::app::components::system_settings_quick_messages::view(app))
                .padding(iced::Padding { left: 8.0, right: 16.0, ..Default::default() }),
        )
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill)
        .into(),
        AgentConfigTab::Heartbeat => {
            scrollable(container(crate::app::components::system_settings_heartbeat::view(app)))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill)
                .into()
        }
    };

    let mut page = column![tabs].spacing(16).width(Length::Fill).height(Length::Fill);
    if let Some(error) = active_tab_save_error(app) {
        let rendered = app
            .agents_settings
            .save_error_translation
            .as_ref()
            .filter(|message| {
                message
                    .render(crate::app::i18n::I18n::new(
                        vw_config_types::ui::DesktopLocale::SimplifiedChinese,
                    ))
                    .is_ok_and(|source| source == error)
            })
            .map(|message| {
                message
                    .render(crate::app::i18n::I18n::new(app.desktop_locale))
                    .expect("valid agent error template")
            })
            .unwrap_or_else(|| {
                crate::app::i18n::I18n::new(app.desktop_locale).__(error).to_owned()
            });
        page = page.push(settings_error_banner(rendered));
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
                tr(app.desktop_locale, "智能体配置"),
                tr(
                    app.desktop_locale,
                    "全局管理内建与自定义智能体的模型、身份文件、工具、技能和运行参数。"
                ),
            ),
            section_card(
                tr(app.desktop_locale, "配置尚未就绪"),
                tr(
                    app.desktop_locale,
                    "为防止启动默认值覆盖已保存配置，加载成功前此页面保持只读。"
                ),
            ),
            text(tr(
                app.desktop_locale,
                "正在等待 Gateway 返回当前作用域的智能体配置；完成前不会使用默认值保存。"
            ))
            .size(12)
            .style(settings_muted_text_style),
            button(text(tr(app.desktop_locale, "重新加载")).size(12))
                .padding([8, 14])
                .on_press(Message::Settings(SettingsMessage::Agents(AgentsMessage::Refresh)))
                .style(rounded_action_btn_style),
        ]
        .spacing(14)
        .width(Length::Fill)
        .into();
    }

    let title = settings_page_intro(
        tr(app.desktop_locale, "智能体配置"),
        tr(
            app.desktop_locale,
            "全局管理内建与自定义智能体的模型、身份文件、工具、技能和运行参数。",
        ),
    );
    let add_agent_bar = row![
        text_input(tr(app.desktop_locale, "新增智能体 key"), &settings.new_agent_key_input)
            .on_input(|value| {
                Message::Settings(SettingsMessage::Agents(AgentsMessage::AddAgentKeyChanged(value)))
            })
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .width(Length::Fill),
        button(text(tr(app.desktop_locale, "新建智能体")).size(12))
            .padding([8, 14])
            .on_press(Message::Settings(SettingsMessage::Agents(AgentsMessage::AddAgentRequested,)))
            .style(rounded_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let mut content = column![
        title,
        catalog::controls(app),
        add_agent_bar,
        scrollable(container(catalog::grid(app)).padding([0, 7]))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill),
    ]
    .spacing(14)
    .width(Length::Fill)
    .height(Length::Fill);

    if settings.loading {
        content = content.push(
            text(tr(app.desktop_locale, "正在刷新提供商 / 模型 / 工具列表 ..."))
                .size(12)
                .style(settings_muted_text_style),
        );
    }

    profile_modal::overlay(app, content.into())
}

#[cfg(test)]
mod profile_modal_tests;

#[cfg(test)]
mod defaults_tests;

#[cfg(test)]
mod mcp_tests;
