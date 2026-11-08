//! 系统设置中模型提供商配置的目录、表单与弹窗能力。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_close_button, settings_control_with_hint, settings_divider, settings_error_banner,
    settings_field_label, settings_group_card, settings_group_heading, settings_modal_card_style,
    settings_modal_header, settings_modal_layout, settings_modal_overlay,
    settings_muted_text_style, settings_responsive_row, settings_segment_button_style,
    settings_tab_button, settings_tab_row, settings_text_input,
};
use crate::app::i18n::settings_models::connection_text::text as connection_text;
use crate::app::state::CustomProviderModelTab;
use crate::app::{App, Message, message};
use iced::widget::{Space, button, checkbox, column, container, row, text};
use iced::{Alignment, Background, Element, Length, Theme};

const MODALITIES: [&str; 5] = ["text", "image", "audio", "video", "pdf"];

pub(super) fn custom_modal_card_style(theme: &Theme) -> container::Style {
    let mut style = settings_modal_card_style(theme);
    // Keep the provider dialog from showing through the active model dialog.
    if let Some(Background::Color(color)) = &mut style.background {
        color.a = 1.0;
    }
    style
}

pub(super) fn custom_segment_style(
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

pub(super) fn custom_modal_heading<'a>(
    title: &'a str,
    description: &'a str,
) -> Element<'a, Message> {
    column![text(title).size(20), text(description).size(13).style(settings_muted_text_style)]
        .spacing(4)
        .into()
}

pub(super) fn field_row<'a>(
    label: &'a str,
    description: &'a str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        column![settings_field_label(label), settings_control_with_hint(control, description)]
            .spacing(6),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

pub(super) fn text_row<'a>(
    label: &'a str,
    description: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    settings_responsive_row(label, description, move || {
        settings_text_input(placeholder, value).on_input(on_input.clone()).width(Length::Fill)
    })
}

fn modality_checked(values: Option<&Vec<String>>, modality: &str) -> bool {
    values.map(|values| values.iter().any(|value| value == modality)).unwrap_or(modality == "text")
}

