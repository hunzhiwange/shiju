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

use crate::view_style::ViewStyle;
use super::custom_model_modal::{
    custom_modal_card_style, custom_segment_style, responsive_field_with_help, stacked_field_row,
};
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{
    icon_btn, primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_control_with_hint, settings_divider, settings_error_banner, settings_field_label,
    settings_group_card, settings_modal_header, settings_modal_layout,
    settings_muted_text_style, settings_tab_button, settings_tab_row, settings_text_input,
    settings_toggle, settings_value_badge,
};
use desktop_i18n::settings_models::connection_text::text as connection_text;
use desktop_i18n::settings_models::messages::render as owned_text;
use crate::provider_state::{CustomProviderModelFilter, CustomProviderTab};
use crate::view_message::{Message};
use crate::view_message as message;
use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

fn provider_tab_button(
    label: &'static str,
    tab: CustomProviderTab,
    active_tab: CustomProviderTab,
) -> Element<'static, Message> {
    settings_tab_button(
        label,
        tab == active_tab,
        Some(Message::Settings(message::SettingsMessage::CustomProviderTabSelected(tab))),
    )
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
pub fn modal<'a>(s: &'a crate::provider_state::ProviderSettingsState, style: ViewStyle<'a>) -> Option<(Message, Element<'a, Message>)> {
    if s.custom_provider_modal_open {
        let is_editing = s.custom_editing_provider_id.is_some();
        let close_message = Message::Settings(message::SettingsMessage::CustomProviderClose);
        let close_btn = settings_close_button(close_message.clone());

        let locale = style.locale;
        let title = connection_text(locale, if is_editing { "编辑提供商" } else { "自定义提供商" });
        let description = connection_text(
            locale,
            if is_editing {
                "调整现有 provider 的连接地址、默认鉴权信息和模型列表。"
            } else {
                "按 base URL 接入一个与 OpenAI 兼容的 provider；模型可稍后配置。"
            },
        );
        let header_row = settings_modal_header(
            style.help_label(title, text(title).size(20), description).wrap(),
            close_btn,
        );

        let field_label = |label, description| {
            style.help_label(label, settings_field_label(label), description)
                .wrap()
        };

        let provider_id_label = connection_text(locale, "提供商 ID");
        let provider_id_row = if is_editing {
            responsive_field_with_help(
                style,
                provider_id_label,
                connection_text(locale, "作为配置中的稳定标识，编辑模式下保持只读。"),
                move || {
                    container(settings_value_badge(s.custom.provider_id.as_str()))
                        .width(Length::Fill)
                },
            )
        } else {
            let description = connection_text(
                locale,
                "作为配置中的稳定标识，只允许小写字母、数字、连字符或下划线。",
            );
            let example = connection_text(locale, "例如 myprovider。");
            responsive_field_with_help(
                style,
                provider_id_label,
                format!("{description} {example}"),
                move || {
                    settings_control_with_hint(
                        settings_text_input("", &s.custom.provider_id)
                            .on_input(|value| {
                                Message::Settings(
                                    message::SettingsMessage::CustomProviderIdChanged(value),
                                )
                            })
                            .width(Length::Fill),
                        connection_text(locale, "小写字母、数字、下划线或连字符。"),
                    )
                },
            )
        };

        let display_name_row = responsive_field_with_help(
            style,
            connection_text(locale, "显示名称"),
            connection_text(locale, "展示给用户的 provider 名称。"),
            move || {
                settings_text_input(
                    connection_text(locale, "我是 AI 提供商"),
                    &s.custom.display_name,
                )
                .on_input(|value| {
                    Message::Settings(message::SettingsMessage::CustomProviderNameChanged(value))
                })
                .width(Length::Fill)
            },
        );

        let base_url_row = responsive_field_with_help(
            style,
            connection_text(locale, "基础 URL"),
            connection_text(locale, "OpenAI 兼容 API 根地址，例如 https://api.example.com/v1。"),
            || {
                settings_text_input("https://api.example.com/v1", &s.custom.base_url)
                    .on_input(|value| {
                        Message::Settings(message::SettingsMessage::CustomProviderBaseUrlChanged(
                            value,
                        ))
                    })
                    .width(Length::Fill)
            },
        );

        let api_key_row = if crate::provider_state::is_shiju_official_provider_id(&s.custom.provider_id)
        {
            responsive_field_with_help(
                style,
                connection_text(style.locale, "账户自动授权"),
                connection_text(
                    style.locale,
                    "登录识矩账户后自动配置模型，无需填写模型密钥。",
                ),
                move || {
                    text(connection_text(style.locale, "密钥由账户管理，不显示明文"))
                        .style(settings_muted_text_style)
                },
            )
        } else {
            responsive_field_with_help(
                style,
                "API Key",
                connection_text(locale, "可选默认密钥；填写后会作为 Authorization 头使用。"),
                move || {
                    column![
                        settings_text_input("sk-...", &s.custom.api_key)
                            .secure(true)
                            .on_input(|value| Message::Settings(
                                message::SettingsMessage::CustomProviderApiKeyChanged(value)
                            ))
                            .width(Length::Fill),
                        text(connection_text(locale, "可选"))
                            .size(12)
                            .style(settings_muted_text_style),
                    ]
                    .spacing(6)
                    .width(Length::Fill)
                },
            )
        };

        let mut headers_controls = column![
            row![
                text(owned_text(
                    style.locale,
                    "已配置 %s 项",
                    &[s.custom.headers.len().to_string()]
                ))
                .size(12)
                .style(settings_muted_text_style),
                Space::new().width(Length::Fill),
                icon_btn(
                    Icon::Plus,
                    connection_text(style.locale, "添加请求头"),
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
                    settings_text_input(connection_text(style.locale, "请求头名称"), &h.key)
                        .on_input(move |v| Message::Settings(
                            message::SettingsMessage::CustomProviderHeaderKeyChanged(i, v)
                        ))
                        .width(Length::Fill),
                    settings_text_input(connection_text(style.locale, "请求头值"), &h.value)
                        .on_input(move |v| Message::Settings(
                            message::SettingsMessage::CustomProviderHeaderValueChanged(i, v)
                        ))
                        .width(Length::Fill),
                    if s.custom.headers.len() > 1 {
                        icon_btn(
                            Icon::Trash,
                            connection_text(style.locale, "移除"),
                            Some(Message::Settings(
                                message::SettingsMessage::CustomProviderHeaderRemove(i),
                            )),
                        )
                    } else {
                        icon_btn(Icon::Trash, connection_text(style.locale, "移除"), None)
                    },
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }

        let headers_row = stacked_field_row(
            field_label(
                connection_text(locale, "请求头"),
                connection_text(
                    locale,
                    "为特殊鉴权场景补充额外 header，例如代理网关或自定义令牌头。",
                ),
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
            settings_value_badge(desktop_i18n::settings_models::enabled_count(
                style.locale,
                enabled_count,
                total_count
            )),
            button(text(if s.custom_models_discovering {
                connection_text(style.locale, "发现中…")
            } else {
                connection_text(style.locale, "自动发现模型")
            }))
            .on_press_maybe((!s.custom_models_discovering).then_some(Message::Settings(
                message::SettingsMessage::CustomProviderModelsDiscover,
            )))
            .padding([7, 12])
            .style(primary_action_btn_style),
            icon_btn(
                Icon::Plus,
                connection_text(style.locale, "添加模型"),
                Some(Message::Settings(message::SettingsMessage::CustomProviderModelOpen(None))),
            ),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .wrap();

        let filter_row = row![
            model_filter_button(
                connection_text(style.locale, "全部"),
                CustomProviderModelFilter::All,
                s.custom_models_filter
            ),
            model_filter_button(
                connection_text(style.locale, "已启用"),
                CustomProviderModelFilter::Enabled,
                s.custom_models_filter,
            ),
            model_filter_button(
                connection_text(style.locale, "已禁用"),
                CustomProviderModelFilter::Disabled,
                s.custom_models_filter,
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap();

        let mut models_panel = column![
            stacked_field_row(
                field_label(
                    connection_text(locale, "模型"),
                    connection_text(
                        locale,
                        "可留空，或自动发现该 provider 的模型；也可手动添加并启用/禁用。"
                    ),
                ),
                model_actions,
            ),
            settings_divider(),
            stacked_field_row(
                field_label(
                    connection_text(locale, "过滤"),
                    connection_text(locale, "按启用状态筛选模型列表。"),
                ),
                filter_row,
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
                    connection_text(style.locale, "未命名模型")
                } else {
                    m.model_id.as_str()
                }
            } else {
                m.display_name.as_str()
            };
            let subtitle = if m.model_id.trim().is_empty() {
                connection_text(style.locale, "尚未填写 model_id")
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
                            connection_text(style.locale, "编辑"),
                            Some(Message::Settings(
                                message::SettingsMessage::CustomProviderModelOpen(Some(i))
                            ))
                        ),
                        icon_btn(
                            Icon::Trash,
                            if m.from_catalog {
                                connection_text(style.locale, "禁用")
                            } else {
                                connection_text(style.locale, "移除")
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
                            connection_text(style.locale, "暂无模型")
                        }
                        CustomProviderModelFilter::Enabled => {
                            connection_text(style.locale, "当前没有已启用的模型")
                        }
                        CustomProviderModelFilter::Disabled => {
                            connection_text(style.locale, "当前没有已禁用的模型")
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
            style.group_heading(connection_text(style.locale, "基础信息"), connection_text(
                    style.locale,
                    "命名 provider，并指定展示名称与 OpenAI 兼容接口地址。"
                )),
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
            style.group_heading(connection_text(style.locale, "认证与请求头"), connection_text(
                    style.locale,
                    "设置默认 API Key，并为需要额外 header 的网关场景补充请求头。"
                )),
            settings_group_card(column![api_key_row, settings_divider(), headers_row].spacing(0)),
        ]
        .spacing(16);
        let models_tab = column![
            style.group_heading(connection_text(style.locale, "模型"), connection_text(
                    style.locale,
                    "维护该 provider 的模型列表，后续聊天与生成界面会复用这里的配置。"
                )),
            settings_group_card(models_panel),
        ]
        .spacing(16);

        let tabs = settings_tab_row([
            provider_tab_button(
                connection_text(style.locale, "基础信息"),
                CustomProviderTab::Basic,
                s.custom_provider_tab,
            ),
            provider_tab_button(
                connection_text(style.locale, "认证与请求头"),
                CustomProviderTab::Authentication,
                s.custom_provider_tab,
            ),
            provider_tab_button(
                connection_text(style.locale, "模型"),
                CustomProviderTab::Models,
                s.custom_provider_tab,
            ),
        ])
        .wrap();

        let body_col: Element<'_, Message> = match s.custom_provider_tab {
            CustomProviderTab::Basic => basic_tab.into(),
            CustomProviderTab::Authentication => authentication_tab.into(),
            CustomProviderTab::Models => models_tab.into(),
        };

        let footer_row =
            row![
                button(text(connection_text(style.locale, "取消")))
                    .on_press(close_message.clone())
                    .padding([8, 14])
                    .style(rounded_action_btn_style),
                button(text(connection_text(style.locale, "保存")))
                    .on_press_maybe((!s.custom_models_discovering).then_some(Message::Settings(
                        message::SettingsMessage::CustomProviderSave,
                    )))
                    .padding([8, 14])
                    .style(primary_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .wrap()
            .align_x(iced::alignment::Horizontal::Right);

        let mut header = column![header_row, tabs].spacing(12);
        if let Some(error) = &s.save_error {
            header = header.push(settings_error_banner(error.display(style.locale)));
        }

        let card = settings_modal_layout(header, body_col, Some(footer_row.into()))
            .style(custom_modal_card_style)
            .width(780)
            .height(620);
        return Some((close_message, card.into()));
    }

    None
}

#[cfg(test)]
pub(super) fn view_overlays<'a>(app: &'a crate::provider_view_test_support::Fixture, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some((close, card)) = modal(&app.provider_settings, crate::provider_view_test_support::style(app)) else { return base };
    desktop_ui_controls::system_settings_common::settings_modal_overlay(Some(base), close, card)
}
