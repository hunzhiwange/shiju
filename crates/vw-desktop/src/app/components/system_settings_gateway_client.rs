//! 系统设置中 gateway client 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::number_input::NumberInput;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_divider, settings_group_card, settings_group_heading,
    settings_help_button_with_locale, settings_muted_text_style, settings_page_heading,
    settings_row_with_flexible_control, settings_text_input_style, with_settings_help_modal,
};
use crate::app::i18n::settings_models::connection_text::text as connection_text;
use crate::app::i18n::settings_models::messages::render as owned_text;
use crate::app::message::settings::{GatewayClientMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{Space, button, column, container, row, text, text::Wrapping, text_input};
use iced::{Alignment, Color, Element, Length, Theme};

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_row_with_flexible_control(label, description, 460.0, control)
}

fn text_row<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
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

fn secure_text_row<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
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

fn number_row<'a>(
    label: &'static str,
    description: &'static str,
    value: u32,
    min: u32,
    max: u32,
    on_change: impl Fn(u32) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        NumberInput::new(value as f32, min as f32, max as f32, 1.0, 0, 0.15, move |raw| {
            on_change(raw.round() as u32)
        })
        .settings_style(),
    )
}

fn server_button<'a>(
    label: String,
    detail: String,
    healthy: bool,
    selected: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let mut entry = row![
        status_dot(healthy),
        column![
            text(label).size(14).wrapping(Wrapping::WordOrGlyph),
            text(detail).size(12).wrapping(Wrapping::WordOrGlyph).style(settings_muted_text_style),
        ]
        .spacing(4)
        .width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    if selected {
        entry = entry.push(text("✓").size(14));
    }
    button(entry)
        .on_press(on_press)
        .padding([10, 14])
        .width(Length::Fill)
        .style(move |theme, status| {
            let mut style = rounded_action_btn_style(theme, status);
            let accent = theme.palette().primary;
            if selected {
                style.border.color = accent.scale_alpha(0.42);
            }
            let tint = match (selected, status) {
                (true, button::Status::Pressed) => 0.28,
                (true, button::Status::Hovered) => 0.21,
                (true, _) => 0.14,
                (false, button::Status::Pressed) => 0.14,
                (false, button::Status::Hovered) => 0.08,
                (false, _) => 0.0,
            };
            if tint > 0.0 {
                style.background = Some(accent.scale_alpha(tint).into());
            }
            style
        })
        .into()
}

fn status_dot(healthy: bool) -> Element<'static, Message> {
    container(Space::new().width(Length::Fixed(8.0)).height(Length::Fixed(8.0)))
        .style(move |theme: &Theme| {
            let color = if healthy {
                Color::from_rgb8(18, 190, 35)
            } else {
                theme.palette().text.scale_alpha(0.28)
            };
            iced::widget::container::Style {
                background: Some(iced::Background::Color(color)),
                border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 4.0.into() },
                ..Default::default()
            }
        })
        .into()
}

fn server_healthy(app: &App, server: &crate::app::state::GatewayClientServerDraft) -> bool {
    crate::app::message::gateway_health::server_health_key(server)
        .and_then(|key| app.gateway_client_settings.health.get(&key).copied())
        .unwrap_or(false)
}

