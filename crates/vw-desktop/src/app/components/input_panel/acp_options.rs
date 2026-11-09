//! Backend-advertised session options, using the shared settings modal controls.
use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_close_button, settings_field_label,
    settings_hover_help_label, settings_modal_header, settings_modal_layout_with_scroll_id,
    settings_modal_overlay_with_escape, settings_muted_text_style, settings_panel_style,
};
use crate::app::{App, Message, i18n::I18n, message::ChatMessage};
use iced::widget::{Space, button as action, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};

use super::styles::selectable_list_button_style;
use vw_gateway_client::vw_api_types::acp_session::{AcpOptionKind, AcpSessionSelection};

pub(crate) const OPTIONS_SCROLL: &str = "acp_session_options_scroll";

pub(super) fn entry<'a>(
    app: &'a App,
    runtime: Option<&crate::app::state::SessionRuntimeState>,
) -> Option<Element<'a, Message>> {
    let runtime =
        runtime.filter(|runtime| runtime.acp_agent.is_some() && runtime.draft.is_none())?;
    let i18n = I18n::new(app.desktop_locale);
    Some(
        action(text(i18n.__("ACP 选项")).size(13))
            .padding([6, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe(
                (!runtime.is_requesting
                    && !runtime.acp_options_pending
                    && runtime.acp_source_pending.is_none())
                .then_some(Message::Chat(ChatMessage::AcpOptionsOpen)),
            )
            .into(),
    )
}

pub(crate) fn with_modal<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(runtime) =
        app.current_session_runtime_ref().filter(|r| r.acp_options_open && r.acp_agent.is_some())
    else {
        return base;
    };
    let i18n = I18n::new(app.desktop_locale);
    let close = Message::Chat(ChatMessage::AcpOptionsClose);
    let mut fields = column![
        container(access_status(app)).padding(12).width(Length::Fill).style(settings_panel_style),
        container(source_controls(app)).padding(12).width(Length::Fill).style(settings_panel_style),
    ]
    .spacing(12);
    // Managed sessions can expose saved reasoning options between connections.
    let options =
        runtime.acp_options.options.iter().filter(|option| option.kind != AcpOptionKind::Mode);
    if options.clone().next().is_none() {
        let hint = if runtime.acp_options.connected || app.current_session_has_started() {
            "当前会话没有可调整的选项"
        } else {
            "连接会话后可读取后端支持的选项"
        };
        fields = fields.push(text(hint).size(13).style(settings_muted_text_style));
    }
    let can_select = !runtime.acp_options_pending
        && !runtime.is_requesting
        && runtime.acp_source_pending.is_none();
    for option in options {
        let model_locked = option.kind == AcpOptionKind::Model
            || matches!(&option.kind, AcpOptionKind::Config { id } if id == "model");
        // Copilot may display the first level before an explicit selection has been saved.
        let can_confirm_current = runtime.acp_agent.as_deref() == Some("copilot")
            && runtime.acp_options.acp_model_source
                == vw_gateway_client::vw_api_types::acp_model_source::AcpModelSource::SessionModel
            && matches!(&option.kind, AcpOptionKind::Config { id } if id == "reasoning_effort");
        let mut choices = column![settings_field_label(&option.name)].spacing(6);
        if can_confirm_current {
            choices = choices.push(
                text(i18n.__("选择档位后将在下一轮生效")).size(12).style(settings_muted_text_style),
            );
        }
        for choice in &option.choices {
            let selected = option.current == choice.id;
            let control = action(
                text(format!("{}{}", if selected { "✓ " } else { "" }, choice.name))
                    .size(14)
                    .wrapping(text::Wrapping::WordOrGlyph),
            )
            .width(Length::Fill)
            .padding([8, 12])
            .style(move |theme, status| {
                let mut style = selectable_list_button_style(theme, status, selected);
                if !selected && status == iced::widget::button::Status::Disabled {
                    style.text_color = theme.palette().text.scale_alpha(0.55);
                }
                style
            })
            .on_press_maybe(
                (can_select && !model_locked && (!selected || can_confirm_current)).then(|| {
                    Message::Chat(ChatMessage::AcpOptionSelected(AcpSessionSelection {
                        kind: option.kind.clone(),
                        value: choice.id.clone(),
                    }))
                }),
            );
            choices = choices.push(super::session_lock_tooltip(control, i18n, model_locked));
        }
        fields = fields
            .push(container(choices).padding(12).width(Length::Fill).style(settings_panel_style));
    }
    let header = settings_modal_header(
        column![
            text("ACP 会话选项").size(20),
            text("查看权限状态，调整模型来源和会话选项").size(13).style(settings_muted_text_style),
        ]
        .spacing(4),
        settings_close_button(close.clone()),
    );
    let footer = row![
        action(text(if runtime.acp_options_pending { "处理中…" } else { "刷新" }))
            .padding([8, 16])
            .style(rounded_action_btn_style)
            .on_press_maybe(can_select.then_some(Message::Chat(ChatMessage::AcpOptionsOpen))),
        Space::new().width(Length::Fill),
        action(text("关闭"))
            .padding([8, 16])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let card = settings_modal_layout_with_scroll_id(
        header,
        fields,
        Some(footer.into()),
        Some(OPTIONS_SCROLL),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .max_width(520)
    .max_height(560);
    let card = container(card).padding(12).center_x(Length::Fill).center_y(Length::Fill);
    settings_modal_overlay_with_escape(base, close, card)
}

pub(super) fn source_controls(app: &App) -> Element<'_, Message> {
    use vw_gateway_client::vw_api_types::acp_model_source::{
        AcpModelSource, AcpModelSourceCapability, AcpModelSourceRejectionReason,
    };
    let runtime = app.current_session_runtime();
    let started = app.current_session_has_started();
    let i18n = I18n::new(app.desktop_locale);
    let client = "客户端模型";
    let native = "智能体原生模型";
    let description = format!(
        "{client}：使用当前供应商与模型，无需重复填写密钥\n{native}：使用执行主机上的配置与登录状态"
    );
    let title = "模型来源";
    let mut controls = column![
        settings_hover_help_label(
            i18n,
            &app.app_theme,
            title,
            settings_field_label(title),
            description,
        )
        .wrap()
    ]
    .spacing(6);
    for (source, label) in
        [(AcpModelSource::SessionModel, client), (AcpModelSource::CliLocal, native)]
    {
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
        let control =
            action(text(format!("{}{}", if selected { "✓ " } else { "" }, label)).size(13))
                .width(Length::Fill)
                .padding([8, 10])
                .style(move |theme: &Theme, status| {
                    if selected {
                        selectable_list_button_style(theme, status, true)
                    } else {
                        rounded_action_btn_style(theme, status)
                    }
                })
                .on_press_maybe(
                    (matches!(capability, Some(AcpModelSourceCapability::Supported { .. }))
                        && !selected
                        && !started
                        && !runtime.is_requesting
                        && !runtime.acp_options_pending
                        && runtime.acp_source_pending.is_none())
                    .then_some(Message::Chat(ChatMessage::AcpModelSourceSelected(source))),
                );
        controls = controls.push(super::session_lock_tooltip(control, i18n, started));
        if let Some(reason) = reason {
            controls = controls.push(text(reason).size(12).style(settings_muted_text_style));
        }
    }
    if runtime.acp_model_source == AcpModelSource::SessionModel {
        let model = if runtime.auto_model { "自动选择".to_string() } else { runtime.model };
        controls = controls.push(
            text(format!("当前客户端 Provider/model：{model}"))
                .size(12)
                .wrapping(text::Wrapping::WordOrGlyph)
                .style(settings_muted_text_style),
        );
    }
    controls = controls.push(
        text(if started {
            "会话开始后不能切换模型"
        } else if runtime.acp_source_pending.is_some() {
            "正在等待执行主机确认并保存…"
        } else {
            "切换来源将创建新的 ACP 会话；原生上下文不继承"
        })
        .size(12)
        .style(settings_muted_text_style),
    );
    controls.into()
}

fn access_status(app: &App) -> Element<'_, Message> {
    use vw_gateway_client::vw_api_types::acp_session::AcpAccessSupport;
    let runtime = app.current_session_runtime();
    let can_change = super::send_controls::has_permission_context(app)
        && !runtime.is_requesting
        && !runtime.acp_options_pending
        && runtime.acp_source_pending.is_none();
    let mut status = column![
        settings_field_label("工具权限"),
        super::send_controls::acp_permission_modes(runtime.full_access_enabled, can_change),
        text("审批模式自动允许符合工作区安全边界的普通文件编辑；其他操作仍可能要求审批或被拒绝。")
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(6);
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
            AcpAccessSupport::Unavailable if !observed.requested => "按审批规则执行",
            AcpAccessSupport::Unavailable => "完全访问不可用",
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
        status = status.push(text("生效能力：未验证；后端未连接，连接后可刷新查看").size(13));
    }
    status
        .push(
            text("此处仅指工具权限；用户问答、计划确认及组织策略仍可能需要确认。")
                .size(12)
                .style(settings_muted_text_style),
        )
        .into()
}

#[cfg(test)]
#[path = "acp_options_ui_refresh_42_tests.rs"]
pub(super) mod ui_refresh_42_tests;
