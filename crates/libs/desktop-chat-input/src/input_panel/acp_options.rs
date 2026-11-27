//! Backend-advertised session options, using the shared settings modal controls.
use desktop_ui_controls::system_settings_common::{
    help::settings_hover_help_label,
    rounded_action_btn_style, settings_close_button, settings_field_label, settings_modal_header,
    settings_modal_layout_with_scroll_id,
    settings_modal_overlay_with_escape, settings_muted_text_style,
};
use desktop_i18n::I18n;
use super::session_lock_tooltip;
use iced::widget::{Space, button as action, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};

use super::styles::selectable_list_button_style;
use super::surface_styles::BOTTOM_BAR_ICON_BUTTON_SIZE;
use vw_api_acp_model::acp_session::{AcpOptionKind, AcpSessionOption, AcpSessionOptions, AcpSessionSelection};

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
    pub full_access_enabled: bool,
}

pub const OPTIONS_SCROLL: &str = "acp_session_options_scroll";

pub fn reasoning_effort_option(options: &AcpSessionOptions) -> Option<&AcpSessionOption> {
    options.options.iter().find(|option| {
        !option.choices.is_empty()
            && matches!(
                &option.kind,
                AcpOptionKind::Config { id }
                    if matches!(id.as_str(), "reasoning_effort" | "thought_level" | "effort")
            )
    })
}

fn options_editable(runtime: &OptionsState<'_>) -> bool {
    !runtime.acp_options_pending && !runtime.is_requesting && !runtime.source_pending
}

fn can_confirm_current_option(runtime: &OptionsState<'_>, kind: &AcpOptionKind) -> bool {
    // Copilot may display the first level before an explicit selection has been saved.
    runtime.acp_agent == Some("copilot")
        && runtime.acp_options.acp_model_source
            == vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
        && matches!(kind, AcpOptionKind::Config { id } if id == "reasoning_effort")
}

pub fn reasoning_effort_control<Message: Clone + 'static>(
    runtime: &OptionsState<'_>,
    emit: fn(Action) -> Message,
) -> Option<Element<'static, Message>> {
    let option = reasoning_effort_option(runtime.acp_options)?;
    let can_edit = options_editable(runtime);
    let can_confirm_current = can_confirm_current_option(runtime, &option.kind);
    let choices = row(option.choices.iter().map(|choice| {
        let selected = choice.id == option.current;
        action(text(choice.name.clone()).size(12))
            .padding([5, 8])
            .style(move |theme, status| {
                let mut style = selectable_list_button_style(theme, status, selected);
                if !can_edit && !selected {
                    style.text_color = theme.palette().text.scale_alpha(0.55);
                }
                style
            })
            .on_press_maybe((can_edit && (!selected || can_confirm_current)).then(|| {
                emit(Action::AcpOptionSelected(AcpSessionSelection {
                    kind: option.kind.clone(),
                    value: choice.id.clone(),
                }))
            }))
            .into()
    }));
    Some(container(column![
        text(runtime.i18n.__("推理强度")).size(12).style(settings_muted_text_style),
        choices.spacing(4).wrap().vertical_spacing(4),
    ].spacing(6))
        .padding([8, 10])
        .width(Length::Fill)
        .style(super::reasoning_selector::reasoning_control_surface)
        .into())
}

