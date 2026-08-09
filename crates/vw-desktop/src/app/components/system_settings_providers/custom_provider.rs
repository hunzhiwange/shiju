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

use crate::app::assets::Icon;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, SETTINGS_LABEL_WIDTH, icon_btn,
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_divider,
    settings_error_banner, settings_modal_card, settings_modal_overlay, settings_muted_text_style,
    settings_page_intro, settings_panel, settings_section_card, settings_segment_button_style,
    settings_text_input_style, settings_value_badge,
};
use crate::app::{App, Message, message};
use crate::app::state::{CustomProviderModelFilter, CustomProviderTab};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text, text_input, toggler};
use iced::{Alignment, Element, Length};

const SCROLLBAR_WIDTH: f32 = 4.0;
const SCROLLBAR_GUTTER: u16 = 3;
// 两侧同时预留滚动条宽度与内容间隔，避免内容贴住 4px 滚动条。
const SCROLLABLE_SIDE_PADDING: u16 = SCROLLBAR_WIDTH as u16 + SCROLLBAR_GUTTER;

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
        .padding([8, 14])
        .on_press(Message::Settings(message::SettingsMessage::CustomProviderTabSelected(tab)))
        .style(move |theme, status| settings_segment_button_style(theme, status, is_active))
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
        .on_press(Message::Settings(
            message::SettingsMessage::CustomProviderModelFilterSelected(filter),
        ))
        .style(move |theme, status| settings_segment_button_style(theme, status, is_active))
        .into()
}

