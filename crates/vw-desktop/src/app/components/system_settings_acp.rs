//! 系统设置中 ACP 配置页面的界面拼装与交互消息转换。

use crate::app::components::input_panel::icons::acp_agent_icon;
use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_divider,
    settings_error_banner, settings_field_label, settings_group_card,
    settings_group_heading_with_help, settings_hover_help_label, settings_icon_tile_style,
    settings_modal_card_style, settings_modal_header, settings_modal_layout,
    settings_modal_overlay, settings_muted_text_style, settings_page_heading_with_help,
    settings_panel, settings_panel_style, settings_success_banner, settings_text_input,
    settings_value_badge,
};
use crate::app::i18n::I18n;
use crate::app::message::settings::{AcpMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{Space, button, column, container, responsive, row, scrollable, text};
use iced::{Alignment, Element, Length};
use vw_config_types::config::AcpAgentConfig;

#[path = "system_settings_acp_diagnostic.rs"]
mod diagnostic;
use diagnostic::diagnostic_status;

const ACP_ORDER: &[&str] = &[
    "claude",
    "gemini",
    "opencode",
    "codex",
    "openclaw",
    "copilot",
    "pi",
    "auggie",
    "cursor",
    "droid",
    "iflow",
    "kilocode",
    "qwen",
    "kimi",
    "kiro",
    "qoder",
    "trae",
    "codebuddy",
    "goose",
    "vibe",
    "hermes",
    "snow",
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
        "codebuddy" => "CodeBuddy",
        "goose" => "Goose",
        "vibe" => "Mistral Vibe",
        "hermes" => "Hermes Agent",
        "snow" => "Snow CLI",
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
        "kimi" => "Kimi Code CLI 的 ACP 后端。",
        "kiro" => "Kiro 官方 CLI 的 ACP 后端。",
        "qoder" => "Qoder CLI 的 ACP 后端。",
        "trae" => "TRAE CLI 的 ACP 服务。",
        "codebuddy" => "腾讯 CodeBuddy 原生 ACP 后端。",
        "goose" => "Goose 原生 ACP 后端。",
        "vibe" => "Mistral Vibe 的 vibe-acp 后端。",
        "hermes" => "Nous Research Hermes Agent 的 ACP 后端。",
        "snow" => "Snow CLI 的 ACP 模式。",
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
    let testing = app.acp_settings.diagnostic_agent.as_deref() == Some(agent.as_str())
        && app.acp_settings.diagnostic_id.is_some();
    let command = command_line(config);
    let toggle_label = if saving {
        I18n::new(app.desktop_locale).__("保存中...")
    } else if enabled {
        I18n::new(app.desktop_locale).__("禁用")
    } else {
        I18n::new(app.desktop_locale).__("启用")
    };

    let locale = app.desktop_locale;
    let tr = move |label: &'static str| I18n::new(locale).__(label);

    let toggle_message = if saving {
        None
    } else {
        Some(Message::Settings(SettingsMessage::Acp(AcpMessage::SetEnabled {
            agent: agent.clone(),
            enabled: !enabled,
        })))
    };
    let summary_agent = agent.clone();
    let summary = responsive(move |size| {
        let title = tr(acp_title(&summary_agent));
        let identity = row![
            container(acp_agent_icon(&summary_agent, 28.0))
                .center_x(44.0)
                .center_y(44.0)
                .style(settings_icon_tile_style),
            row![
                settings_hover_help_label(
                    I18n::new(locale),
                    &app.app_theme,
                    title,
                    text(title).size(16),
                    tr(acp_description(&summary_agent)),
                )
                .wrap(),
                settings_value_badge(tr(if enabled { "已启用" } else { "未启用" })),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .wrap(),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill);
        let actions = row![
            button(text(tr("编辑")).size(13))
                .padding([7, 12])
                .style(rounded_action_btn_style)
                .on_press(acp_message(AcpMessage::EditAgent(summary_agent.clone()))),
            button(text(tr(if testing { "取消测试" } else { "测试连接" })).size(13))
                .padding([7, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe(if testing {
                    Some(acp_message(AcpMessage::CancelDiagnostic))
                } else {
                    app.acp_settings
                        .diagnostic_id
                        .is_none()
                        .then(|| acp_message(AcpMessage::TestAgent(summary_agent.clone())))
                }),
            button(text(toggle_label).size(13))
                .padding([7, 12])
                .style(if enabled || saving {
                    rounded_action_btn_style
                } else {
                    primary_action_btn_style
                })
                .on_press_maybe(toggle_message.clone()),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap();
        if size.width >= 540.0 {
            row![identity, actions].spacing(12).align_y(Alignment::Center).into()
        } else {
            column![identity, actions].spacing(8).into()
        }
    })
    .height(Length::Shrink);

    let installation: Element<'_, Message> =
        if vw_config_types::acp_install::acp_install_command(&agent).is_some() {
            let installing = app.acp_settings.installing_agent.as_deref() == Some(agent.as_str());
            let title = tr("官方安装包 / 安装器");
            row![
                column![
                    settings_hover_help_label(
                        I18n::new(locale),
                        &app.app_theme,
                        title,
                        text(title).size(12),
                        tr("网关按操作系统选择并执行安装程序，无需复制命令。"),
                    )
                    .wrap(),
                    text(tr("在网关主机安装，安装后仍需完成认证。"))
                        .size(12)
                        .style(settings_muted_text_style),
                ]
                .spacing(4)
                .width(Length::Fill),
                button(text(tr(if installing { "安装中…" } else { "一键安装" })).size(13))
                    .padding([7, 14])
                    .style(rounded_action_btn_style)
                    .on_press_maybe(
                        app.acp_settings
                            .installing_agent
                            .is_none()
                            .then(|| { acp_message(AcpMessage::InstallAgent(agent.clone())) })
                    ),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        } else {
            text(tr("请自行安装自定义 ACP。")).size(12).style(settings_muted_text_style).into()
        };

    let mut content = column![
        summary,
        settings_divider(),
        installation,
        text(I18n::new(app.desktop_locale).__(setup_hint(&agent)))
            .size(12)
            .style(settings_muted_text_style),
        container(
            row![
                text(command.clone()).size(12).width(Length::Fill),
                button(
                    text(if copied { "✓" } else { I18n::new(app.desktop_locale).__("复制") })
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
    .spacing(10);
    if let Some(status) = app.acp_settings.diagnostic_results.get(&agent) {
        content = content.push(diagnostic_status(*status, app.status_animation_frame, locale));
    }
    container(content).padding([16, 18]).width(Length::Fill).style(settings_panel_style).into()
}

fn form_field<'a>(
    label: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![settings_field_label(label), control.into()].spacing(6).width(Length::Fill).into()
}

fn acp_message(message: AcpMessage) -> Message {
    Message::Settings(SettingsMessage::Acp(message))
}

fn new_agent_form(app: &App) -> Element<'_, Message> {
    let state = &app.acp_settings;
    let locale = app.desktop_locale;
    let tr = |label: &'static str| I18n::new(locale).__(label);
    let mut name = settings_text_input(tr("名称"), &state.new_agent_name_input);
    if state.editing_agent.is_none() && !state.adding_agent {
        name = name.on_input(|v| acp_message(AcpMessage::NewAgentNameChanged(v)));
    }
    let command = settings_text_input(tr("启动命令"), &state.new_agent_command_input)
        .on_input(|v| acp_message(AcpMessage::NewAgentCommandChanged(v)));
    let can_restore = !state.adding_agent
        && state.editing_agent.as_ref().is_some_and(|agent| state.defaults.contains_key(agent));
    let mut arguments = column![settings_group_heading_with_help(
        I18n::new(locale),
        &app.app_theme,
        tr("参数"),
        tr("每项独立保存，保留空格与逗号。"),
    )]
    .spacing(10);
    for (i, arg) in state.editor_args.iter().enumerate() {
        arguments = arguments.push(
            row![
                settings_text_input(tr("参数"), arg)
                    .on_input(move |v| acp_message(AcpMessage::ArgumentChanged(i, v))),
                button(text(tr("删除")).size(13))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press(acp_message(AcpMessage::RemoveArgument(i)))
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }
    arguments = arguments.push(
        row![
            button(text(tr("添加参数")).size(13))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press(acp_message(AcpMessage::AddArgument)),
            button(text(tr("恢复默认参数")).size(13))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe(
                    can_restore.then(|| acp_message(AcpMessage::RestoreDefaultArguments))
                ),
        ]
        .spacing(8)
        .wrap(),
    );
    let mut environment = column![settings_field_label(tr("环境变量"))].spacing(10);
    for (i, (key, value)) in state.editor_env.iter().enumerate() {
        environment = environment.push(
            row![
                settings_text_input(tr("名称"), key)
                    .on_input(move |v| acp_message(AcpMessage::EnvironmentChanged(i, true, v))),
                settings_text_input(tr("值"), value)
                    .secure(true)
                    .on_input(move |v| acp_message(AcpMessage::EnvironmentChanged(i, false, v))),
                button(text(tr("删除")).size(13))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press(acp_message(AcpMessage::RemoveEnvironment(i)))
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }
    environment = environment.push(
        row![
            button(text(tr("添加环境变量")).size(13))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press(acp_message(AcpMessage::AddEnvironment)),
            button(text(tr("恢复默认环境变量")).size(13))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe(
                    can_restore.then(|| acp_message(AcpMessage::RestoreDefaultEnvironment))
                ),
        ]
        .spacing(8)
        .wrap(),
    );
    let mut body = column![].spacing(12);
    if let Some(status) = state.diagnostic_status {
        body = body.push(diagnostic_status(status, app.status_animation_frame, locale));
    }
    if let Some(error) = save_error(app) {
        body = body.push(settings_error_banner(error));
    }
    body = body
        .push(settings_panel(
            column![
                form_field(tr("名称（已有名称不可修改）"), name),
                form_field(tr("启动命令"), command)
            ]
            .spacing(12),
        ))
        .push(settings_panel(arguments))
        .push(settings_panel(environment));
    let title = tr(if state.editing_agent.is_some() { "编辑 ACP" } else { "新增 ACP" });
    let header = settings_modal_header(
        column![
            settings_hover_help_label(
                I18n::new(locale),
                &app.app_theme,
                title,
                text(title).size(20),
                tr("诊断使用当前草稿进行连接测试，不发送消息或自动认证。"),
            )
            .wrap(),
            text(tr("诊断在网关主机执行。")).size(13).style(settings_muted_text_style),
        ]
        .spacing(4),
        settings_close_button(acp_message(AcpMessage::CancelEditor)),
    );
    let testing = state.diagnostic_id.is_some();
    let footer = row![
        Space::new().width(Length::Fill),
        button(text(tr(if testing { "取消诊断" } else { "测试连接 / 重试" })).size(13))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(acp_message(if testing {
                AcpMessage::CancelDiagnostic
            } else {
                AcpMessage::StartDiagnostic
            })),
        button(text(tr("取消")).size(13))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(acp_message(AcpMessage::CancelEditor)),
        button(text(tr(if state.adding_agent { "保存中..." } else { "保存" })).size(13))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe((!state.adding_agent).then(|| acp_message(AcpMessage::AddAgent))),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    settings_modal_layout(header, body, Some(footer.into()))
        .style(|theme| {
            let mut style = settings_modal_card_style(theme);
            if let Some(iced::Background::Color(color)) = &mut style.background {
                color.a = 1.0;
            }
            style
        })
        .width(720)
        .height(580)
        .into()
}

fn save_error(app: &App) -> Option<String> {
    let state = &app.acp_settings;
    state.save_error.as_ref().map(|error| {
        state
            .error_translation
            .as_ref()
            .map(|message| {
                message.render(I18n::new(app.desktop_locale)).expect("valid ACP error template")
            })
            .unwrap_or_else(|| error.clone())
    })
}

/// 构建 ACP 系统设置页。
pub fn view(app: &App) -> Element<'_, Message> {
    let refresh_btn = button(
        text(if app.acp_settings.loading {
            I18n::new(app.desktop_locale).__("刷新中...")
        } else {
            I18n::new(app.desktop_locale).__("刷新")
        })
        .size(13),
    )
    .padding([7, 14])
    .on_press(Message::Settings(SettingsMessage::Acp(AcpMessage::Refresh)))
    .style(rounded_action_btn_style);
    let add_btn = button(
        text(if app.acp_settings.new_agent_expanded {
            I18n::new(app.desktop_locale).__("收起新增")
        } else {
            I18n::new(app.desktop_locale).__("新增 ACP")
        })
        .size(13),
    )
    .padding([7, 14])
    .on_press(Message::Settings(SettingsMessage::Acp(AcpMessage::NewAgentPanelToggled)))
    .style(rounded_action_btn_style);

    let i18n = I18n::new(app.desktop_locale);
    let title = i18n.__("可用 ACP");
    let mut content = column![
        settings_page_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("ACP 配置"),
            i18n.__("查看可用 ACP 智能体，按需初始化本机环境，并通过网关更新启用状态。"),
        ),
        row![add_btn, refresh_btn].spacing(8).align_y(Alignment::Center).wrap(),
        settings_group_card(
            settings_hover_help_label(
                i18n,
                &app.app_theme,
                title,
                text(title).size(14),
                i18n.__("启用后会出现在聊天、任务和设计生成的 ACP 后端选择中。"),
            )
            .wrap(),
        )
        .padding([14, 16]),
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(message) = &app.acp_settings.status_message {
        content = content.push(settings_success_banner(
            app.acp_settings
                .status_translation
                .as_ref()
                .map(|m| {
                    m.render(I18n::new(app.desktop_locale)).expect("valid ACP status template")
                })
                .unwrap_or_else(|| message.clone()),
        ));
    }
    if !app.acp_settings.new_agent_expanded
        && let Some(error) = save_error(app)
    {
        content = content.push(settings_error_banner(error));
    }

    let agents = ordered_agents(app);
    if agents.is_empty() {
        content = content.push(settings_panel(
            column![
                text(
                    I18n::new(app.desktop_locale)
                        .__("暂未从网关加载到 ACP 目录，点击刷新后会重新读取。")
                )
                .size(12)
                .style(settings_muted_text_style)
            ]
            .spacing(0),
        ));
    } else {
        let mut cards = column![].spacing(8);
        for agent in agents {
            if let Some(config) = app.acp_settings.catalog.get(&agent) {
                cards = cards.push(acp_card(app, agent, config));
            }
        }
        content = content.push(cards);
    }

    let content: Element<'_, Message> =
        scrollable(container(content).padding(iced::Padding::default().left(7.0).right(7.0)))
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .height(Length::Fill)
            .into();

    if app.acp_settings.new_agent_expanded {
        settings_modal_overlay(
            Some(content),
            acp_message(AcpMessage::CancelEditor),
            container(new_agent_form(app)).padding(12),
        )
    } else {
        content
    }
}
#[cfg(test)]
#[path = "system_settings_acp_tests.rs"]
mod system_settings_acp_tests;

#[cfg(test)]
#[path = "system_settings_acp_ui_refresh_18_tests.rs"]
mod ui_refresh_18_tests;