fn effective_capability(
    override_value: Option<bool>,
    inherited: Option<&crate::app::state::ModelCapabilityDefaults>,
    inherited_value: impl FnOnce(&crate::app::state::ModelCapabilityDefaults) -> bool,
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
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.provider_settings;
    let mut base = dialog;

    if let Some(mm) = &s.custom_model_modal {
        let close_message = Message::Settings(message::SettingsMessage::CustomProviderModelClose);
        let close_btn = settings_close_button(close_message.clone());

        let title = if mm.edit_index.is_some() {
            connection_text(app.desktop_locale, "编辑模型")
        } else {
            connection_text(app.desktop_locale, "添加模型")
        };

        let id_row = text_row(
            connection_text(app.desktop_locale, "模型 ID"),
            connection_text(app.desktop_locale, "用于请求 provider 的真实 model_id。"),
            connection_text(app.desktop_locale, "例如 gpt-4o-mini"),
            &mm.model_id,
            |v| Message::Settings(message::SettingsMessage::CustomProviderModelModalIdChanged(v)),
        );

        let name_row = text_row(
            connection_text(app.desktop_locale, "显示名称"),
            connection_text(app.desktop_locale, "仅影响 UI 展示，可留空回退为 model_id。"),
            connection_text(app.desktop_locale, "可选"),
            &mm.display_name,
            |v| Message::Settings(message::SettingsMessage::CustomProviderModelModalNameChanged(v)),
        );

        let family_row = text_row(
            connection_text(app.desktop_locale, "模型家族"),
            connection_text(app.desktop_locale, "可选 family 元数据，用于区分同一厂商的模型系列。"),
            connection_text(app.desktop_locale, "例如 qwen、gpt、claude"),
            &mm.family,
            |v| {
                Message::Settings(message::SettingsMessage::CustomProviderModelModalFamilyChanged(
                    v,
                ))
            },
        );

        let context_limit_row = text_row(
            connection_text(app.desktop_locale, "上下文限制（token）"),
            connection_text(app.desktop_locale, "总上下文窗口；留空时沿用内置基线或安全默认值。"),
            connection_text(app.desktop_locale, "例如 200000，留空使用默认值"),
            &mm.context_limit,
            |v| {
                Message::Settings(
                    message::SettingsMessage::CustomProviderModelModalContextLimitChanged(v),
                )
            },
        );
        let input_limit_row = text_row(
            connection_text(app.desktop_locale, "最大输入（token）"),
            connection_text(app.desktop_locale, "最大输入窗口；留空时沿用内置基线或安全默认值。"),
            connection_text(app.desktop_locale, "例如 200000，留空使用默认值"),
            &mm.input_limit,
            |v| {
                Message::Settings(
                    message::SettingsMessage::CustomProviderModelModalInputLimitChanged(v),
                )
            },
        );
        let output_limit_row = text_row(
            connection_text(app.desktop_locale, "最大输出（token）"),
            connection_text(app.desktop_locale, "最大输出窗口；0 可表示该模型不产生文本输出。"),
            connection_text(app.desktop_locale, "例如 4096，留空使用默认值"),
            &mm.output_limit,
            |v| {
                Message::Settings(
                    message::SettingsMessage::CustomProviderModelModalOutputLimitChanged(v),
                )
            },
        );

        let reasoning = capability_checkbox(
            effective_capability(
                mm.reasoning,
                mm.inherited_capabilities.as_ref(),
                |capabilities| capabilities.reasoning,
                false,
            ),
            connection_text(app.desktop_locale, "推理"),
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
            connection_text(app.desktop_locale, "工具调用"),
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
            connection_text(app.desktop_locale, "温度"),
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
            connection_text(app.desktop_locale, "附件"),
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
            connection_text(app.desktop_locale, "交错输入输出"),
            |value| {
                Message::Settings(
                    message::SettingsMessage::CustomProviderModelModalInterleavedChanged(value),
                )
            },
        );
        let capabilities_row = field_row(
            connection_text(app.desktop_locale, "能力"),
            connection_text(
                app.desktop_locale,
                "未显式设置时沿用基线；新模型默认支持文本和工具调用。",
            ),
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
                    crate::app::i18n::settings_models::modality_label(
                        app.desktop_locale,
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
                    crate::app::i18n::settings_models::modality_label(
                        app.desktop_locale,
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
            field_row(
                connection_text(app.desktop_locale, "输入模态"),
                connection_text(
                    app.desktop_locale,
                    "选择 provider 接受的输入类型；留空时按文本模型默认值处理。"
                ),
                row(input_modality_checks).spacing(12).align_y(Alignment::Center).wrap(),
            ),
            settings_divider(),
            field_row(
                connection_text(app.desktop_locale, "输出模态"),
                connection_text(
                    app.desktop_locale,
                    "选择 provider 返回的输出类型；留空时按文本模型默认值处理。"
                ),
                row(output_modality_checks).spacing(12).align_y(Alignment::Center).wrap(),
            ),
        ]
        .spacing(0);

        let info_tab = column![
            settings_group_heading(
                connection_text(app.desktop_locale, "模型信息"),
                connection_text(
                    app.desktop_locale,
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
            settings_group_heading(
                connection_text(app.desktop_locale, "限制"),
                connection_text(
                    app.desktop_locale,
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
            settings_group_heading(
                connection_text(app.desktop_locale, "能力与模态"),
                connection_text(
                    app.desktop_locale,
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
                connection_text(app.desktop_locale, "模型信息"),
                CustomProviderModelTab::Info,
                mm.active_tab,
            ),
            model_tab_button(
                connection_text(app.desktop_locale, "限制"),
                CustomProviderModelTab::Limits,
                mm.active_tab,
            ),
            model_tab_button(
                connection_text(app.desktop_locale, "能力与模态"),
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
                custom_modal_heading(
                    title,
                    connection_text(app.desktop_locale, "填写模型 ID 和展示名称。"),
                ),
                close_btn,
            ),
            tabs,
        ]
        .spacing(12);
        if let Some(error) = &s.save_error {
            header = header.push(settings_error_banner(error.display(app.desktop_locale)));
        }

        let actions = row![
            Space::new().width(Length::Fill),
            button(text(connection_text(app.desktop_locale, "取消")))
                .on_press(close_message.clone())
                .padding([8, 14])
                .style(rounded_action_btn_style),
            button(text(connection_text(app.desktop_locale, "保存")))
                .on_press(Message::Settings(message::SettingsMessage::CustomProviderModelModalSave))
                .padding([8, 14])
                .style(primary_action_btn_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let card = settings_modal_layout(header, body, Some(actions.into()))
            .style(custom_modal_card_style)
            .width(840)
            .height(680);
        base = settings_modal_overlay(Some(base), close_message, container(card).padding(12));
    }

    base
}

#[cfg(test)]
#[path = "custom_model_modal_ui_refresh_09_tests.rs"]
mod ui_refresh_09_tests;
