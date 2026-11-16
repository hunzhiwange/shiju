use desktop_ui_controls::system_settings_common::*;
use desktop_i18n::settings_models::connection_text::text as connection_text;
use crate::view_message::{Message, SettingsMessage};
use crate::view_message as message;
use crate::state::CustomProviderModelTab;
use crate::view_style::ViewStyle;
use iced::widget::{button, checkbox, column, container, responsive, row, text};
use iced::{Alignment, Background, Element, Length, Theme};
use std::borrow::Cow;
const MODALITIES: [&str; 5] = ["text", "image", "audio", "video", "pdf"];

pub fn custom_modal_card_style(theme: &Theme) -> container::Style {
    let mut style = settings_modal_card_style(theme);
    // Keep the provider dialog from showing through the active model dialog.
    if let Some(Background::Color(color)) = &mut style.background {
        color.a = 1.0;
    }
    style
}

pub fn custom_segment_style(
    theme: &Theme,
    status: button::Status,
    is_active: bool,
) -> button::Style {
    let mut style = settings_segment_button_style(theme, status, is_active);
    if is_active {
        style.text_color = theme.palette().text;
    }
    style
}

pub fn stacked_field_row<'a, M: 'a>(
    label: impl Into<Element<'a, M>>,
    control: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    container(column![label.into(), control.into()].spacing(6))
        .padding([14, 0])
        .width(Length::Fill)
        .into()
}

fn text_row_with_help<'a>(
    style: ViewStyle<'a>,
    label: &'a str,
    description: impl Into<Cow<'a, str>>,
    placeholder: &'a str,
    value: &'a str,
    on_input: fn(String) -> SettingsMessage,
) -> Element<'a, Message> {
    responsive_field_with_help(style, label, description, move || {
        settings_text_input(placeholder, value)
            .on_input(move |value| Message::Settings(on_input(value)))
            .width(Length::Fill)
    })
}

pub(crate) fn responsive_field_with_help<'a, Control>(style: ViewStyle<'a>, label: &'a str, description: impl Into<Cow<'a,str>>, control: impl Fn() -> Control + 'a) -> Element<'a,Message>
where Control: Into<Element<'a,Message>> {
    responsive_field(description, control, move |description, wide| {
        let label = style.help_label(label, settings_field_label(label), description);
        if wide { label.width(Length::Fill).wrap().into() } else { label.wrap().into() }
    })
}
/// 同一响应式字段同时用于提供商与模型表单；帮助文本由调用方按消息类型构建。
pub fn responsive_field<'a, M: Clone + 'a, Control>(
    description: impl Into<Cow<'a,str>>,
    control: impl Fn() -> Control + 'a,
    help_label: impl Fn(Cow<'a,str>, bool) -> Element<'a,M> + 'a,
) -> Element<'a,M>
where Control: Into<Element<'a,M>> {
    let description = description.into();
    responsive(move |size| {
        let control = control().into();
        if size.width >= 600.0 {
            settings_flexible_row_with_label(help_label(description.clone(), true), 460.0, control)
                .padding([6, 0]).into()
        } else {
            column![help_label(description.clone(), false), control].spacing(6).width(Length::Fill).into()
        }
    }).height(Length::Shrink).into()
}

fn modality_checked(values: Option<&Vec<String>>, modality: &str) -> bool {
    values.map(|values| values.iter().any(|value| value == modality)).unwrap_or(modality == "text")
}

fn effective_capability(
    override_value: Option<bool>,
    inherited: Option<&crate::state::ModelCapabilityDefaults>,
    inherited_value: impl FnOnce(&crate::state::ModelCapabilityDefaults) -> bool,
    default: bool,
) -> bool {
    override_value.or_else(|| inherited.map(inherited_value)).unwrap_or(default)
}

fn capability_checkbox(
    checked: bool,
    label: &'static str,
    on_toggle: impl Fn(bool) -> Message + 'static,
) -> Element<'static, Message> {
    checkbox(checked).label(label).on_toggle(on_toggle).style(settings_checkbox_style).into()
}