pub fn entry<'a, Message: Clone + 'static>(
    i18n: I18n, visible: bool, enabled: bool, emit: fn(Action) -> Message,
) -> Option<Element<'a, Message>> {
    if !visible { return None; }
    Some(
        action(container(text(i18n.__("ACP 选项")).size(13)).center_y(BOTTOM_BAR_ICON_BUTTON_SIZE))
            .padding([0, 10])
            .style(|theme, status| super::surface_styles::composer_selector_button_style(theme, status, false))
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
    let mut fields = column![access, source].spacing(16).width(Length::Fill);
    // Managed sessions can expose saved reasoning options between connections.
    let options =
        runtime.acp_options.options.iter().filter(|option| option.kind != AcpOptionKind::Mode);
    if options.clone().next().is_none() {
        let hint = if runtime.acp_options.connected || started {
            "当前会话没有可调整的选项"
        } else {
            "连接会话后可读取后端支持的选项"
        };
        fields = fields.push(text(i18n.__(hint)).size(13).style(settings_muted_text_style));
    }
    let can_select = options_editable(runtime);
    for option in options {
        let model_locked = option.kind == AcpOptionKind::Model
            || matches!(&option.kind, AcpOptionKind::Config { id } if id == "model");
        let can_confirm_current = can_confirm_current_option(runtime, &option.kind);
        let label: Element<'a, Message> = if can_confirm_current {
            settings_hover_help_label(
                settings_field_label(&option.name),
                text(i18n.__("选择档位后将在下一轮生效")).size(13).into(),
            )
            .into()
        } else {
            settings_field_label(&option.name).into()
        };
        let mut choices = column![label].spacing(6).width(Length::Fill);
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
        fields = fields.push(choices);
    }
    let header = settings_modal_header(
        text(i18n.__("ACP 会话选项")).size(20),
        settings_close_button(close.clone()),
    );
    let footer = row![
        action(text(i18n.__(if runtime.acp_options_pending { "处理中…" } else { "刷新" })))
            .padding([8, 16])
            .style(rounded_action_btn_style)
            .on_press_maybe(can_select.then_some(emit(Action::AcpOptionsOpen))),
        Space::new().width(Length::Fill),
        action(text(i18n.__("关闭")))
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
    runtime: &OptionsState<'_>,
    started: bool,
    help: impl FnOnce(&'static str, String) -> Element<'a, Message>,
    emit: fn(Action) -> Message,
) -> Element<'a, Message> {
    let i18n = runtime.i18n;
    use vw_api_acp_model::acp_model_source::{
        AcpModelSource, AcpModelSourceCapability, AcpModelSourceRejectionReason,
    };
    let client = i18n.__("客户端模型");
    let native = i18n.__("智能体原生模型");
    let mut description = i18n.__(
        "客户端模型：使用当前供应商与模型，无需重复填写密钥\n智能体原生模型：使用执行主机上的配置与登录状态",
    ).to_owned();
    let mut controls = column![].spacing(6);
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
                AcpModelSourceRejectionReason::UnsupportedCli
                    if source == AcpModelSource::SessionModel
                        && runtime.acp_options.verified_cli_release.is_some() =>
                {
                    "网关当前未提供客户端模型能力，请检查平台和 ACP 安装。"
                }
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
                    let mut style = selectable_list_button_style(theme, status, selected);
                    if !selected && status == iced::widget::button::Status::Disabled {
                        style.text_color = theme.palette().text.scale_alpha(0.55);
                    }
                    style
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
        controls = controls.push(control);
        if let Some(reason) = reason {
            description.push_str(&format!("\n{label} ({})", i18n.__(reason)));
        }
    }
    let status = if started {
        "会话开始后无法更改。如需更换，请新建会话。"
    } else if runtime.source_pending {
        "正在等待执行主机确认并保存…"
    } else {
        "切换来源将创建新的 ACP 会话；原生上下文不继承"
    };
    description.push_str("\n\n");
    description.push_str(i18n.__(status));
    column![help("模型来源", description), controls].spacing(6).into()
}

pub(crate) fn permission_section<'a, Message: Clone + 'static>(
    i18n: I18n, permission: Element<'a, Message>,
) -> Element<'a, Message> {
    let help = settings_hover_help_label(
        settings_field_label(i18n.__("工具权限")),
        column![
            text(i18n.__("审批模式自动允许符合工作区安全边界的普通文件编辑；其他操作仍可能要求审批或被拒绝。"))
                .size(13)
                .wrapping(text::Wrapping::WordOrGlyph),
            text(i18n.__("此处仅指工具权限；用户问答、计划确认及组织策略仍可能需要确认。"))
                .size(13)
                .wrapping(text::Wrapping::WordOrGlyph),
        ]
        .spacing(8)
        .into(),
    );
    column![help, permission].spacing(6).into()
}

pub fn with_session_modal<'a, Message: Clone + 'static>(runtime: &OptionsState<'a>, started: bool, base: Element<'a, Message>, can_change: bool, source: Element<'a, Message>, emit: fn(Action) -> Message, emit_send: fn(super::send_message::Message) -> Message) -> Element<'a, Message> {
    let permission = super::send_controls::acp_permission_modes(runtime.i18n, runtime.full_access_enabled, can_change).map(emit_send);
    let access = permission_section(runtime.i18n, permission);
    with_modal(runtime, runtime.i18n, started, base, access, source, emit)
}
