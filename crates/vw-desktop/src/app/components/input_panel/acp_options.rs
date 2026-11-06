//! Backend-advertised session options, using the shared settings modal controls.
use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_modal_card,
    settings_modal_overlay,
};
use crate::app::{App, Message, message::ChatMessage};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button as action, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};
use vw_gateway_client::vw_api_types::acp_session::AcpSessionSelection;

pub(crate) fn with_modal<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(runtime) =
        app.current_session_runtime_ref().filter(|r| r.acp_options_open && r.acp_agent.is_some())
    else {
        return base;
    };
    let close = Message::Chat(ChatMessage::AcpOptionsClose);
    let mut fields = column![access_status(app), source_controls(app)].spacing(16);
    if !runtime.acp_options.connected {
        fields = fields.push(text("连接会话后可读取后端支持的选项").size(13));
    } else if runtime.acp_options.options.is_empty() {
        fields = fields.push(text("后端未提供可调整的会话选项").size(13));
    }
    for option in &runtime.acp_options.options {
        let mut choices = column![text(&option.name).size(14)].spacing(6);
        for choice in &option.choices {
            let selected = option.current == choice.id;
            let control = action(
                text(format!("{}{}", if selected { "✓ " } else { "" }, choice.name)).size(13),
            )
            .width(Length::Fill)
            .padding([8, 12])
            .style(if selected { primary_action_btn_style } else { rounded_action_btn_style })
            .on_press_maybe(
                (!selected && !runtime.acp_options_pending && !runtime.is_requesting).then(|| {
                    Message::Chat(ChatMessage::AcpOptionSelected(AcpSessionSelection {
                        kind: option.kind.clone(),
                        value: choice.id.clone(),
                    }))
                }),
            );
            choices = choices.push(control);
        }
        fields = fields.push(choices);
    }
    let contents = scrollable(container(fields).padding([0, 7]))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fixed(320.0));
    let card = settings_modal_card(
        column![
            row![
                text("ACP 会话选项").size(20),
                Space::new().width(Length::Fill),
                settings_close_button(close.clone())
            ]
            .align_y(Alignment::Center),
            text("选择后即时保存；失败时保留原值").size(12),
            contents,
            row![
                action(text(if runtime.acp_options_pending { "处理中…" } else { "刷新" }))
                    .padding([8, 16])
                    .style(rounded_action_btn_style)
                    .on_press_maybe(
                        (!runtime.acp_options_pending)
                            .then_some(Message::Chat(ChatMessage::AcpOptionsOpen))
                    ),
                Space::new().width(Length::Fill),
                action(text("关闭"))
                    .padding([8, 16])
                    .style(rounded_action_btn_style)
                    .on_press(close.clone())
            ]
            .spacing(8)
        ]
        .spacing(16),
    )
    .width(Length::Fixed(480.0));
    settings_modal_overlay(Some(base), close, card)
}

pub(super) fn source_controls(app: &App) -> Element<'_, Message> {
    use vw_gateway_client::vw_api_types::acp_model_source::{
        AcpModelSource, AcpModelSourceCapability, AcpModelSourceRejectionReason,
    };
    let runtime = app.current_session_runtime();
    let mut controls = column![text("模型来源").size(14)].spacing(6);
    for (source, label, description) in [
        (AcpModelSource::SessionModel, "客户端模型", "使用当前供应商与模型，无需重复填写密钥"),
        (AcpModelSource::CliLocal, "智能体原生模型", "使用执行主机上的配置与登录状态"),
    ] {
        let capability =
            runtime.acp_options.model_source_capabilities.iter().find(
                |capability| match capability {
                    AcpModelSourceCapability::Supported { source: value }
                    | AcpModelSourceCapability::Unsupported { source: value, .. } => {
                        *value == source
                    }
                },
            );
        let reason = match capability {
            Some(AcpModelSourceCapability::Unsupported { reason, .. }) => Some(match reason {
                AcpModelSourceRejectionReason::UnsupportedCli => {
                    "此 CLI 尚无经过验证的客户端模型入口"
                }
                AcpModelSourceRejectionReason::UnsupportedProtocol => "所选模型协议不受支持",
                AcpModelSourceRejectionReason::UnsupportedModelCapability => "所选模型能力不受支持",
                AcpModelSourceRejectionReason::ModelUnavailable => "所选模型不可用或已禁用",
                AcpModelSourceRejectionReason::MissingCredentials => "请先配置供应商凭据",
            }),
            _ => None,
        };
        let selected = runtime.acp_model_source == source;
        controls = controls.push(
            action(
                column![
                    text(format!("{}{}", if selected { "✓ " } else { "" }, label)).size(13),
                    text(description).size(11),
                ]
                .spacing(4),
            )
            .width(Length::Fill)
            .padding([8, 10])
            .style(if selected { primary_action_btn_style } else { rounded_action_btn_style })
            .on_press_maybe(
                (matches!(capability, Some(AcpModelSourceCapability::Supported { .. }))
                    && !selected
                    && !runtime.is_requesting
                    && !runtime.acp_options_pending
                    && runtime.acp_source_pending.is_none())
                .then_some(Message::Chat(ChatMessage::AcpModelSourceSelected(source))),
            ),
        );
        if let Some(reason) = reason {
            controls = controls.push(text(reason).size(12));
        }
    }
    if runtime.acp_model_source == AcpModelSource::SessionModel {
        let model = if runtime.auto_model { "自动选择".to_string() } else { runtime.model };
        controls = controls.push(text(format!("当前客户端 Provider/model：{model}")).size(12));
    }
    controls = controls.push(
        text(if runtime.acp_source_pending.is_some() {
            "正在等待执行主机确认并保存…"
        } else {
            "切换来源将创建新的 ACP 会话；原生上下文不继承"
        })
        .size(11),
    );
    controls.into()
}

fn access_status(app: &App) -> Element<'_, Message> {
    use vw_gateway_client::vw_api_types::acp_session::AcpAccessSupport;
    let runtime = app.current_session_runtime();
    let requested = if runtime.full_access_enabled { "已开启" } else { "已关闭" };
    let mut status = column![text(format!("完全访问授权：{requested}")).size(14)].spacing(6);
    let observed = runtime.acp_options.access_status.as_ref();
    if runtime.acp_rebuild_required
        || observed.is_some_and(|s| s.requested != runtime.full_access_enabled)
    {
        status = status.push(text("生效能力：未验证；下次发送时同步当前授权").size(13));
    } else if let Some(observed) = observed {
        let label = match observed.support {
            AcpAccessSupport::Native => "原生权限已确认",
            AcpAccessSupport::ClientOnly => "仅客户端",
            AcpAccessSupport::Partial => "部分支持",
            AcpAccessSupport::Unverified => "未验证",
            AcpAccessSupport::Unavailable => "未启用或不可用",
            AcpAccessSupport::Failed => "同步失败或未确认",
        };
        status = status
            .push(text(format!("生效能力：{label}")).size(13))
            .push(
                text(format!(
                    "后端：{}；版本：{}",
                    observed.backend,
                    observed.version.as_deref().unwrap_or("未提供")
                ))
                .size(12),
            )
            .push(text(observed.explanation.clone()).size(12));
    } else {
        status = status.push(text("生效能力：未验证；连接后刷新查看，读取不会启动后端").size(13));
    }
    status
        .push(text("此处仅指工具权限；用户问答、计划确认及组织策略仍可能需要确认。").size(12))
        .into()
}
