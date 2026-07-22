//! 系统设置中 gateway client 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::number_input::NumberInput;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, SETTINGS_LABEL_WIDTH,
    settings_help_button, settings_muted_text_style, settings_page_intro, settings_panel,
    settings_section_card, settings_text_input_style, settings_value_badge,
    with_settings_help_modal,
};
use crate::app::message::settings::{GatewayClientMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Alignment, Color, Element, Length, Theme};

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
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
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
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
    suffix: &'static str,
    on_change: impl Fn(u32) -> Message + 'a,
) -> Element<'a, Message> {
    let display = if suffix.is_empty() { value.to_string() } else { format!("{value} {suffix}") };

    field_row(
        label,
        description,
        row![
            NumberInput::new(value as f32, min as f32, max as f32, 1.0, 0, 0.15, move |raw| {
                on_change(raw.round() as u32)
            })
            .settings_style(),
            settings_value_badge(display),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    )
}

fn server_button<'a>(
    label: String,
    detail: String,
    healthy: bool,
    selected: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let marker = if selected { "✓" } else { "" };
    button(
        row![
            status_dot(healthy),
            column![text(label).size(13), text(detail).size(11).style(settings_muted_text_style)]
                .spacing(3)
                .width(Length::Fill),
            text(marker).size(14),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .on_press(on_press)
    .padding([10, 12])
    .width(Length::Fill)
    .style(move |theme, status| {
        let palette = theme.extended_palette();
        let background = if selected || status == iced::widget::button::Status::Hovered {
            Some(iced::Background::Color(palette.background.weak.color))
        } else {
            None
        };
        iced::widget::button::Style {
            background,
            text_color: theme.palette().text,
            border: iced::Border {
                width: 1.0,
                color: palette.background.strong.color.scale_alpha(0.60),
                radius: 6.0.into(),
            },
            ..Default::default()
        }
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

fn rounded_action_button<'a>(
    label: Element<'a, Message>,
    enabled: bool,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let base = button(container(label).height(Length::Fill).align_y(Alignment::Center))
        .height(Length::Fixed(36.0))
        .padding([0, 14])
        .style(move |theme: &Theme, status| {
            let palette = theme.extended_palette();
            let hovered = enabled && status == iced::widget::button::Status::Hovered;
            iced::widget::button::Style {
                background: Some(iced::Background::Color(if hovered {
                    palette.background.weak.color
                } else {
                    theme.palette().background
                })),
                text_color: if enabled {
                    theme.palette().text
                } else {
                    theme.palette().text.scale_alpha(0.38)
                },
                border: iced::Border {
                    width: 1.0,
                    color: palette.background.strong.color.scale_alpha(0.65),
                    radius: 8.0.into(),
                },
                shadow: iced::Shadow::default(),
                ..Default::default()
            }
        });

    if let Some(on_press) = on_press { base.on_press(on_press).into() } else { base.into() }
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

    let help_btn = settings_help_button(Message::Settings(SettingsMessage::GatewayClient(
        GatewayClientMessage::HelpOpen,
    )));

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
        column![text("暂无网关").size(13).style(settings_muted_text_style)].into()
    } else {
        column(server_items).spacing(8).into()
    };

    let add_server = rounded_action_button(
        row![text("+").size(16), text("新增网关").size(13)]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        true,
        Some(Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::AddServer))),
    );
    let can_remove_server = s.servers.len() > 1;
    let remove_server = rounded_action_button(
        row![text("删除当前").size(13)].align_y(Alignment::Center).into(),
        can_remove_server,
        can_remove_server.then(|| {
            Message::Settings(SettingsMessage::GatewayClient(
                GatewayClientMessage::RemoveServerRequested(s.selected_server_id.clone()),
            ))
        }),
    );
    let refresh_server = rounded_action_button(
        row![text("刷新状态").size(13)].align_y(Alignment::Center).into(),
        true,
        Some(Message::GatewayHealthTick),
    );

    let name_row = text_row(
        "网关名称",
        "用于区分多个客户端网关。",
        "本地网关",
        &s.name_input,
        |value| {
            Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::NameChanged(
                value,
            )))
        },
    );
    let host_row = text_row(
        "目标主机",
        "桌面端请求所连接的 Gateway 主机。",
        "127.0.0.1",
        &s.host_input,
        |value| {
            Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::HostChanged(
                value,
            )))
        },
    );
    let port_row = number_row(
        "目标端口",
        "桌面端请求所连接的 Gateway 端口。",
        s.port as u32,
        1,
        u16::MAX as u32,
        "",
        |value| {
            Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::PortChanged(
                value as u16,
            )))
        },
    );
    let platform_id_row = text_row(
        "平台 ID",
        "当前网关请求所属的平台租户。",
        "10000",
        &s.platform_id_input,
        |value| {
            Message::Settings(SettingsMessage::GatewayClient(
                GatewayClientMessage::PlatformIdChanged(value),
            ))
        },
    );
    let company_id_row = text_row(
        "公司 ID",
        "当前网关请求所属的公司租户。",
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
        "可选 skey，会作为 Authorization: Bearer 发送。",
        "可选 skey",
        &s.skey_input,
        |value| {
            Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::SkeyChanged(
                value,
            )))
        },
    );

    let content = column![
        row![
            container(settings_page_intro(
                "客户端网关",
                "配置 Desktop 作为客户端访问一个或多个 Gateway 时使用的地址与认证信息。",
            ))
            .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start),
        settings_section_card(
            "网关服务",
            "可以维护多个客户端网关，当前选中的网关会用于 Desktop 的 Gateway 请求。",
        ),
        settings_panel(
            row![
                column![server_list, row![add_server, remove_server, refresh_server].spacing(8)]
                    .spacing(12)
                    .width(Length::Fixed(360.0)),
                Space::new().width(Length::Fixed(10.0)),
                column![name_row, host_row, port_row, platform_id_row, company_id_row]
                    .spacing(0)
                    .width(Length::Fill),
            ]
            .spacing(12)
            .align_y(Alignment::Start),
        ),
        settings_section_card(
            "认证",
            "可选填写 skey。请求会通过 Authorization: Bearer <skey> 发送。",
        ),
        settings_panel(column![skey_row].spacing(0)),
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
                "确认删除客户端网关",
                format!(
                    "将删除客户端网关「{}」。此操作不可撤销。\n地址: {}:{}\nID: {}",
                    server.name, server.host, server.port, server.id
                ),
                "确认删除",
                "取消",
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

    let help_text = r#"客户端网关配置说明

一、作用
- 这里配置桌面端发请求时要连接的 Gateway 地址、租户与认证信息。
- 支持多个 Gateway 服务；当前选中的服务会成为 Desktop 的客户端请求目标。
- 这些设置不会修改服务端网关的监听配置。

二、字段含义
1) host
- 目标 Gateway 主机地址，默认 127.0.0.1。

2) port
- 目标 Gateway 端口，默认 42617。

3) platform_id / company_id
- 平台与公司租户 ID，默认 10000 / 100000000；不能为空。

4) skey
- 可选 skey。填写后会作为 Authorization: Bearer <skey> 发送给网关。

三、示例
{
  "gateway_client": {
    "active_server_id": "local",
    "servers": [
      {
        "id": "local",
        "name": "本地网关",
        "host": "127.0.0.1",
        "port": 42617,
        "platform_id": "10000",
        "company_id": "100000000",
        "skey": ""
      }
    ],
    "host": "127.0.0.1",
    "port": 42617,
    "platform_id": "10000",
    "company_id": "100000000",
    "skey": ""
  }
}
"#;

    with_settings_help_modal(
        app,
        dialog,
        "客户端网关配置帮助",
        help_text,
        Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::HelpClose)),
    )
}
#[cfg(test)]
#[path = "system_settings_gateway_client_tests.rs"]
mod system_settings_gateway_client_tests;