fn model_tab_button(
    label: &'static str,
    tab: CustomProviderModelTab,
    active_tab: CustomProviderModelTab,
) -> Element<'static, Message> {
    settings_tab_button(
        label,
        tab == active_tab,
        Some(Message::Settings(message::SettingsMessage::CustomProviderModelTabSelected(tab))),
    )
    .into()
}

/// 构建或处理 `view_overlays` 对应的界面片段与交互数据。
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
pub fn modal_card<'a>(mm: &'a crate::state::CustomProviderModelModalState, error: Option<&'a desktop_i18n::settings_models::messages::Feedback>, style: ViewStyle<'a>) -> (Message, Element<'a, Message>) {
    let close_message = Message::Settings(message::SettingsMessage::CustomProviderModelClose);
    let close_btn = settings_close_button(close_message.clone());

    let title = if mm.edit_index.is_some() {
        connection_text(style.locale, "编辑模型")
    } else {
        connection_text(style.locale, "添加模型")
    };

            let id_row = text_row_with_help(
        style,
        connection_text(style.locale, "模型 ID"),
        connection_text(style.locale, "用于请求 provider 的真实 model_id。"),
        connection_text(style.locale, "例如 gpt-4o-mini"),
        &mm.model_id,
        SettingsMessage::CustomProviderModelModalIdChanged,
    );

    let name_row = text_row_with_help(
        style,
        connection_text(style.locale, "显示名称"),
        connection_text(style.locale, "仅影响 UI 展示，可留空回退为 model_id。"),
        connection_text(style.locale, "可选"),
        &mm.display_name,
        SettingsMessage::CustomProviderModelModalNameChanged,
    );

    let family_row = text_row_with_help(
        style,
        connection_text(style.locale, "模型家族"),
        connection_text(style.locale, "可选 family 元数据，用于区分同一厂商的模型系列。"),
        connection_text(style.locale, "例如 qwen、gpt、claude"),
        &mm.family,
        SettingsMessage::CustomProviderModelModalFamilyChanged,
    );

    let empty_limit = connection_text(style.locale, "留空使用默认值");
    let input_example = connection_text(style.locale, "例如 200000。");
    let output_example = connection_text(style.locale, "例如 4096。");
    let context_description =
        connection_text(style.locale, "总上下文窗口；留空时沿用内置基线或安全默认值。");
    let input_description =
        connection_text(style.locale, "最大输入窗口；留空时沿用内置基线或安全默认值。");
    let output_description =
        connection_text(style.locale, "最大输出窗口；0 可表示该模型不产生文本输出。");
    let context_limit_row = text_row_with_help(
        style,
        connection_text(style.locale, "上下文限制（token）"),
        format!("{context_description} {input_example}"),
        empty_limit,
        &mm.context_limit,
        SettingsMessage::CustomProviderModelModalContextLimitChanged,
    );
    let input_limit_row = text_row_with_help(
        style,
        connection_text(style.locale, "最大输入（token）"),
        format!("{input_description} {input_example}"),
        empty_limit,
        &mm.input_limit,
        SettingsMessage::CustomProviderModelModalInputLimitChanged,
    );
    let output_limit_row = text_row_with_help(
        style,
        connection_text(style.locale, "最大输出（token）"),
        format!("{output_description} {output_example}"),
        empty_limit,
        &mm.output_limit,
        SettingsMessage::CustomProviderModelModalOutputLimitChanged,
    );

    let reasoning = capability_checkbox(
        effective_capability(
            mm.reasoning,
            mm.inherited_capabilities.as_ref(),
            |capabilities| capabilities.reasoning,
            false,
        ),
        connection_text(style.locale, "推理"),
        |value| {
            Message::Settings(
                message::SettingsMessage::CustomProviderModelModalReasoningChanged(value),
            )
        },
    );
    let tool_call = capability_checkbox(
        effective_capability(
            mm.tool_call,
            mm.inherited_capabilities.as_ref(),
            |capabilities| capabilities.tool_call,
            true,
        ),
        connection_text(style.locale, "工具调用"),
        |value| {
            Message::Settings(
                message::SettingsMessage::CustomProviderModelModalToolCallChanged(value),
            )
        },
    );
    let temperature = capability_checkbox(
        effective_capability(
            mm.temperature,
            mm.inherited_capabilities.as_ref(),
            |capabilities| capabilities.temperature,
            false,
        ),
        connection_text(style.locale, "温度"),
        |value| {
            Message::Settings(
                message::SettingsMessage::CustomProviderModelModalTemperatureChanged(value),
            )
        },
    );
    let attachment = capability_checkbox(
        effective_capability(
            mm.attachment,
            mm.inherited_capabilities.as_ref(),
            |capabilities| capabilities.attachment,
            false,
        ),
        connection_text(style.locale, "附件"),
        |value| {
            Message::Settings(
                message::SettingsMessage::CustomProviderModelModalAttachmentChanged(value),
            )
        },
    );
    let interleaved = capability_checkbox(
        effective_capability(
            mm.interleaved,
            mm.inherited_capabilities.as_ref(),
            |capabilities| capabilities.interleaved,
            false,
        ),
        connection_text(style.locale, "交错输入输出"),
        |value| {
            Message::Settings(
                message::SettingsMessage::CustomProviderModelModalInterleavedChanged(value),
            )
        },
    );
    let field_help = |label, description| {
        let label = connection_text(style.locale, label);
        style.help_label(
            label,
            settings_field_label(label),
            connection_text(style.locale, description),
        )
        .wrap()
    };
    let capabilities_row = stacked_field_row(
        field_help("能力", "未显式设置时沿用基线；新模型默认支持文本和工具调用。"),
        row![reasoning, tool_call, temperature, attachment, interleaved]
            .spacing(14)
            .align_y(Alignment::Center)
            .wrap(),
    );

    let input_modality_checks = MODALITIES.into_iter().map(|modality| {
        let inherited = mm
            .inherited_capabilities
            .as_ref()
            .map(|capabilities| &capabilities.input_modalities);
        let checked = modality_checked(mm.input_modalities.as_ref().or(inherited), modality);
        let modality = modality.to_string();
        checkbox(checked)
            .label(
                desktop_i18n::settings_models::modality_label(
                    style.locale,
                    &modality,
                )
                .to_string(),
            )
            .on_toggle(move |value| {
                Message::Settings(
                    message::SettingsMessage::CustomProviderModelModalInputModalityToggled(
                        modality.clone(),
                        value,
                    ),
                )
            })
            .style(settings_checkbox_style)
            .into()
    });
    let output_modality_checks = MODALITIES.into_iter().map(|modality| {
        let inherited = mm
            .inherited_capabilities
            .as_ref()
            .map(|capabilities| &capabilities.output_modalities);
        let checked = modality_checked(mm.output_modalities.as_ref().or(inherited), modality);
        let modality = modality.to_string();
        checkbox(checked)
            .label(
                desktop_i18n::settings_models::modality_label(
                    style.locale,
                    &modality,
                )
                .to_string(),
            )
            .on_toggle(move |value| {
                Message::Settings(
                    message::SettingsMessage::CustomProviderModelModalOutputModalityToggled(
                        modality.clone(),
                        value,
                    ),
                )
            })
            .style(settings_checkbox_style)
            .into()
    });
    let modalities_row = column![
        stacked_field_row(
            field_help(
                "输入模态",
                "选择 provider 接受的输入类型；留空时按文本模型默认值处理。"
            ),
            row(input_modality_checks).spacing(12).align_y(Alignment::Center).wrap(),
        ),
        settings_divider(),
        stacked_field_row(
            field_help(
                "输出模态",
                "选择 provider 返回的输出类型；留空时按文本模型默认值处理。"
            ),
            row(output_modality_checks).spacing(12).align_y(Alignment::Center).wrap(),
        ),
    ]
    .spacing(0);

    let info_tab = column![
        style.group_heading(
            connection_text(style.locale, "模型信息"),
            connection_text(
                style.locale,
                "模型 ID 会直接用于请求；显示名称和家族仅用于元数据。"
            )
        ),
        settings_group_card(
            column![id_row, settings_divider(), name_row, settings_divider(), family_row]
                .spacing(0),
        ),
    ]
    .spacing(16);
    let limits_tab = column![
        style.group_heading(
            connection_text(style.locale, "限制"),
            connection_text(
                style.locale,
                "空值明确表示沿用内置基线或安全默认值，数值单位为 token。"
            )
        ),
        settings_group_card(
            column![
                context_limit_row,
                settings_divider(),
                input_limit_row,
                settings_divider(),
                output_limit_row,
            ]
            .spacing(0),
        ),
    ]
    .spacing(16);
    let capabilities_tab = column![
        style.group_heading(
            connection_text(style.locale, "能力与模态"),
            connection_text(
                style.locale,
                "只保存用户显式设置的覆盖，保持自动发现模型的其它配置。"
            )
        ),
        settings_group_card(
            column![capabilities_row, settings_divider(), modalities_row].spacing(0)
        ),
    ]
    .spacing(16);

    let tabs = settings_tab_row([
        model_tab_button(
            connection_text(style.locale, "模型信息"),
            CustomProviderModelTab::Info,
            mm.active_tab,
        ),
        model_tab_button(
            connection_text(style.locale, "限制"),
            CustomProviderModelTab::Limits,
            mm.active_tab,
        ),
        model_tab_button(
            connection_text(style.locale, "能力与模态"),
            CustomProviderModelTab::Capabilities,
            mm.active_tab,
        ),
    ])
    .wrap();

    let body: Element<'_, Message> = match mm.active_tab {
        CustomProviderModelTab::Info => info_tab.into(),
        CustomProviderModelTab::Limits => limits_tab.into(),
        CustomProviderModelTab::Capabilities => capabilities_tab.into(),
    };

    let mut header = column![
        settings_modal_header(
            style.help_label(
                title,
                text(title).size(20),
                connection_text(style.locale, "填写模型 ID 和展示名称。"),
            )
            .wrap(),
            close_btn,
        ),
        tabs,
    ]
    .spacing(12);
    if let Some(error) = error {
        header = header.push(settings_error_banner(error.display(style.locale)));
    }

    let actions = row![
        button(text(connection_text(style.locale, "取消")))
            .on_press(close_message.clone())
            .padding([8, 14])
            .style(rounded_action_btn_style),
        button(text(connection_text(style.locale, "保存")))
            .on_press(Message::Settings(message::SettingsMessage::CustomProviderModelModalSave))
            .padding([8, 14])
            .style(primary_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .width(Length::Fill)
    .wrap()
    .align_x(iced::alignment::Horizontal::Right);

    let card = settings_modal_layout(header, body, Some(actions.into()))
        .style(custom_modal_card_style)
        .width(840)
        .height(680);
    (close_message, card.into())
}

/// 将局部模型草稿叠加到调用方页面，并沿用调用方的消息映射。
pub fn view_overlays<'a, M: Clone + 'a>(
    modal: Option<&'a crate::state::CustomProviderModelModalState>,
    error: Option<&'a desktop_i18n::settings_models::messages::Feedback>,
    style: ViewStyle<'a>,
    dialog: Element<'a, M>,
    map_message: impl Fn(Message) -> M + Clone + 'a,
) -> Element<'a, M> {
    let Some(modal) = modal else { return dialog; };
    let (close, card) = modal_card(modal, error, style);
    settings_modal_overlay(Some(dialog), map_message(close), card.map(map_message))
}

#[cfg(test)]
#[path = "custom_model_modal_tests.rs"]
mod tests;