fn model_matches_filter(
    enabled: bool,
    filter: CustomProviderModelFilter,
) -> bool {
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

        let header_row = row![
            container(settings_page_intro(
                if is_editing { "编辑提供商" } else { "自定义提供商" },
                if is_editing {
                    "调整现有 provider 的连接地址、默认鉴权信息和模型列表。"
                } else {
                    "按 base URL 接入一个与 OpenAI 兼容的 provider，并配置默认模型。"
                },
            ))
            .width(Length::Fill),
            close_btn,
        ]
        .align_y(Alignment::Start);

        let provider_id_row = if is_editing {
            field_row(
                "提供商 ID",
                "作为配置中的稳定标识，编辑模式下保持只读。",
                container(settings_value_badge(s.custom.provider_id.as_str())).width(Length::Fill),
            )
        } else {
            text_row(
                "提供商 ID",
                "作为配置中的稳定标识，只允许小写字母、数字、连字符或下划线。",
                "myprovider",
                &s.custom.provider_id,
                |v| Message::Settings(message::SettingsMessage::CustomProviderIdChanged(v)),
            )
        };

        let display_name_row = text_row(
            "显示名称",
            "展示给用户的 provider 名称。",
            "我是 AI 提供商",
            &s.custom.display_name,
            |v| Message::Settings(message::SettingsMessage::CustomProviderNameChanged(v)),
        );

        let base_url_row = text_row(
            "基础 URL",
            "OpenAI 兼容 API 根地址，例如 https://api.example.com/v1。",
            "https://api.example.com/v1",
            &s.custom.base_url,
            |v| Message::Settings(message::SettingsMessage::CustomProviderBaseUrlChanged(v)),
        );

        let api_key_row = secure_text_row(
            "API Key",
            "可选默认密钥；填写后会作为 Authorization 头使用。",
            "sk-...",
            &s.custom.api_key,
            |v| Message::Settings(message::SettingsMessage::CustomProviderApiKeyChanged(v)),
        );

        let mut headers_controls = column![
            row![
                text(format!("已配置 {} 项", s.custom.headers.len()))
                    .size(12)
                    .style(settings_muted_text_style),
                container(text(" ")).width(Length::Fill),
                icon_btn(
                    Icon::Plus,
                    "添加请求头",
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
                    text_input("请求头名称", &h.key)
                        .on_input(move |v| Message::Settings(
                            message::SettingsMessage::CustomProviderHeaderKeyChanged(i, v)
                        ))
                        .padding(SETTINGS_CONTROL_PADDING)
                        .size(SETTINGS_CONTROL_TEXT_SIZE)
                        .style(settings_text_input_style)
                        .width(Length::Fill),
                    text_input("请求头值", &h.value)
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
                            "移除",
                            Some(Message::Settings(
                                message::SettingsMessage::CustomProviderHeaderRemove(i),
                            )),
                        )
                    } else {
                        icon_btn(Icon::Trash, "移除", None)
                    },
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }

        let headers_row = field_row(
            "请求头",
            "为特殊鉴权场景补充额外 header，例如代理网关或自定义令牌头。",
            headers_controls,
        );

        let enabled_count =
            s.custom.models.iter().filter(|model| model.enabled && !model.model_id.trim().is_empty()).count();
        let total_count =
            s.custom.models.iter().filter(|model| !model.model_id.trim().is_empty()).count();
        let mut model_actions = row![
            settings_value_badge(format!("{enabled_count}/{total_count} 已启用")),
            container(text(" ")).width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        if is_editing {
            model_actions = model_actions.push(
                button(text(if s.custom_models_discovering {
                    "发现中…"
                } else {
                    "自动发现模型"
                }))
                .on_press_maybe((!s.custom_models_discovering).then_some(Message::Settings(
                    message::SettingsMessage::CustomProviderModelsDiscover,
                )))
                .padding([7, 12])
                .style(if s.custom_models_discovering {
                    rounded_action_btn_style
                } else {
                    primary_action_btn_style
                }),
            );
        }
        model_actions = model_actions.push(icon_btn(
            Icon::Plus,
            "添加模型",
            Some(Message::Settings(message::SettingsMessage::CustomProviderModelOpen(None))),
        ));

        let filter_row = row![
            model_filter_button("全部", CustomProviderModelFilter::All, s.custom_models_filter),
            model_filter_button(
                "已启用",
                CustomProviderModelFilter::Enabled,
                s.custom_models_filter,
            ),
            model_filter_button(
                "已禁用",
                CustomProviderModelFilter::Disabled,
                s.custom_models_filter,
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let mut models_panel = column![
            field_row(
                "模型",
                "维护该 provider 可用的模型列表；可启用/禁用，展示名称可留空。",
                model_actions,
            ),
            settings_divider(),
            field_row("过滤", "按启用状态筛选模型列表。", filter_row),
        ]
        .spacing(0);

        let mut visible_count = 0usize;
        for (i, m) in s.custom.models.iter().enumerate() {
            if !model_matches_filter(m.enabled, s.custom_models_filter) {
                continue;
            }
            visible_count += 1;
            let title = if m.display_name.trim().is_empty() {
                if m.model_id.trim().is_empty() { "未命名模型" } else { m.model_id.as_str() }
            } else {
                m.display_name.as_str()
            };
            let subtitle = if m.model_id.trim().is_empty() {
                "尚未填写 model_id"
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
                            text(title)
                                .size(13)
                                .style(move |theme: &iced::Theme| text::Style {
                                    color: Some(theme.palette().text.scale_alpha(title_alpha)),
                                }),
                            text(subtitle)
                                .size(11)
                                .style(move |theme: &iced::Theme| text::Style {
                                    color: Some(
                                        theme.palette().text.scale_alpha(subtitle_alpha),
                                    ),
                                }),
                        ]
                        .spacing(4)
                        .width(Length::Fill),
                        toggler(enabled).on_toggle(move |value| {
                            Message::Settings(
                                message::SettingsMessage::CustomProviderModelEnabledToggled(
                                    i, value,
                                ),
                            )
                        }),
                        icon_btn(
                            Icon::Pencil,
                            "编辑",
                            Some(Message::Settings(
                                message::SettingsMessage::CustomProviderModelOpen(Some(i))
                            ))
                        ),
                        icon_btn(
                            Icon::Trash,
                            if m.from_catalog { "禁用" } else { "移除" },
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
                        CustomProviderModelFilter::All => "暂无模型",
                        CustomProviderModelFilter::Enabled => "当前没有已启用的模型",
                        CustomProviderModelFilter::Disabled => "当前没有已禁用的模型",
                    })
                    .size(12)
                    .style(settings_muted_text_style),
                )
                .padding([14, 0])
                .width(Length::Fill),
            );
        }

        let basic_tab = column![
            settings_section_card(
                "基础信息",
                "命名 provider，并指定展示名称与 OpenAI 兼容接口地址。",
            ),
            settings_panel(
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
            settings_section_card(
                "认证与请求头",
                "设置默认 API Key，并为需要额外 header 的网关场景补充请求头。",
            ),
            settings_panel(column![api_key_row, settings_divider(), headers_row].spacing(0)),
        ]
        .spacing(16);
        let models_tab = column![
            settings_section_card(
                "模型",
                "维护该 provider 的模型列表，后续聊天与生成界面会复用这里的配置。",
            ),
            settings_panel(models_panel),
        ]
        .spacing(16);

        let tabs = row![
            provider_tab_button("基础信息", CustomProviderTab::Basic, s.custom_provider_tab),
            provider_tab_button(
                "认证与请求头",
                CustomProviderTab::Authentication,
                s.custom_provider_tab,
            ),
            provider_tab_button("模型", CustomProviderTab::Models, s.custom_provider_tab),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let body_col: Element<'_, Message> = match s.custom_provider_tab {
            CustomProviderTab::Basic => basic_tab.into(),
            CustomProviderTab::Authentication => authentication_tab.into(),
            CustomProviderTab::Models => models_tab.into(),
        };

        let footer_row =
            row![
                button(text("取消"))
                    .on_press(close_message.clone())
                    .padding([8, 14])
                    .style(rounded_action_btn_style),
                container(text(" ")).width(Length::Fill),
                button(text("保存"))
                    .on_press_maybe((!s.custom_models_discovering).then_some(Message::Settings(
                        message::SettingsMessage::CustomProviderSave,
                    )))
                    .padding([8, 14])
                    .style(primary_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        let scroll_body = scrollable(
            container(body_col).padding([0, SCROLLABLE_SIDE_PADDING]).width(Length::Fill),
        )
            .direction(
                Direction::Vertical(
                    Scrollbar::new().width(SCROLLBAR_WIDTH).scroller_width(SCROLLBAR_WIDTH),
                ),
            )
            .height(Length::Fill);

        let mut modal_col = column![header_row, tabs].spacing(16).height(Length::Fill);
        if let Some(err) = &s.save_error {
            modal_col = modal_col.push(settings_error_banner(err));
        }
        modal_col = modal_col.push(scroll_body).push(footer_row);

        let card =
            settings_modal_card(modal_col).width(Length::Fixed(780.0)).height(Length::Fixed(620.0));

        base = settings_modal_overlay(Some(base), close_message, card);
    }

    base
}
