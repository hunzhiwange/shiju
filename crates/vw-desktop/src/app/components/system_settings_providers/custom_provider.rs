//! 自定义提供商配置模块
//!
//! 本模块提供自定义 AI 提供商配置界面的视图组件。
//! 允许用户添加与 OpenAI 兼容的第三方提供商，
//! 支持配置提供商 ID、显示名称、基础 URL、API 密钥、自定义请求头和模型列表。
//!
//! # 主要功能
//!
//! - 渲染自定义提供商配置模态对话框
//! - 支持动态添加/移除自定义请求头
//! - 支持动态添加/移除模型配置
//! - 提供表单验证和错误提示

use super::custom_model_modal::{
    custom_modal_card_style, custom_modal_heading, custom_segment_style,
};
pub(super) use super::custom_model_modal::{field_row, text_row};
use crate::app::assets::Icon;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, icon_btn, primary_action_btn_style,
    rounded_action_btn_style, settings_close_button, settings_divider, settings_error_banner,
    settings_group_card, settings_group_heading, settings_modal_header, settings_modal_layout,
    settings_modal_overlay, settings_muted_text_style, settings_text_input_style, settings_toggle,
    settings_value_badge,
};
use crate::app::i18n::settings_models::connection_text::text as connection_text;
use crate::app::i18n::settings_models::messages::render as owned_text;
use crate::app::state::{CustomProviderModelFilter, CustomProviderTab};
use crate::app::{App, Message, message};
use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

pub(super) fn secure_text_row<'a>(
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
            .secure(true)
            .on_input(on_input)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .width(Length::Fill),
    )
}

fn provider_tab_button(
    label: &'static str,
    tab: CustomProviderTab,
    active_tab: CustomProviderTab,
) -> Element<'static, Message> {
    let is_active = tab == active_tab;
    button(text(label).size(13))
        .padding([8, 12])
        .on_press(Message::Settings(message::SettingsMessage::CustomProviderTabSelected(tab)))
        .style(move |theme, status| custom_segment_style(theme, status, is_active))
        .into()
}

fn model_filter_button(
    label: &'static str,
    filter: CustomProviderModelFilter,
    active: CustomProviderModelFilter,
) -> Element<'static, Message> {
    let is_active = filter == active;
    button(text(label).size(12))
        .padding([6, 10])
        .on_press(Message::Settings(message::SettingsMessage::CustomProviderModelFilterSelected(
            filter,
        )))
        .style(move |theme, status| custom_segment_style(theme, status, is_active))
        .into()
}

fn model_matches_filter(enabled: bool, filter: CustomProviderModelFilter) -> bool {
    match filter {
        CustomProviderModelFilter::All => true,
        CustomProviderModelFilter::Enabled => enabled,
        CustomProviderModelFilter::Disabled => !enabled,
    }
}

