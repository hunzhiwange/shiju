//! Backend-advertised session options, using the shared settings modal controls.
use desktop_ui_controls::system_settings_common::{
    rounded_action_btn_style, settings_close_button, settings_field_label,
 settings_modal_header, settings_modal_layout_with_scroll_id,
    settings_modal_overlay_with_escape, settings_muted_text_style, settings_panel_style,
};
use desktop_i18n::I18n;
use super::session_lock_tooltip;
use iced::widget::{Space, button as action, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};

use super::styles::selectable_list_button_style;
use vw_api_acp_model::acp_session::{AcpOptionKind, AcpSessionSelection};

#[derive(Debug, Clone)]
pub enum Action {
    AcpOptionsOpen,
    AcpOptionsClose,
    AcpOptionSelected(AcpSessionSelection),
    AcpModelSourceSelected(vw_api_acp_model::acp_model_source::AcpModelSource),
}

/// Borrowed ACP option inputs; the desktop retains request execution and persistence.
pub struct OptionsState<'a> {
    pub i18n: I18n,
    pub acp_agent: Option<&'a str>,
    pub acp_options: &'a vw_api_acp_model::acp_session::AcpSessionOptions,
    pub acp_model_source: vw_api_acp_model::acp_model_source::AcpModelSource,
    pub is_requesting: bool,
    pub acp_options_pending: bool,
    pub source_pending: bool,
    pub auto_model: bool,
    pub model: &'a str,
    pub full_access_enabled: bool,
    pub acp_rebuild_required: bool,
}

pub const OPTIONS_SCROLL: &str = "acp_session_options_scroll";

pub fn entry<'a, Message: Clone + 'static>(
    i18n: I18n, visible: bool, enabled: bool, emit: fn(Action) -> Message,
) -> Option<Element<'a, Message>> {
    if !visible { return None; }
    Some(
        action(text(i18n.__("ACP 选项")).size(13))
            .padding([6, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe(
                enabled
                .then_some(emit(Action::AcpOptionsOpen)),
            )
            .into(),
    )
}

pub fn with_modal<'a, Message: Clone + 'static>(
    runtime: &OptionsState<'a>, i18n: I18n, started: bool,
    base: Element<'a, Message>, access: Element<'a, Message>, source: Element<'a, Message>,
    emit: fn(Action) -> Message,
) -> Element<'a, Message> {
    let close = emit(Action::AcpOptionsClose);
    let mut fields = column![
        container(access).padding(12).width(Length::Fill).style(settings_panel_style),
        container(source).padding(12).width(Length::Fill).style(settings_panel_style),
    ]
    .spacing(12);
    // Managed sessions can expose saved reasoning options between connections.
    let options =
        runtime.acp_options.options.iter().filter(|option| option.kind != AcpOptionKind::Mode);
    if options.clone().next().is_none() {
        let hint = if runtime.acp_options.connected || started {
            "当前会话没有可调整的选项"
        } else {
            "连接会话后可读取后端支持的选项"
        };
        fields = fields.push(text(hint).size(13).style(settings_muted_text_style));
    }
    let can_select = !runtime.acp_options_pending
        && !runtime.is_requesting
        && !runtime.source_pending;
    for option in options {
        let model_locked = option.kind == AcpOptionKind::Model
            || matches!(&option.kind, AcpOptionKind::Config { id } if id == "model");
        // Copilot may display the first level before an explicit selection has been saved.
        let can_confirm_current = runtime.acp_agent.as_deref() == Some("copilot")
            && runtime.acp_options.acp_model_source
                == vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
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
                    emit(Action::AcpOptionSelected(AcpSessionSelection {
                        kind: option.kind.clone(),
                        value: choice.id.clone(),
                    }))
                }),
            );
            choices = choices.push(session_lock_tooltip(control, i18n, model_locked));
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
            .on_press_maybe(can_select.then_some(emit(Action::AcpOptionsOpen))),
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

pub fn source_controls<'a, Message: Clone + 'static>(
    runtime: &OptionsState<'_>, started: bool, help: Element<'a, Message>,
    emit: fn(Action) -> Message,
) -> Element<'a, Message> {
    use vw_api_acp_model::acp_model_source::{
        AcpModelSource, AcpModelSourceCapability, AcpModelSourceRejectionReason,
    };
    let client = "客户端模型";
    let native = "智能体原生模型";
    let mut controls = column![help].spacing(6);
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
                        && !runtime.source_pending)
                    .then_some(emit(Action::AcpModelSourceSelected(source))),
                );
        controls = controls.push(session_lock_tooltip(control, runtime.i18n, started));
        if let Some(reason) = reason {
            controls = controls.push(text(reason).size(12).style(settings_muted_text_style));
        }
    }
    if runtime.acp_model_source == AcpModelSource::SessionModel {
        let model = if runtime.auto_model { "自动选择".to_string() } else { runtime.model.to_owned() };
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
        } else if runtime.source_pending {
            "正在等待执行主机确认并保存…"
        } else {
            "切换来源将创建新的 ACP 会话；原生上下文不继承"
        })
        .size(12)
        .style(settings_muted_text_style),
    );
    controls.into()
}

pub fn access_status<'a, Message: Clone + 'static>(
    runtime: &OptionsState<'a>, permission: Element<'a, Message>,
) -> Element<'a, Message> {
    use vw_api_acp_model::acp_session::AcpAccessSupport;
    let mut status = column![
        settings_field_label("工具权限"),
        permission,
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

pub fn with_session_modal<'a, Message: Clone + 'static>(runtime: &OptionsState<'a>, started: bool, base: Element<'a, Message>, can_change: bool, source: Element<'a, Message>, emit: fn(Action) -> Message, emit_send: fn(super::send_message::Message) -> Message) -> Element<'a, Message> {
    let permission = super::send_controls::acp_permission_modes(runtime.full_access_enabled, can_change).map(emit_send);
    let access = access_status(runtime, permission);
    with_modal(runtime, runtime.i18n, started, base, access, source, emit)
}
