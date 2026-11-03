//! 系统设置中 ACP 配置页面的界面拼装与交互消息转换。

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_close_button, settings_error_banner, settings_modal_card,
    settings_modal_overlay, settings_muted_text_style, settings_page_intro, settings_panel,
    settings_section_card, settings_success_banner, settings_text_input_style,
    settings_value_badge,
};
use crate::app::message::settings::{AcpMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};
use vw_config_types::config::AcpAgentConfig;

const ACP_ORDER: &[&str] = &[
    "claude", "gemini", "opencode", "codex", "openclaw", "copilot", "pi", "auggie", "cursor",
    "droid", "iflow", "kilocode", "qwen", "kimi", "kiro", "qoder", "trae",
];

pub(super) fn acp_title(agent: &str) -> &'static str {
    match agent {
        "codex" => "Codex CLI",
        "claude" => "Claude Code",
        "gemini" => "Gemini CLI",
        "copilot" => "GitHub Copilot",
        "openclaw" => "OpenClaw",
        "pi" => "PI",
        "auggie" => "Auggie CLI",
        "cursor" => "Cursor",
        "droid" => "Factory droid",
        "iflow" => "iFlow",
        "kilocode" => "Kilocode",
        "opencode" => "OpenCode",
        "qwen" => "Qwen Code",
        "kimi" => "Kimi Code CLI",
        "kiro" => "Kiro",
        "qoder" => "Qoder CLI",
        "trae" => "TRAE CLI",
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
        crate::app::i18n::I18n::new(app.desktop_locale).__("保存中...")
    } else if enabled {
        crate::app::i18n::I18n::new(app.desktop_locale).__("禁用")
    } else {
        crate::app::i18n::I18n::new(app.desktop_locale).__("启用")
    };

    let mut header = row![
        column![
            row![
                text(crate::app::i18n::I18n::new(app.desktop_locale).__(acp_title(&agent)))
                    .size(15),
                settings_value_badge(if enabled {
                    crate::app::i18n::I18n::new(app.desktop_locale).__("已启用")
                } else {
                    crate::app::i18n::I18n::new(app.desktop_locale).__("未启用")
                }),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text(crate::app::i18n::I18n::new(app.desktop_locale).__(acp_description(&agent)))
                .size(12)
                .style(settings_muted_text_style),
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
    header = header.push(
        button(text("编辑").size(13)).padding([7, 14]).style(rounded_action_btn_style).on_press(
            Message::Settings(SettingsMessage::Acp(AcpMessage::EditAgent(agent.clone()))),
        ),
    );
    header = header.push(toggle);

    settings_panel(
        column![
            header,
            text(crate::app::i18n::I18n::new(app.desktop_locale).__(setup_hint(&agent)))
                .size(12)
                .style(settings_muted_text_style),
            container(
                row![
                    text(command.clone()).size(12).width(Length::Fill),
                    button(
                        text(if copied {
                            "✓"
                        } else {
                            crate::app::i18n::I18n::new(app.desktop_locale).__("复制")
                        })
                        .size(12)
                    )
                    .padding([5, 10])
                    .on_press(Message::Settings(SettingsMessage::Acp(AcpMessage::CopyCommand {
                        agent: agent.clone(),
                        command
                    })))
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

fn acp_message(message: AcpMessage) -> Message {
    Message::Settings(SettingsMessage::Acp(message))
}

fn new_agent_form(app: &App) -> Element<'_, Message> {
    let state = &app.acp_settings;
    let mut name = text_input("名称", &state.new_agent_name_input)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style);
    if state.editing_agent.is_none() && !state.adding_agent {
        name = name.on_input(|v| acp_message(AcpMessage::NewAgentNameChanged(v)));
    }
    let command = text_input("启动命令", &state.new_agent_command_input)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .on_input(|v| acp_message(AcpMessage::NewAgentCommandChanged(v)));
    let mut fields = column![
        form_field("名称（已有名称不可修改）", name),
        form_field("启动命令", command),
        text("参数：每项独立保存，保留空格与逗号")
    ]
    .spacing(12);
    for (i, arg) in state.editor_args.iter().enumerate() {
        fields = fields.push(
            row![
                text_input("参数", arg)
                    .padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_text_input_style)
                    .on_input(move |v| acp_message(AcpMessage::ArgumentChanged(i, v))),
                button(text("删除"))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press(acp_message(AcpMessage::RemoveArgument(i)))
            ]
            .spacing(8),
        );
    }
    fields = fields.push(
        button(text("添加参数"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(acp_message(AcpMessage::AddArgument)),
    );
    fields = fields.push(text("环境变量"));
    for (i, (key, value)) in state.editor_env.iter().enumerate() {
        fields = fields.push(
            row![
                text_input("名称", key)
                    .padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_text_input_style)
                    .on_input(move |v| acp_message(AcpMessage::EnvironmentChanged(i, true, v))),
                text_input("值", value)
                    .secure(true)
                    .padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_text_input_style)
                    .on_input(move |v| acp_message(AcpMessage::EnvironmentChanged(i, false, v))),
                button(text("删除"))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press(acp_message(AcpMessage::RemoveEnvironment(i)))
            ]
            .spacing(8),
        );
    }
    fields = fields.push(
        button(text("添加环境变量"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(acp_message(AcpMessage::AddEnvironment)),
    );
    if let Some(error) = &state.save_error {
        fields = fields.push(settings_error_banner(error.clone()));
    }
    let mut save = button(text(if state.adding_agent { "保存中..." } else { "保存" }))
        .padding([8, 16])
        .style(primary_action_btn_style);
    if !state.adding_agent {
        save = save.on_press(acp_message(AcpMessage::AddAgent));
    }
    settings_modal_card(
        column![
            row![
                text(if state.editing_agent.is_some() { "编辑 ACP" } else { "新增 ACP" })
                    .size(18)
                    .width(Length::Fill),
                settings_close_button(acp_message(AcpMessage::CancelEditor))
            ]
            .align_y(Alignment::Center),
            text(
                app.acp_settings
                    .diagnostic_status
                    .as_deref()
                    .unwrap_or("诊断使用当前草稿，在网关主机执行，不发送消息或自动认证")
            )
            .size(12)
            .style(settings_muted_text_style),
            scrollable(container(fields).padding([0, 7]))
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4)
                ))
                .height(Length::Fill),
            row![
                button(text("取消"))
                    .padding([8, 16])
                    .style(rounded_action_btn_style)
                    .on_press(acp_message(AcpMessage::CancelEditor)),
                button(text(if app.acp_settings.diagnostic_id.is_some() {
                    "取消诊断"
                } else {
                    "测试连接 / 重试"
                }))
                .padding([8, 16])
                .style(rounded_action_btn_style)
                .on_press(acp_message(
                    if app.acp_settings.diagnostic_id.is_some() {
                        AcpMessage::CancelDiagnostic
                    } else {
                        AcpMessage::StartDiagnostic
                    }
                )),
                save
            ]
            .spacing(8)
        ]
        .spacing(16),
    )
    .width(Length::Fixed(720.0))
    .height(Length::Fixed(580.0))
    .into()
}

/// 构建 ACP 系统设置页。
pub fn view(app: &App) -> Element<'_, Message> {
    let refresh_btn = button(
        text(if app.acp_settings.loading {
            crate::app::i18n::I18n::new(app.desktop_locale).__("刷新中...")
        } else {
            crate::app::i18n::I18n::new(app.desktop_locale).__("刷新")
        })
        .size(13),
    )
    .padding([7, 14])
    .on_press(Message::Settings(SettingsMessage::Acp(AcpMessage::Refresh)))
    .style(rounded_action_btn_style);
    let add_btn = button(
        text(if app.acp_settings.new_agent_expanded {
            crate::app::i18n::I18n::new(app.desktop_locale).__("收起新增")
        } else {
            crate::app::i18n::I18n::new(app.desktop_locale).__("新增 ACP")
        })
        .size(13),
    )
    .padding([7, 14])
    .on_press(Message::Settings(SettingsMessage::Acp(AcpMessage::NewAgentPanelToggled)))
    .style(rounded_action_btn_style);

    let mut content = column![
        row![
            container(settings_page_intro(
                crate::app::i18n::I18n::new(app.desktop_locale).__("ACP 配置"),
                crate::app::i18n::I18n::new(app.desktop_locale)
                    .__("查看可用 ACP 智能体，按需初始化本机环境，并通过网关更新启用状态。")
            ))
            .width(Length::Fill),
            row![add_btn, refresh_btn].spacing(8),
        ]
        .align_y(Alignment::Start),
        settings_section_card(
            crate::app::i18n::I18n::new(app.desktop_locale).__("可用 ACP"),
            crate::app::i18n::I18n::new(app.desktop_locale)
                .__("启用后会出现在聊天、任务和设计生成的 ACP 后端选择中。")
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(message) = &app.acp_settings.status_message {
        content = content.push(settings_success_banner(
            app.acp_settings
                .status_translation
                .as_ref()
                .map(|m| {
                    m.render(crate::app::i18n::I18n::new(app.desktop_locale))
                        .expect("valid ACP status template")
                })
                .unwrap_or_else(|| message.clone()),
        ));
    }
    if let Some(error) = &app.acp_settings.save_error {
        content = content.push(settings_error_banner(
            app.acp_settings
                .error_translation
                .as_ref()
                .map(|m| {
                    m.render(crate::app::i18n::I18n::new(app.desktop_locale))
                        .expect("valid ACP error template")
                })
                .unwrap_or_else(|| error.clone()),
        ));
    }

    let agents = ordered_agents(app);
    if agents.is_empty() {
        content = content.push(settings_panel(
            column![
                text(
                    crate::app::i18n::I18n::new(app.desktop_locale)
                        .__("暂未从网关加载到 ACP 目录，点击刷新后会重新读取。")
                )
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

    if app.acp_settings.new_agent_expanded {
        settings_modal_overlay(
            Some(content.into()),
            acp_message(AcpMessage::CancelEditor),
            new_agent_form(app),
        )
    } else {
        content.into()
    }
}
#[cfg(test)]
#[path = "system_settings_acp_tests.rs"]
mod system_settings_acp_tests;