/// 渲染自定义提供商配置模态对话框的叠加层视图
///
/// 此函数负责构建完整的自定义提供商配置界面，包括模态对话框、
/// 表单字段、请求头列表、模型列表以及操作按钮。
/// 当 `custom_provider_modal_open` 标志为 true 时，会在原有对话框之上
/// 叠加显示配置模态框。
///
/// # 参数
///
/// - `app`: 应用程序状态引用，包含提供商设置和配置数据
/// - `dialog`: 底层对话框元素，模态框将叠加在其上方
///
/// # 返回值
///
/// 返回组合后的 UI 元素，如果模态框未打开则返回原始对话框，
/// 否则返回叠加了配置模态框的完整视图
///
/// # 示例
///
/// ```ignore
/// let base_dialog = text("基础对话框");
/// let overlay = view_overlays(&app, base_dialog);
/// ```
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.provider_settings;
    let mut base = dialog;

    if s.custom_provider_modal_open {
        let is_editing = s.custom_editing_provider_id.is_some();
        let close_message = Message::Settings(message::SettingsMessage::CustomProviderClose);
        let close_btn = settings_close_button(close_message.clone());

        let header_row = settings_modal_header(
            custom_modal_heading(
                if is_editing {
                    connection_text(app.desktop_locale, "编辑提供商")
                } else {
                    connection_text(app.desktop_locale, "自定义提供商")
                },
                if is_editing {
                    connection_text(
                        app.desktop_locale,
                        "调整现有 provider 的连接地址、默认鉴权信息和模型列表。",
                    )
                } else {
                    connection_text(
                        app.desktop_locale,
                        "按 base URL 接入一个与 OpenAI 兼容的 provider；模型可稍后配置。",
                    )
                },
            ),
            close_btn,
        );

        let provider_id_row = if is_editing {
            field_row(
                connection_text(app.desktop_locale, "提供商 ID"),
                connection_text(app.desktop_locale, "作为配置中的稳定标识，编辑模式下保持只读。"),
                container(settings_value_badge(s.custom.provider_id.as_str())).width(Length::Fill),
            )
        } else {
            text_row(
                connection_text(app.desktop_locale, "提供商 ID"),
                connection_text(
                    app.desktop_locale,
                    "作为配置中的稳定标识，只允许小写字母、数字、连字符或下划线。",
                ),
                "myprovider",
                &s.custom.provider_id,
                |v| Message::Settings(message::SettingsMessage::CustomProviderIdChanged(v)),
            )
        };

        let display_name_row = text_row(
            connection_text(app.desktop_locale, "显示名称"),
            connection_text(app.desktop_locale, "展示给用户的 provider 名称。"),
            connection_text(app.desktop_locale, "我是 AI 提供商"),
            &s.custom.display_name,
            |v| Message::Settings(message::SettingsMessage::CustomProviderNameChanged(v)),
        );

        let base_url_row = text_row(
            connection_text(app.desktop_locale, "基础 URL"),
            connection_text(
                app.desktop_locale,
                "OpenAI 兼容 API 根地址，例如 https://api.example.com/v1。",
            ),
            "https://api.example.com/v1",
            &s.custom.base_url,
            |v| Message::Settings(message::SettingsMessage::CustomProviderBaseUrlChanged(v)),
        );

        let api_key_row = if crate::app::state::is_shiju_official_provider_id(&s.custom.provider_id)
        {
            field_row(
                connection_text(app.desktop_locale, "账户自动授权"),
                connection_text(
                    app.desktop_locale,
                    "登录识矩账户后自动配置模型，无需填写模型密钥。",
                ),
                text(connection_text(app.desktop_locale, "密钥由账户管理，不显示明文"))
                    .style(settings_muted_text_style),
            )
        } else {
            secure_text_row(
                "API Key",
                connection_text(
                    app.desktop_locale,
                    "可选默认密钥；填写后会作为 Authorization 头使用。",
                ),
                "sk-...",
                &s.custom.api_key,
                |v| Message::Settings(message::SettingsMessage::CustomProviderApiKeyChanged(v)),
            )
        };

        let mut headers_controls = column![
            row![
                text(owned_text(
                    app.desktop_locale,
                    "已配置 %s 项",
                    &[s.custom.headers.len().to_string()]
                ))
                .size(12)
                .style(settings_muted_text_style),
                Space::new().width(Length::Fill),
                icon_btn(
                    Icon::Plus,
                    connection_text(app.desktop_locale, "添加请求头"),
                    Some(Message::Settings(message::SettingsMessage::CustomProviderHeaderAdd))
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(10)
        .width(Length::Fill);

        for (i, h) in s.custom.headers.iter().enumerate() {
            headers_controls = headers_controls.push(
                row![
                    text_input(connection_text(app.desktop_locale, "请求头名称"), &h.key)
                        .on_input(move |v| Message::Settings(
                            message::SettingsMessage::CustomProviderHeaderKeyChanged(i, v)
                        ))
                        .padding(SETTINGS_CONTROL_PADDING)
                        .size(SETTINGS_CONTROL_TEXT_SIZE)
                        .style(settings_text_input_style)
                        .width(Length::Fill),
                    text_input(connection_text(app.desktop_locale, "请求头值"), &h.value)
                        .on_input(move |v| Message::Settings(
                            message::SettingsMessage::CustomProviderHeaderValueChanged(i, v)
                        ))
                        .padding(SETTINGS_CONTROL_PADDING)
                        .size(SETTINGS_CONTROL_TEXT_SIZE)
                        .style(settings_text_input_style)
                        .width(Length::Fill),
                    if s.custom.headers.len() > 1 {
                        icon_btn(
                            Icon::Trash,
                            connection_text(app.desktop_locale, "移除"),
                            Some(Message::Settings(
                                message::SettingsMessage::CustomProviderHeaderRemove(i),
                            )),
                        )
                    } else {
                        icon_btn(Icon::Trash, connection_text(app.desktop_locale, "移除"), None)
                    },
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }

        let headers_row = field_row(
            connection_text(app.desktop_locale, "请求头"),
            connection_text(
                app.desktop_locale,
                "为特殊鉴权场景补充额外 header，例如代理网关或自定义令牌头。",
            ),
            headers_controls,
        );

        let enabled_count = s
            .custom
            .models
            .iter()
            .filter(|model| model.enabled && !model.model_id.trim().is_empty())
            .count();
        let total_count =
            s.custom.models.iter().filter(|model| !model.model_id.trim().is_empty()).count();
        let model_actions = row![
            settings_value_badge(crate::app::i18n::settings_models::enabled_count(
                app.desktop_locale,
                enabled_count,
                total_count
            )),
            button(text(if s.custom_models_discovering {
                connection_text(app.desktop_locale, "发现中…")
            } else {
                connection_text(app.desktop_locale, "自动发现模型")
            }))
            .on_press_maybe((!s.custom_models_discovering).then_some(Message::Settings(
                message::SettingsMessage::CustomProviderModelsDiscover,
            )))
            .padding([7, 12])
            .style(primary_action_btn_style),
            icon_btn(
                Icon::Plus,
                connection_text(app.desktop_locale, "添加模型"),
                Some(Message::Settings(message::SettingsMessage::CustomProviderModelOpen(None))),
            ),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .wrap();

        let filter_row = row![
            model_filter_button(
                connection_text(app.desktop_locale, "全部"),
                CustomProviderModelFilter::All,
                s.custom_models_filter
            ),
            model_filter_button(
                connection_text(app.desktop_locale, "已启用"),
                CustomProviderModelFilter::Enabled,
                s.custom_models_filter,
            ),
            model_filter_button(
                connection_text(app.desktop_locale, "已禁用"),
                CustomProviderModelFilter::Disabled,
                s.custom_models_filter,
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap();

        let mut models_panel = column![
            field_row(
                connection_text(app.desktop_locale, "模型"),
                connection_text(
                    app.desktop_locale,
                    "可留空，或自动发现该 provider 的模型；也可手动添加并启用/禁用。"
                ),
                model_actions,
            ),
            settings_divider(),
            field_row(
                connection_text(app.desktop_locale, "过滤"),
                connection_text(app.desktop_locale, "按启用状态筛选模型列表。"),
                filter_row
            ),
        ]
        .spacing(0);

        let mut visible_count = 0usize;
        for (i, m) in s.custom.models.iter().enumerate() {
            if !model_matches_filter(m.enabled, s.custom_models_filter) {
                continue;
            }
            visible_count += 1;
            let title = if m.display_name.trim().is_empty() {
                if m.model_id.trim().is_empty() {
                    connection_text(app.desktop_locale, "未命名模型")
                } else {
                    m.model_id.as_str()
                }
            } else {
                m.display_name.as_str()
            };
            let subtitle = if m.model_id.trim().is_empty() {
                connection_text(app.desktop_locale, "尚未填写 model_id")
            } else {
                m.model_id.as_str()
            };
            let title_alpha = if m.enabled { 1.0 } else { 0.65 };
            let subtitle_alpha = if m.enabled { 0.65 } else { 0.5 };
            let enabled = m.enabled;
            let remove_message = if m.from_catalog {
                Message::Settings(message::SettingsMessage::CustomProviderModelEnabledToggled(
                    i, false,
                ))
            } else {
                Message::Settings(message::SettingsMessage::CustomProviderModelRemove(i))
            };

            models_panel = models_panel.push(settings_divider());
            models_panel = models_panel.push(
                container(
                    row![
                        column![
                            text(title).size(14).wrapping(Wrapping::WordOrGlyph).style(
                                move |theme: &iced::Theme| text::Style {
                                    color: Some(theme.palette().text.scale_alpha(title_alpha)),
                                }
                            ),
                            text(subtitle).size(12).wrapping(Wrapping::WordOrGlyph).style(
                                move |theme: &iced::Theme| text::Style {
                                    color: Some(theme.palette().text.scale_alpha(subtitle_alpha),),
                                }
                            ),
                        ]
                        .spacing(4)
                        .width(Length::Fill),
                        settings_toggle(enabled).on_toggle(move |value| {
                            Message::Settings(
                                message::SettingsMessage::CustomProviderModelEnabledToggled(
                                    i, value,
                                ),
                            )
                        }),
                        icon_btn(
                            Icon::Pencil,
                            connection_text(app.desktop_locale, "编辑"),
                            Some(Message::Settings(
                                message::SettingsMessage::CustomProviderModelOpen(Some(i))
                            ))
                        ),
                        icon_btn(
                            Icon::Trash,
                            if m.from_catalog {
                                connection_text(app.desktop_locale, "禁用")
                            } else {
                                connection_text(app.desktop_locale, "移除")
                            },
                            Some(remove_message),
                        ),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .padding([14, 0])
                .width(Length::Fill),
            );
        }
        if visible_count == 0 {
            models_panel = models_panel.push(settings_divider());
            models_panel = models_panel.push(
                container(
                    text(match s.custom_models_filter {
                        CustomProviderModelFilter::All => {
                            connection_text(app.desktop_locale, "暂无模型")
                        }
                        CustomProviderModelFilter::Enabled => {
                            connection_text(app.desktop_locale, "当前没有已启用的模型")
                        }
                        CustomProviderModelFilter::Disabled => {
                            connection_text(app.desktop_locale, "当前没有已禁用的模型")
                        }
                    })
                    .size(12)
                    .style(settings_muted_text_style),
                )
                .padding([14, 0])
                .width(Length::Fill),
            );
        }

        let basic_tab = column![
            settings_group_heading(
                connection_text(app.desktop_locale, "基础信息"),
                connection_text(
                    app.desktop_locale,
                    "命名 provider，并指定展示名称与 OpenAI 兼容接口地址。"
                ),
            ),
            settings_group_card(
                column![
                    provider_id_row,
                    settings_divider(),
                    display_name_row,
                    settings_divider(),
                    base_url_row
                ]
                .spacing(0),
            ),
        ]
        .spacing(16);
        let authentication_tab = column![
            settings_group_heading(
                connection_text(app.desktop_locale, "认证与请求头"),
                connection_text(
                    app.desktop_locale,
                    "设置默认 API Key，并为需要额外 header 的网关场景补充请求头。"
                ),
            ),
            settings_group_card(column![api_key_row, settings_divider(), headers_row].spacing(0)),
        ]
        .spacing(16);
        let models_tab = column![
            settings_group_heading(
                connection_text(app.desktop_locale, "模型"),
                connection_text(
                    app.desktop_locale,
                    "维护该 provider 的模型列表，后续聊天与生成界面会复用这里的配置。"
                ),
            ),
            settings_group_card(models_panel),
        ]
        .spacing(16);

        let tabs = row![
            provider_tab_button(
                connection_text(app.desktop_locale, "基础信息"),
                CustomProviderTab::Basic,
                s.custom_provider_tab
            ),
            provider_tab_button(
                connection_text(app.desktop_locale, "认证与请求头"),
                CustomProviderTab::Authentication,
                s.custom_provider_tab,
            ),
            provider_tab_button(
                connection_text(app.desktop_locale, "模型"),
                CustomProviderTab::Models,
                s.custom_provider_tab
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap();

        let body_col: Element<'_, Message> = match s.custom_provider_tab {
            CustomProviderTab::Basic => basic_tab.into(),
            CustomProviderTab::Authentication => authentication_tab.into(),
            CustomProviderTab::Models => models_tab.into(),
        };

        let footer_row =
            row![
                Space::new().width(Length::Fill),
                button(text(connection_text(app.desktop_locale, "取消")))
                    .on_press(close_message.clone())
                    .padding([8, 14])
                    .style(rounded_action_btn_style),
                button(text(connection_text(app.desktop_locale, "保存")))
                    .on_press_maybe((!s.custom_models_discovering).then_some(Message::Settings(
                        message::SettingsMessage::CustomProviderSave,
                    )))
                    .padding([8, 14])
                    .style(primary_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        let mut header = column![header_row, tabs].spacing(12);
        if let Some(error) = &s.save_error {
            header = header.push(settings_error_banner(error.display(app.desktop_locale)));
        }

        let card = settings_modal_layout(header, body_col, Some(footer_row.into()))
            .style(custom_modal_card_style)
            .width(780)
            .height(620);
        base = settings_modal_overlay(Some(base), close_message, container(card).padding(12));
    }

    base
}
