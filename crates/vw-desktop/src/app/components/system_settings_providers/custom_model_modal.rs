//! 系统设置中模型提供商配置的目录、表单与弹窗能力。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, SETTINGS_LABEL_WIDTH,
    primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_close_button, settings_divider, settings_error_banner, settings_modal_card,
    settings_modal_overlay, settings_muted_text_style, settings_page_intro, settings_section_card,
    settings_section_content, settings_segment_button_style, settings_text_input_style,
};
use crate::app::i18n::settings_models::connection_text::{
    Text as ConnectionText, text as connection_text,
};
use crate::app::state::CustomProviderModelTab;
use crate::app::{App, Message, message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};

const SCROLLBAR_WIDTH: f32 = 4.0;
const SCROLLBAR_GUTTER: u16 = 3;
// 两侧同时预留滚动条宽度与内容间隔，避免内容贴住 4px 滚动条。
const SCROLLABLE_SIDE_PADDING: u16 = SCROLLBAR_WIDTH as u16 + SCROLLBAR_GUTTER;

const MODALITIES: [&str; 5] = ["text", "image", "audio", "video", "pdf"];

pub(super) fn field_row<'a>(
    label: &'a str,
    description: &'a str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Start),
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
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        text_input(placeholder, value)
            .on_input(on_input)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .width(Length::Fill),
    )
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
    let is_active = tab == active_tab;
    button(text(label).size(13))
        .padding([8, 14])
        .on_press(Message::Settings(message::SettingsMessage::CustomProviderModelTabSelected(tab)))
        .style(move |theme, status| settings_segment_button_style(theme, status, is_active))
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
            connection_text(app.desktop_locale, ConnectionText::EditModel)
        } else {
            connection_text(app.desktop_locale, ConnectionText::AddModel)
        };

        let id_row = text_row(
            connection_text(app.desktop_locale, ConnectionText::ModelId),
            connection_text(app.desktop_locale, ConnectionText::ActualModelIdSentTo),
            connection_text(app.desktop_locale, ConnectionText::ExampleGpt4oMini),
            &mm.model_id,
            |v| Message::Settings(message::SettingsMessage::CustomProviderModelModalIdChanged(v)),
        );

        let name_row = text_row(
            connection_text(app.desktop_locale, ConnectionText::DisplayName),
            connection_text(app.desktop_locale, ConnectionText::DisplayOnlyLeaveEmptyTo),
            connection_text(app.desktop_locale, ConnectionText::Optional),
            &mm.display_name,
            |v| Message::Settings(message::SettingsMessage::CustomProviderModelModalNameChanged(v)),
        );

        let family_row = text_row(
            connection_text(app.desktop_locale, ConnectionText::ModelFamily),
            connection_text(
                app.desktop_locale,
                ConnectionText::OptionalFamilyMetadataDistinguishingModel,
            ),
            connection_text(app.desktop_locale, ConnectionText::ExampleQwenGptClaude),
            &mm.family,
            |v| {
                Message::Settings(message::SettingsMessage::CustomProviderModelModalFamilyChanged(
                    v,
                ))
            },
        );

        let context_limit_row = text_row(
            connection_text(app.desktop_locale, ConnectionText::ContextLimitTokens),
            connection_text(app.desktop_locale, ConnectionText::TotalContextWindowLeaveEmpty),
            connection_text(app.desktop_locale, ConnectionText::Example200000LeaveEmptyFor),
            &mm.context_limit,
            |v| {
                Message::Settings(
                    message::SettingsMessage::CustomProviderModelModalContextLimitChanged(v),
                )
            },
        );
        let input_limit_row = text_row(
            connection_text(app.desktop_locale, ConnectionText::MaximumInputTokens),
            connection_text(app.desktop_locale, ConnectionText::MaximumInputWindowLeaveEmpty),
            connection_text(app.desktop_locale, ConnectionText::Example200000LeaveEmptyFor),
            &mm.input_limit,
            |v| {
                Message::Settings(
                    message::SettingsMessage::CustomProviderModelModalInputLimitChanged(v),
                )
            },
        );
        let output_limit_row = text_row(
            connection_text(app.desktop_locale, ConnectionText::MaximumOutputTokens),
            connection_text(app.desktop_locale, ConnectionText::MaximumOutputWindowZeroCan),
            connection_text(app.desktop_locale, ConnectionText::Example4096LeaveEmptyFor),
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
            connection_text(app.desktop_locale, ConnectionText::Reasoning),
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
            connection_text(app.desktop_locale, ConnectionText::ToolCalls),
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
            connection_text(app.desktop_locale, ConnectionText::Temperature),
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
            connection_text(app.desktop_locale, ConnectionText::Attachments),
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
            connection_text(app.desktop_locale, ConnectionText::InterleavedInputOutput),
            |value| {
                Message::Settings(
                    message::SettingsMessage::CustomProviderModelModalInterleavedChanged(value),
                )
            },
        );
        let capabilities_row = field_row(
            connection_text(app.desktop_locale, ConnectionText::Capabilities),
            connection_text(app.desktop_locale, ConnectionText::UnsetValuesUseTheBaseline),
            row![reasoning, tool_call, temperature, attachment, interleaved].spacing(16),
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
                connection_text(app.desktop_locale, ConnectionText::InputModalities),
                connection_text(app.desktop_locale, ConnectionText::ChooseProviderInputTypesLeave),
                row(input_modality_checks).spacing(12),
            ),
            settings_divider(),
            field_row(
                connection_text(app.desktop_locale, ConnectionText::OutputModalities),
                connection_text(app.desktop_locale, ConnectionText::ChooseProviderOutputTypesLeave),
                row(output_modality_checks).spacing(12),
            ),
        ]
        .spacing(0);

        let info_tab = column![
            settings_section_card(
                connection_text(app.desktop_locale, ConnectionText::ModelInformation),
                connection_text(app.desktop_locale, ConnectionText::ModelIdIsSentIn)
            ),
            settings_section_content(
                column![id_row, settings_divider(), name_row, settings_divider(), family_row]
                    .spacing(0),
            ),
        ]
        .spacing(16);
        let limits_tab = column![
            settings_section_card(
                connection_text(app.desktop_locale, ConnectionText::Limits),
                connection_text(app.desktop_locale, ConnectionText::EmptyValuesUseTheBuilt)
            ),
            settings_section_content(
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
            settings_section_card(
                connection_text(app.desktop_locale, ConnectionText::CapabilitiesAndModalities),
                connection_text(
                    app.desktop_locale,
                    ConnectionText::SaveOnlyExplicitOverridesPreserving
                )
            ),
            settings_section_content(
                column![capabilities_row, settings_divider(), modalities_row].spacing(0)
            ),
        ]
        .spacing(16);

        let tabs = row![
            model_tab_button(
                connection_text(app.desktop_locale, ConnectionText::ModelInformation),
                CustomProviderModelTab::Info,
                mm.active_tab
            ),
            model_tab_button(
                connection_text(app.desktop_locale, ConnectionText::Limits),
                CustomProviderModelTab::Limits,
                mm.active_tab
            ),
            model_tab_button(
                connection_text(app.desktop_locale, ConnectionText::CapabilitiesAndModalities),
                CustomProviderModelTab::Capabilities,
                mm.active_tab,
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = match mm.active_tab {
            CustomProviderModelTab::Info => info_tab.into(),
            CustomProviderModelTab::Limits => limits_tab.into(),
            CustomProviderModelTab::Capabilities => capabilities_tab.into(),
        };

        let error_banner: Element<'_, Message> = s
            .save_error
            .as_ref()
            .map(|error| settings_error_banner(error.display(app.desktop_locale)))
            .unwrap_or_else(|| container(text("")).height(0).into());

        let modal_col = column![
            row![
                container(settings_page_intro(
                    title,
                    connection_text(app.desktop_locale, ConnectionText::EnterTheModelIdAnd)
                ))
                .width(Length::Fill),
                close_btn,
            ]
            .align_y(Alignment::Start),
            tabs,
            error_banner,
            scrollable(container(body).padding([0, SCROLLABLE_SIDE_PADDING]).width(Length::Fill),)
                .direction(Direction::Vertical(
                    Scrollbar::new().width(SCROLLBAR_WIDTH).scroller_width(SCROLLBAR_WIDTH),
                ),)
                .height(Length::Fill),
            row![
                container(text("")).width(Length::Fill),
                button(text(connection_text(app.desktop_locale, ConnectionText::Cancel)))
                    .on_press(close_message.clone())
                    .padding([8, 14])
                    .style(rounded_action_btn_style),
                button(text(connection_text(app.desktop_locale, ConnectionText::Save)))
                    .on_press(Message::Settings(
                        message::SettingsMessage::CustomProviderModelModalSave
                    ))
                    .padding([8, 14])
                    .style(primary_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(16)
        .height(Length::Fill);

        let card =
            settings_modal_card(modal_col).width(Length::Fixed(840.0)).height(Length::Fixed(680.0));
        base = settings_modal_overlay(Some(base), close_message, card);
    }

    base
}
