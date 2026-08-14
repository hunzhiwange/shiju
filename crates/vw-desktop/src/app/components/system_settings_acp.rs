//! 系统设置中 ACP 配置页面的界面拼装与交互消息转换。

use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_error_banner,
    settings_muted_text_style, settings_page_intro, settings_panel, settings_section_card,
    settings_success_banner, settings_text_input_style, settings_value_badge,
};
use crate::app::message::settings::{AcpMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};
use vw_config_types::config::AcpAgentConfig;

const ACP_ORDER: &[&str] = &[
    "claude", "gemini", "opencode", "codex", "openclaw", "copilot", "pi", "auggie", "cursor",
    "droid", "iflow", "kilocode", "qwen",
];

pub(super) fn acp_title(agent: &str) -> &'static str {
    match agent {
        "codex" => "Codex CLI",
        "claude" => "Claude Code",
        "gemini" => "Gemini CLI",
        "copilot" => "GitHub Copilot",
        "openclaw" => "OpenClaw",
        "pi" => "Pi ACP",
        "auggie" => "Auggie CLI",
        "cursor" => "Cursor Agent",
        "droid" => "Factory droid",
        "iflow" => "iFlow",
        "kilocode" => "Kilocode",
        "opencode" => "OpenCode",
        "qwen" => "Qwen Code",
        _ => "自定义 ACP",
    }
}

pub(super) fn acp_description(agent: &str) -> &'static str {
    match agent {
        "codex" => "OpenAI Codex ACP 适配器，适合把 Codex CLI 作为外部编码后端。",
        "claude" => "Claude Code ACP 适配器，适合连接本机 Claude Code 工作流。",
        "gemini" => "Gemini CLI 的实验性 ACP 模式，适合 Google Gemini 本地代理流程。",
        "copilot" => "GitHub Copilot CLI 的 ACP/stdio 模式，依赖本机 Copilot 登录状态。",
        "openclaw" => "OpenClaw ACP 后端，适合沿用 OpenClaw 本地代理生态。",
        "pi" => "Pi ACP 适配器，适合轻量对话与辅助代理实验。",
        "auggie" => "Augment Code Auggie CLI 的 ACP 模式。",
        "cursor" => "Cursor Agent 的 ACP 入口，依赖本机 cursor-agent 命令。",
        "droid" => "Factory droid 的 ACP 输出模式。",
        "iflow" => "iFlow 的实验性 ACP 模式。",
        "kilocode" => "Kilocode ACP 后端。",
        "opencode" => "OpenCode ACP 后端。",
        "qwen" => "Qwen Code ACP 后端。",
        _ => "自定义 ACP 后端，会按全局配置中的 command / args 启动。",
    }
}

pub(super) fn setup_hint(agent: &str) -> &'static str {
    match agent {
        "codex" => "初始化：确保 Node.js 可用；首次运行按 Codex CLI 提示完成登录或令牌配置。",
        "claude" => "初始化：安装或登录 Claude Code；非交互运行建议提前完成认证。",
        "gemini" => "初始化：配置 Gemini CLI 的 API Key 或 OAuth；实验模式需要 CLI 版本支持 ACP。",
        "copilot" => "初始化：先完成 GitHub Copilot CLI 登录，再启用 ACP/stdio。",
        "openclaw" => "初始化：确保 openclaw 命令可用，并提前完成它需要的本地认证。",
        "pi" => "初始化：确保 Node.js 可用；npx 会按需拉取 pi-acp 适配器。",
        _ => "初始化：确认命令在 PATH 中可执行，并提前完成该代理需要的认证。",
    }
}

