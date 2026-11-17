use crate::help::{settings_hover_help_label, settings_page_heading_with_help, settings_group_heading_with_help};
use crate::message::{Event, AcpMessage};
// 系统设置中 ACP 配置页面的界面拼装与交互消息转换。

use desktop_ui_controls::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_divider,
    settings_error_banner, settings_field_label, settings_group_card,
    settings_icon_tile_style,
    settings_modal_card_style, settings_modal_header, settings_modal_layout,
    settings_modal_overlay, settings_muted_text_style, settings_panel, settings_panel_style, settings_success_banner, settings_text_input,
    settings_value_badge,
};
use desktop_i18n::I18n;
use iced::widget::{Space, button, column, container, responsive, row, scrollable, text};
use iced::{Alignment, Element, Length};
use config_aggregate::AcpAgentConfig;

use crate::acp_diagnostic::diagnostic_status;

use crate::acp::{acp_title, acp_description, setup_hint, command_line, ordered_agents};

pub fn acp_card<'a>(input: Input<'a>, agent: String, config: &'a AcpAgentConfig) -> Element<'a, Event> {
    let enabled = input.state.enabled.contains(&agent);
    let saving = input.state.saving_agent.as_deref() == Some(agent.as_str());
    let copied = input.state.copied_agent.as_deref() == Some(agent.as_str());
    let testing = input.state.diagnostic_agent.as_deref() == Some(agent.as_str())
        && input.state.diagnostic_id.is_some();
    let command = command_line(config);
    let toggle_label = if saving {
        I18n::new(input.locale).__("保存中...")
    } else if enabled {
        I18n::new(input.locale).__("禁用")
    } else {
        I18n::new(input.locale).__("启用")
    };

    let locale = input.locale;
    let tr = move |label: &'static str| I18n::new(locale).__(label);

    let toggle_message = if saving {
        None
    } else {
        Some(Event::Acp(AcpMessage::SetEnabled {
            agent: agent.clone(),
            enabled: !enabled,
        }))
    };
    let summary_agent = agent.clone();
    let summary = responsive(move |size| {
        let title = tr(acp_title(&summary_agent));
        let identity = row![
            container((input.icon)(&summary_agent, 28.0))
                .center_x(44.0)
                .center_y(44.0)
                .style(settings_icon_tile_style),
            row![
                settings_hover_help_label(input.help,
                    I18n::new(locale),
                    input.theme,
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
                    input.state
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

    let installation: Element<'_, Event> =
        if config_acp_install::acp_install_supported(&agent) {
            let installing = input.state.installing_agent.as_deref() == Some(agent.as_str());
            let title = tr("官方安装包 / 安装器");
            row![
                column![
                    settings_hover_help_label(input.help,
                        I18n::new(locale),
                        input.theme,
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
                        input.state
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
        text(I18n::new(input.locale).__(setup_hint(&agent)))
            .size(12)
            .style(settings_muted_text_style),
        container(
            row![
                text(command.clone()).size(12).width(Length::Fill),
                button(
                    text(if copied { "✓" } else { I18n::new(input.locale).__("复制") })
                        .size(12)
                )
                .padding([5, 10])
                .on_press(Event::Acp(AcpMessage::CopyCommand {
                    agent: agent.clone(),
                    command
                }))
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
    if let Some(status) = input.state.diagnostic_results.get(&agent) {
        content = content.push(diagnostic_status(*status, input.frame, locale));
    }
    container(content).padding([16, 18]).width(Length::Fill).style(settings_panel_style).into()
}

fn form_field<'a>(
    label: &'static str,
    control: impl Into<Element<'a, Event>>,
) -> Element<'a, Event> {
    column![settings_field_label(label), control.into()].spacing(6).width(Length::Fill).into()
}

fn acp_message(message: AcpMessage) -> Event {
    Event::Acp(message)
}

pub fn new_agent_form(input: Input<'_>) -> Element<'_, Event> {
    let state = input.state;
    let locale = input.locale;
    let tr = |label: &'static str| I18n::new(locale).__(label);
    let mut name = settings_text_input(tr("名称"), &state.new_agent_name_input);
    if state.editing_agent.is_none() && !state.adding_agent {
        name = name.on_input(|v| acp_message(AcpMessage::NewAgentNameChanged(v)));
    }
    let command = settings_text_input(tr("启动命令"), &state.new_agent_command_input)
        .on_input(|v| acp_message(AcpMessage::NewAgentCommandChanged(v)));
    let can_restore = !state.adding_agent
        && state.editing_agent.as_ref().is_some_and(|agent| state.defaults.contains_key(agent));
    let mut arguments = column![settings_group_heading_with_help(input.help,
        I18n::new(locale),
        input.theme,
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
        body = body.push(diagnostic_status(status, input.frame, locale));
    }
    if let Some(error) = save_error(input) {
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
            settings_hover_help_label(input.help,
                I18n::new(locale),
                input.theme,
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

fn save_error(input: Input<'_>) -> Option<String> {
    let state = input.state;
    state.save_error.as_ref().map(|error| {
        state
            .error_translation
            .as_ref()
            .map(|message| {
                message.render(I18n::new(input.locale)).expect("valid ACP error template")
            })
            .unwrap_or_else(|| error.clone())
    })
}

/// 构建 ACP 系统设置页。
pub fn view(input: Input<'_>) -> Element<'_, Event> {
    let refresh_btn = button(
        text(if input.state.loading {
            I18n::new(input.locale).__("刷新中...")
        } else {
            I18n::new(input.locale).__("刷新")
        })
        .size(13),
    )
    .padding([7, 14])
    .on_press(Event::Acp(AcpMessage::Refresh))
    .style(rounded_action_btn_style);
    let add_btn = button(
        text(if input.state.new_agent_expanded {
            I18n::new(input.locale).__("收起新增")
        } else {
            I18n::new(input.locale).__("新增 ACP")
        })
        .size(13),
    )
    .padding([7, 14])
    .on_press(Event::Acp(AcpMessage::NewAgentPanelToggled))
    .style(rounded_action_btn_style);

    let i18n = I18n::new(input.locale);
    let title = i18n.__("可用 ACP");
    let mut content = column![
        settings_page_heading_with_help(input.help,
            i18n,
            input.theme,
            i18n.__("ACP 配置"),
            i18n.__("查看可用 ACP 智能体，按需初始化本机环境，并通过网关更新启用状态。"),
        ),
        row![add_btn, refresh_btn].spacing(8).align_y(Alignment::Center).wrap(),
        settings_group_card(
            settings_hover_help_label(input.help,
                i18n,
                input.theme,
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

    if let Some(message) = &input.state.status_message {
        content = content.push(settings_success_banner(
            input.state
                .status_translation
                .as_ref()
                .map(|m| {
                    m.render(I18n::new(input.locale)).expect("valid ACP status template")
                })
                .unwrap_or_else(|| message.clone()),
        ));
    }
    if !input.state.new_agent_expanded
        && let Some(error) = save_error(input)
    {
        content = content.push(settings_error_banner(error));
    }

    let agents = ordered_agents(&input.state.catalog);
    if agents.is_empty() {
        content = content.push(settings_panel(
            column![
                text(
                    I18n::new(input.locale)
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
            if let Some(config) = input.state.catalog.get(&agent) {
                cards = cards.push(acp_card(input, agent, config));
            }
        }
        content = content.push(cards);
    }

    let content: Element<'_, Event> =
        scrollable(container(content).padding(iced::Padding::default().left(7.0).right(7.0)))
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .height(Length::Fill)
            .into();

    if input.state.new_agent_expanded {
        settings_modal_overlay(
            Some(content),
            acp_message(AcpMessage::CancelEditor),
            container(new_agent_form(input)).padding(12),
        )
    } else {
        content
    }
}

#[derive(Clone, Copy)]
pub struct Input<'a> {
    pub help: crate::help::Renderer,
    pub state: &'a crate::acp_state::AcpSettingsState,
    pub locale: config_ui::DesktopLocale,
    pub theme: &'a iced::Theme,
    pub frame: usize,
    pub icon: fn(&str, f32) -> Element<'static, Event>,
}