/// 构建或处理 `view` 对应的界面片段与交互数据。
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
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.gateway_client_settings;

    let help_btn = settings_help_button_with_locale(
        app.desktop_locale,
        Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::HelpOpen)),
    );

    let server_items = s
        .servers
        .iter()
        .map(|server| {
            let selected = server.id == s.selected_server_id;
            server_button(
                server.name.clone(),
                format!("{}:{}", server.host, server.port),
                server_healthy(app, server),
                selected,
                Message::Settings(SettingsMessage::GatewayClient(
                    GatewayClientMessage::SelectServer(server.id.clone()),
                )),
            )
        })
        .collect::<Vec<_>>();
    let server_list: Element<'_, Message> = if server_items.is_empty() {
        column![
            text(connection_text(app.desktop_locale, "暂无网关"))
                .size(13)
                .style(settings_muted_text_style)
        ]
        .into()
    } else {
        column(server_items).spacing(8).into()
    };

    let add_server = button(
        row![text("+").size(13), text(connection_text(app.desktop_locale, "新增网关")).size(13)]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .padding([8, 14])
    .style(primary_action_btn_style)
    .on_press(Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::AddServer)));
    let remove_server = button(text(connection_text(app.desktop_locale, "删除当前")).size(13))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press_maybe((s.servers.len() > 1).then(|| {
            Message::Settings(SettingsMessage::GatewayClient(
                GatewayClientMessage::RemoveServerRequested(s.selected_server_id.clone()),
            ))
        }));
    let refresh_server = button(text(connection_text(app.desktop_locale, "刷新状态")).size(13))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press(Message::GatewayHealthTick);

    let name_row = text_row(
        connection_text(app.desktop_locale, "网关名称"),
        connection_text(app.desktop_locale, "用于区分多个客户端网关。"),
        connection_text(app.desktop_locale, "本地网关"),
        &s.name_input,
        |value| {
            Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::NameChanged(
                value,
            )))
        },
    );
    let host_row = text_row(
        connection_text(app.desktop_locale, "目标主机"),
        connection_text(app.desktop_locale, "桌面端请求所连接的 Gateway 主机。"),
        "127.0.0.1",
        &s.host_input,
        |value| {
            Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::HostChanged(
                value,
            )))
        },
    );
    let port_row = number_row(
        connection_text(app.desktop_locale, "目标端口"),
        connection_text(app.desktop_locale, "桌面端请求所连接的 Gateway 端口。"),
        s.port as u32,
        1,
        u16::MAX as u32,
        |value| {
            Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::PortChanged(
                value as u16,
            )))
        },
    );
    let platform_id_row = text_row(
        connection_text(app.desktop_locale, "平台 ID"),
        connection_text(app.desktop_locale, "当前网关请求所属的平台租户。"),
        "10000",
        &s.platform_id_input,
        |value| {
            Message::Settings(SettingsMessage::GatewayClient(
                GatewayClientMessage::PlatformIdChanged(value),
            ))
        },
    );
    let company_id_row = text_row(
        connection_text(app.desktop_locale, "公司 ID"),
        connection_text(app.desktop_locale, "当前网关请求所属的公司租户。"),
        "100000000",
        &s.company_id_input,
        |value| {
            Message::Settings(SettingsMessage::GatewayClient(
                GatewayClientMessage::CompanyIdChanged(value),
            ))
        },
    );
    let skey_row = secure_text_row(
        "skey",
        connection_text(app.desktop_locale, "可选 skey，会作为 Authorization: Bearer 发送。"),
        connection_text(app.desktop_locale, "可选 skey"),
        &s.skey_input,
        |value| {
            Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::SkeyChanged(
                value,
            )))
        },
    );

    let content = column![
        row![
            container(settings_page_heading(
                connection_text(app.desktop_locale, "客户端网关"),
                connection_text(
                    app.desktop_locale,
                    "配置 Desktop 作为客户端访问一个或多个 Gateway 时使用的地址与认证信息。"
                ),
            ))
            .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start),
        settings_group_heading(
            connection_text(app.desktop_locale, "网关服务"),
            connection_text(
                app.desktop_locale,
                "可以维护多个客户端网关，当前选中的网关会用于 Desktop 的 Gateway 请求。"
            ),
        ),
        settings_group_card(
            container(
                column![
                    server_list,
                    row![add_server, remove_server, refresh_server]
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .wrap(),
                ]
                .spacing(12),
            )
            .padding([14, 0]),
        ),
        text(connection_text(app.desktop_locale, "连接参数")).size(17),
        settings_group_card(column![
            name_row,
            settings_divider(),
            host_row,
            settings_divider(),
            port_row,
            settings_divider(),
            platform_id_row,
            settings_divider(),
            company_id_row,
        ]),
        settings_group_heading(
            connection_text(app.desktop_locale, "认证"),
            connection_text(
                app.desktop_locale,
                "可选填写 skey。请求会通过 Authorization: Bearer <skey> 发送。"
            ),
        ),
        settings_group_card(skey_row),
    ]
    .spacing(16)
    .width(Length::Fill);

    content.into()
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
    let s = &app.gateway_client_settings;
    let dialog = if let Some(server_id) = &s.pending_remove_server_id {
        if let Some(server) = s.servers.iter().find(|server| &server.id == server_id) {
            let confirm = Message::Settings(SettingsMessage::GatewayClient(
                GatewayClientMessage::RemoveServerConfirmed(server.id.clone()),
            ));
            let cancel = Message::Settings(SettingsMessage::GatewayClient(
                GatewayClientMessage::RemoveServerCanceled,
            ));
            let confirm_dialog = crate::app::components::toast::confirm_dialog(
                connection_text(app.desktop_locale, "确认删除客户端网关"),
                owned_text(
                    app.desktop_locale,
                    "将删除客户端网关「%s」。此操作不可撤销。\n地址: %s:%s\nID: %s",
                    &[
                        server.name.clone(),
                        server.host.clone(),
                        server.port.to_string(),
                        server.id.clone(),
                    ],
                ),
                connection_text(app.desktop_locale, "确认删除"),
                connection_text(app.desktop_locale, "取消"),
                confirm,
                cancel,
            );
            iced::widget::stack![dialog, confirm_dialog].into()
        } else {
            dialog
        }
    } else {
        dialog
    };

    if !s.show_help_modal {
        return dialog;
    }

    let help_text = crate::app::i18n::settings_models::help::gateway_client(app.desktop_locale);

    with_settings_help_modal(
        app,
        dialog,
        connection_text(app.desktop_locale, "客户端网关配置帮助"),
        help_text,
        Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::HelpClose)),
    )
}
#[cfg(test)]
#[path = "system_settings_gateway_client_tests.rs"]
mod system_settings_gateway_client_tests;