pub(super) fn command_line(config: &AcpAgentConfig) -> String {
    std::iter::once(config.command.as_str())
        .chain(config.args.iter().map(String::as_str))
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn ordered_agents(app: &App) -> Vec<String> {
    let mut names = app.acp_settings.catalog.keys().cloned().collect::<Vec<_>>();
    names.sort_by(|left, right| {
        let left_rank =
            ACP_ORDER.iter().position(|candidate| candidate == left).unwrap_or(ACP_ORDER.len());
        let right_rank =
            ACP_ORDER.iter().position(|candidate| candidate == right).unwrap_or(ACP_ORDER.len());
        (left_rank, left.as_str()).cmp(&(right_rank, right.as_str()))
    });
    names
}

fn acp_card<'a>(app: &'a App, agent: String, config: &'a AcpAgentConfig) -> Element<'a, Message> {
    let enabled = app.acp_settings.enabled.contains(&agent);
    let saving = app.acp_settings.saving_agent.as_deref() == Some(agent.as_str());
    let copied = app.acp_settings.copied_agent.as_deref() == Some(agent.as_str());
    let command = command_line(config);
    let toggle_label = if saving {
        "保存中..."
    } else if enabled {
        "禁用"
    } else {
        "启用"
    };

    let mut header = row![
        column![
            row![
                text(acp_title(&agent)).size(15),
                settings_value_badge(if enabled { "已启用" } else { "未启用" }),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text(acp_description(&agent)).size(12).style(settings_muted_text_style),
        ]
        .spacing(5)
        .width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .spacing(12);

    let toggle_message = if saving {
        None
    } else {
        Some(Message::Settings(SettingsMessage::Acp(AcpMessage::SetEnabled {
            agent: agent.clone(),
            enabled: !enabled,
        })))
    };
    let mut toggle =
        button(text(toggle_label).size(13)).padding([7, 14]).style(rounded_action_btn_style);
    if let Some(message) = toggle_message {
        toggle = toggle.on_press(message);
    }
    header = header.push(toggle);

    settings_panel(
        column![
            header,
            text(setup_hint(&agent)).size(12).style(settings_muted_text_style),
            container(
                row![
                    text(command.clone()).size(12).width(Length::Fill),
                    button(text(if copied { "✓" } else { "复制" }).size(12))
                        .padding([5, 10])
                        .on_press(Message::Settings(SettingsMessage::Acp(
                            AcpMessage::CopyCommand { agent: agent.clone(), command }
                        )))
                        .style(rounded_action_btn_style),
                ]
                .align_y(Alignment::Center)
                .spacing(8)
            )
            .padding([9, 12])
            .width(Length::Fill)
            .style(|theme: &iced::Theme| {
                let palette = theme.extended_palette();
                iced::widget::container::Style {
                    text_color: Some(theme.palette().text.scale_alpha(0.92)),
                    background: Some(iced::Background::Color(
                        palette.background.weak.color.scale_alpha(0.58),
                    )),
                    border: iced::Border {
                        radius: 8.0.into(),
                        width: 1.0,
                        color: palette.background.strong.color.scale_alpha(0.55),
                    },
                    ..Default::default()
                }
            }),
        ]
        .spacing(12),
    )
    .into()
}

fn form_field<'a>(
    label: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![text(label).size(11).style(settings_muted_text_style), control.into()]
        .spacing(6)
        .width(Length::Fill)
        .into()
}

fn new_agent_form(app: &App) -> Element<'_, Message> {
    let name = text_input("例如：my-agent", &app.acp_settings.new_agent_name_input)
        .on_input(|value| {
            Message::Settings(SettingsMessage::Acp(AcpMessage::NewAgentNameChanged(value)))
        })
        .padding([8, 10])
        .style(settings_text_input_style);
    let command = text_input("例如：npx", &app.acp_settings.new_agent_command_input)
        .on_input(|value| {
            Message::Settings(SettingsMessage::Acp(AcpMessage::NewAgentCommandChanged(value)))
        })
        .padding([8, 10])
        .style(settings_text_input_style);
    let args =
        text_input("例如：-y, @scope/my-acp（逗号分隔）", &app.acp_settings.new_agent_args_input)
            .on_input(|value| {
                Message::Settings(SettingsMessage::Acp(AcpMessage::NewAgentArgsChanged(value)))
            })
            .padding([8, 10])
            .style(settings_text_input_style);

    let mut save_button = button(text(if app.acp_settings.adding_agent {
        "新增中..."
    } else {
        "新增并启用"
    }))
    .padding([8, 16])
    .style(primary_action_btn_style);
    if !app.acp_settings.adding_agent {
        save_button =
            save_button.on_press(Message::Settings(SettingsMessage::Acp(AcpMessage::AddAgent)));
    }

    settings_panel(
        column![
            text("新增自定义 ACP").size(15),
            text("名称必须唯一；启动命令需已安装并可从 PATH 访问。")
                .size(12)
                .style(settings_muted_text_style),
            row![form_field("名称", name), form_field("启动命令", command)]
                .spacing(12)
                .align_y(Alignment::End),
            form_field("参数", args),
            container(save_button).width(Length::Fill).align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(12),
    )
    .into()
}

/// 构建 ACP 系统设置页。
pub fn view(app: &App) -> Element<'_, Message> {
    let refresh_btn =
        button(text(if app.acp_settings.loading { "刷新中..." } else { "刷新" }).size(13))
            .padding([7, 14])
            .on_press(Message::Settings(SettingsMessage::Acp(AcpMessage::Refresh)))
            .style(rounded_action_btn_style);
    let add_btn = button(
        text(if app.acp_settings.new_agent_expanded { "收起新增" } else { "新增 ACP" }).size(13),
    )
    .padding([7, 14])
    .on_press(Message::Settings(SettingsMessage::Acp(AcpMessage::NewAgentPanelToggled)))
    .style(rounded_action_btn_style);

    let mut content = column![
        row![
            container(settings_page_intro(
                "ACP 配置",
                "查看可用 ACP 智能体，按需初始化本机环境，并通过网关更新启用状态。"
            ))
            .width(Length::Fill),
            row![add_btn, refresh_btn].spacing(8),
        ]
        .align_y(Alignment::Start),
        settings_section_card("可用 ACP", "启用后会出现在聊天、任务和设计生成的 ACP 后端选择中。"),
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(message) = &app.acp_settings.status_message {
        content = content.push(settings_success_banner(message));
    }
    if let Some(error) = &app.acp_settings.save_error {
        content = content.push(settings_error_banner(error));
    }
    if app.acp_settings.new_agent_expanded {
        content = content.push(new_agent_form(app));
    }

    let agents = ordered_agents(app);
    if agents.is_empty() {
        content = content.push(settings_panel(
            column![
                text("暂未从网关加载到 ACP 目录，点击刷新后会重新读取。")
                    .size(12)
                    .style(settings_muted_text_style)
            ]
            .spacing(0),
        ));
    } else {
        for agent in agents {
            if let Some(config) = app.acp_settings.catalog.get(&agent) {
                content = content.push(acp_card(app, agent.clone(), config));
            }
        }
    }

    content.into()
}
#[cfg(test)]
#[path = "system_settings_acp_tests.rs"]
mod system_settings_acp_tests;
