//! 系统设置中 gateway 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use desktop_ui_controls::system_settings_common::*;
use crate::help::{settings_hover_help_label, settings_page_heading_with_help, settings_group_heading_with_help, settings_flexible_row_with_help, with_settings_help_modal, with_settings_help_modal_mapped, HelpRenderer};
use desktop_i18n::I18n;
use desktop_i18n::settings_models::connection_text::text as connection_text;
use desktop_i18n::settings_models::messages::render as owned_text;
use crate::messages::*;
use crate::Message;
use crate::state::*;
use desktop_ui_controls::number_input::NumberInput;
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_assets::assets::Icon;
use chrono::{Datelike, NaiveDate, Utc};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, checkbox, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

const GATEWAY_PAIRED_TOKEN_VISIBLE_ROWS: usize = 10;
const GATEWAY_PAIRED_TOKEN_ROW_HEIGHT: f32 = 36.0;
const GATEWAY_PAIRED_TOKEN_ROW_SPACING: f32 = 10.0;
const GATEWAY_PAIRED_TOKEN_SCROLLBAR_WIDTH: u32 = 4;
const GATEWAY_SKEY_CALENDAR_CELL_SIZE: f32 = 34.0;

fn paired_token_list_max_height(token_count: usize) -> f32 {
    let visible_rows = token_count.min(GATEWAY_PAIRED_TOKEN_VISIBLE_ROWS);
    let row_spacing_count = visible_rows.saturating_sub(1);

    visible_rows as f32 * GATEWAY_PAIRED_TOKEN_ROW_HEIGHT
        + row_spacing_count as f32 * GATEWAY_PAIRED_TOKEN_ROW_SPACING
}

/// 本页实际使用的只读输入；状态由网络领域拥有。
#[derive(Clone, Copy)]
pub struct View<'a> {
    pub gateway_settings: &'a GatewaySettingsState,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: &'a iced::Theme,
    pub last_copied_code_hash: Option<u64>,
    pub render_help: HelpRenderer,
}

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_row_with_flexible_control(label, description, 460.0, control)
}

fn skey_field<'a>(
    label: impl Into<Element<'a, Message>>,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(column![label.into(), settings_control_with_hint(control, description)].spacing(6))
        .padding([16, 0])
        .width(Length::Fill)
        .into()
}

fn text_row_with_hint<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    hint: &'static str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        settings_control_with_hint(
            settings_text_input(placeholder, value).on_input(on_input).width(Length::Fill),
            hint,
        ),
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
        settings_text_input(placeholder, value).secure(true).on_input(on_input).width(Length::Fill),
    )
}

fn bool_row<'a>(
    label: &'static str,
    description: &'static str,
    checked: bool,
    checkbox_label: &'static str,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        checkbox(checked).label(checkbox_label).on_toggle(on_toggle).style(settings_checkbox_style),
    )
}

fn bool_row_with_hint<'a>(
    label: &'static str,
    description: &'static str,
    checked: bool,
    checkbox_label: &'static str,
    hint: &'static str,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        settings_control_with_hint(
            checkbox(checked)
                .label(checkbox_label)
                .on_toggle(on_toggle)
                .style(settings_checkbox_style),
            hint,
        ),
    )
}

fn number_row<'a>(
    app: View<'a>,
    label: &'static str,
    description: &'static str,
    value: u32,
    range: std::ops::RangeInclusive<u32>,
    suffix: &'static str,
    on_change: impl Fn(u32) -> Message + 'a,
) -> Element<'a, Message> {
    let display = if suffix.is_empty() { value.to_string() } else { format!("{value} {suffix}") };

    settings_flexible_row_with_help(app.render_help,
        I18n::new(app.desktop_locale),
        app.app_theme,
        label,
        description,
        460.0,
        row![
            NumberInput::new(
                value as f32,
                *range.start() as f32,
                *range.end() as f32,
                1.0,
                0,
                0.15, Message::None,
                move |raw| on_change(raw.round() as u32),
            )
            .settings_style(),
            settings_value_badge(display),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    )
    .into()
}

pub const GATEWAY_TAB_LABELS: &[(&str, GatewaySettingsTab)] =
    &[("配置", GatewaySettingsTab::Config), ("skey 管理", GatewaySettingsTab::Skeys)];

fn gateway_tab_labels() -> &'static [(&'static str, GatewaySettingsTab)] {
    GATEWAY_TAB_LABELS
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
pub fn view<'a>(app: View<'a>) -> Element<'a, Message> {
    let s = app.gateway_settings;
    let i18n = I18n::new(app.desktop_locale);
    let theme = app.app_theme;

    let help_btn = settings_help_button_with_locale(
        app.desktop_locale,
        Message::Gateway(GatewayMessage::HelpOpen),
    );

    let port_row = number_row(
        app,
        connection_text(app.desktop_locale, "端口"),
        connection_text(app.desktop_locale, "控制服务端 Gateway 的监听端口。"),
        s.port as u32,
        1..=u16::MAX as u32,
        "",
        |value| {
            Message::Gateway(GatewayMessage::PortChanged(value as u16))
        },
    );
    let host_row = settings_flexible_row_with_help(app.render_help,
        i18n,
        theme,
        connection_text(app.desktop_locale, "主机地址"),
        connection_text(app.desktop_locale, "控制服务端 Gateway 的监听地址。"),
        460.0,
        settings_control_with_hint(
            settings_text_input("127.0.0.1", &s.host_input)
                .on_input(|value| {
                    Message::Gateway(GatewayMessage::HostChanged(value))
                })
                .width(Length::Fill),
            connection_text(
                app.desktop_locale,
                "若 host 非本地地址，建议仅在可信反向代理或隧道环境下使用。",
            ),
        ),
    );

    let auth_enabled_row = bool_row(
        connection_text(app.desktop_locale, "启用 skey 鉴权"),
        connection_text(
            app.desktop_locale,
            "开启后，受保护请求必须通过 Authorization: Bearer 提供有效 skey。",
        ),
        s.auth_enabled,
        connection_text(app.desktop_locale, "通过 Authorization: Bearer 校验 skey"),
        |value| {
            Message::Gateway(GatewayMessage::AuthEnabledToggled(value))
        },
    );
    let allow_public_bind_row = bool_row(
        connection_text(app.desktop_locale, "允许公网绑定"),
        "",
        s.allow_public_bind,
        connection_text(app.desktop_locale, "允许绑定到非本地地址"),
        |value| {
            Message::Gateway(GatewayMessage::AllowPublicBindToggled(
                value,
            ))
        },
    );
    let trust_forwarded_headers_row = bool_row_with_hint(
        connection_text(app.desktop_locale, "信任转发头"),
        "",
        s.trust_forwarded_headers,
        connection_text(app.desktop_locale, "信任 X-Forwarded-For / X-Real-IP"),
        connection_text(app.desktop_locale, "仅在你完全信任前置代理时开启 forwarded headers。"),
        |value| {
            Message::Gateway(
                GatewayMessage::TrustForwardedHeadersToggled(value),
            )
        },
    );

    let skeys_section = settings_group_card(
        column![
            skey_field(
                settings_hover_help_label(app.render_help,
                    i18n,
                    theme,
                    connection_text(app.desktop_locale, "skey 名称"),
                    settings_field_label(connection_text(app.desktop_locale, "skey 名称")),
                    connection_text(app.desktop_locale, "用于列表展示和识别该 skey。"),
                )
                .wrap(),
                "",
                row![
                    settings_text_input(
                        connection_text(app.desktop_locale, "例如 iPhone / CI Runner"),
                        &s.new_skey_name_input
                    )
                    .on_input(|value| Message::Gateway(
                        GatewayMessage::NewSkeyNameChanged(value)
                    ))
                    .width(Length::Fill),
                    button(text(connection_text(app.desktop_locale, "生成 skey")).size(13))
                        .padding([10, 14])
                        .on_press(Message::Gateway(
                            GatewayMessage::AddSkey
                        ))
                        .style(primary_action_btn_style),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
            ),
            settings_divider(),
            skey_field(
                settings_field_label(connection_text(app.desktop_locale, "过期日期")),
                connection_text(
                    app.desktop_locale,
                    "可手写 YYYY-MM-DD，也可点日历选择；为空表示永不过期。"
                ),
                skey_date_picker(app)
            )
        ]
        .spacing(0),
    );

    let last_created_skey_panel = s.last_created_skey.as_ref().map(|raw_skey| {
        let copy_label = if s.last_created_skey_copied {
            "✓"
        } else {
            connection_text(app.desktop_locale, "复制")
        };
        settings_group_card(skey_field(
            settings_field_label(connection_text(app.desktop_locale, "原始 skey")),
            connection_text(app.desktop_locale, "仅展示一次，请立即复制。"),
            row![
                text(raw_skey.as_str())
                    .size(13)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                    .width(Length::Fill),
                button(text(copy_label).size(13))
                    .padding([8, 12])
                    .on_press(Message::Gateway(
                        GatewayMessage::CopyLastCreatedSkey
                    ))
                    .style(rounded_action_btn_style),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        ))
    });

    let skeys_list: Element<'a, Message> = if s.skeys.is_empty() {
        settings_group_card(
            text(connection_text(
                app.desktop_locale,
                "当前没有 skey。开启 skey 鉴权后，缺少有效 skey 的请求会被拒绝。",
            ))
            .size(13)
            .style(settings_muted_text_style),
        )
        .padding([16, 22])
        .into()
    } else {
        let list = s.skeys.iter().enumerate().fold(
            column![].spacing(GATEWAY_PAIRED_TOKEN_ROW_SPACING),
            |column, (index, skey)| {
                let name = if skey.name.trim().is_empty() {
                    format!("skey #{}", index + 1)
                } else {
                    skey.name.clone()
                };
                let skey_display = if skey.masked_skey.trim().is_empty() {
                    format!("{}...", skey.skey_hash.chars().take(12).collect::<String>())
                } else {
                    skey.masked_skey.clone()
                };
                let expires_at = skey
                    .expires_at
                    .as_deref()
                    .unwrap_or(connection_text(app.desktop_locale, "永不过期"));
                let status = if skey.enabled {
                    connection_text(app.desktop_locale, "启用")
                } else {
                    connection_text(app.desktop_locale, "禁用")
                };

                column.push(
                    container(
                        row![
                            checkbox(skey.enabled)
                                .label(status)
                                .on_toggle(move |value| {
                                    Message::Gateway(
                                        GatewayMessage::SkeyEnabledToggled(index, value),
                                    )
                                })
                                .style(settings_checkbox_style)
                                .width(Length::Shrink),
                            text(name)
                                .size(13)
                                .width(Length::FillPortion(2))
                                .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                            text(format!("{skey_display} · {expires_at}"))
                                .size(12)
                                .width(Length::FillPortion(3))
                                .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                                .style(settings_muted_text_style),
                            button(text(connection_text(app.desktop_locale, "删除")).size(13))
                                .padding([6, 12])
                                .height(30)
                                .on_press(Message::Gateway(
                                    GatewayMessage::RemoveSkey(index),
                                ))
                                .style(rounded_action_btn_style),
                        ]
                        .spacing(12)
                        .align_y(Alignment::Center),
                    )
                    .padding([3, 0]),
                )
            },
        );

        let list_height = paired_token_list_max_height(s.skeys.len());
        let scrollable_list = container(
            scrollable(container(list).padding([0, 7]).width(Length::Fill))
                .direction(Direction::Vertical(
                    Scrollbar::new()
                        .width(GATEWAY_PAIRED_TOKEN_SCROLLBAR_WIDTH)
                        .scroller_width(GATEWAY_PAIRED_TOKEN_SCROLLBAR_WIDTH),
                ))
                .height(Length::Shrink),
        )
        .width(Length::Fill)
        .max_height(list_height);

        settings_group_card(scrollable_list).padding([16, 15]).into()
    };

    let webhook_rate_row = number_row(
        app,
        connection_text(app.desktop_locale, "Webhook速率限制/分钟"),
        connection_text(app.desktop_locale, "限制 /webhook 每分钟可接受的请求数。"),
        s.webhook_rate_limit_per_minute,
        1..=100_000,
        connection_text(app.desktop_locale, "次/分钟"),
        |value| {
            Message::Gateway(
                GatewayMessage::WebhookRateLimitPerMinuteChanged(value),
            )
        },
    );
    let rate_limit_max_keys_row = number_row(
        app,
        connection_text(app.desktop_locale, "速率限制最大键数"),
        connection_text(app.desktop_locale, "约束网关内部 key map 规模。"),
        s.rate_limit_max_keys,
        1..=100_000,
        connection_text(app.desktop_locale, "个键"),
        |value| {
            Message::Gateway(GatewayMessage::RateLimitMaxKeysChanged(
                value,
            ))
        },
    );

    let idempotency_ttl_row = number_row(
        app,
        connection_text(app.desktop_locale, "幂等性TTL(秒)"),
        connection_text(app.desktop_locale, "控制 webhook 幂等 key 的存活时间。"),
        s.idempotency_ttl_secs,
        1..=86_400,
        connection_text(app.desktop_locale, "秒"),
        |value| {
            Message::Gateway(GatewayMessage::IdempotencyTtlSecsChanged(
                value,
            ))
        },
    );
    let idempotency_max_keys_row = number_row(
        app,
        connection_text(app.desktop_locale, "幂等性最大键数"),
        connection_text(app.desktop_locale, "控制幂等缓存允许保留的 key 数量。"),
        s.idempotency_max_keys,
        1..=100_000,
        connection_text(app.desktop_locale, "个键"),
        |value| {
            Message::Gateway(GatewayMessage::IdempotencyMaxKeysChanged(
                value,
            ))
        },
    );

    let node_control_enabled_row = bool_row(
        connection_text(app.desktop_locale, "节点控制.已启用"),
        "",
        s.node_control_enabled,
        connection_text(app.desktop_locale, "启用实验性的 node-control API"),
        |value| {
            Message::Gateway(GatewayMessage::NodeControlEnabledToggled(
                value,
            ))
        },
    );
    let node_control_auth_token_row = secure_text_row(
        connection_text(app.desktop_locale, "节点控制.认证令牌"),
        connection_text(
            app.desktop_locale,
            "额外共享令牌，客户端需通过 X-Node-Control-Token 传递。",
        ),
        connection_text(app.desktop_locale, "可选共享令牌"),
        &s.node_control_auth_token_input,
        |value| {
            Message::Gateway(
                GatewayMessage::NodeControlAuthTokenChanged(value),
            )
        },
    );
    let node_control_allowed_ids_row = text_row_with_hint(
        connection_text(app.desktop_locale, "节点控制.允许的节点ID"),
        "",
        connection_text(app.desktop_locale, "逗号或换行分隔，支持 *"),
        &s.node_control_allowed_node_ids_input,
        connection_text(
            app.desktop_locale,
            "allowed_node_ids 留空表示不设置显式 allowlist；可使用 * 允许所有节点。",
        ),
        |value| {
            Message::Gateway(
                GatewayMessage::NodeControlAllowedNodeIdsChanged(value),
            )
        },
    );

    let mut skey_management = column![skeys_section].spacing(16).width(Length::Fill);
    if let Some(panel) = last_created_skey_panel {
        skey_management = skey_management.push(panel);
    }
    skey_management = skey_management.push(skeys_list);

    let tab_bar = settings_tab_row(gateway_tab_labels().iter().map(|(label, tab)| {
        settings_tab_button(
            connection_text(app.desktop_locale, label),
            *tab == s.active_tab,
            Some(Message::Gateway(GatewayMessage::TabSelected(*tab))),
        )
        .into()
    }))
    .wrap();

    let config_content: Element<'a, Message> = column![
        settings_group_heading_with_help(app.render_help,
            i18n,
            theme,
            connection_text(app.desktop_locale, "服务管理"),
            connection_text(
                app.desktop_locale,
                "通过本机 gateway 调用 shiju service 管理 OS 服务生命周期。"
            )
        ),
        service_controls_content(app),
        settings_group_heading_with_help(app.render_help,
            i18n,
            theme,
            connection_text(app.desktop_locale, "监听地址"),
            connection_text(
                app.desktop_locale,
                "控制网关监听 host / port。默认仅绑定回环地址，避免意外暴露到公网。"
            )
        ),
        settings_group_card(column![port_row, settings_divider(), host_row,].spacing(0)),
        settings_group_heading_with_help(app.render_help,
            i18n,
            theme,
            connection_text(app.desktop_locale, "安全开关"),
            connection_text(app.desktop_locale, "控制 skey 鉴权、公网绑定与反向代理头信任策略。")
        ),
        settings_group_card(
            column![
                auth_enabled_row,
                settings_divider(),
                allow_public_bind_row,
                settings_divider(),
                trust_forwarded_headers_row,
            ]
            .spacing(0),
        ),
        settings_group_heading_with_help(app.render_help,
            i18n,
            theme,
            connection_text(app.desktop_locale, "速率限制"),
            connection_text(
                app.desktop_locale,
                "限制 /webhook 请求频率，并约束网关内部 key map 规模。"
            )
        ),
        settings_group_card(
            column![webhook_rate_row, settings_divider(), rate_limit_max_keys_row].spacing(0),
        ),
        settings_group_heading_with_help(app.render_help,
            i18n,
            theme,
            connection_text(app.desktop_locale, "幂等性"),
            connection_text(app.desktop_locale, "控制 webhook 幂等 key 的存活时间与内存上限。")
        ),
        settings_group_card(
            column![idempotency_ttl_row, settings_divider(), idempotency_max_keys_row].spacing(0)
        ),
        settings_group_heading_with_help(app.render_help,
            i18n,
            theme,
            "Node Control",
            connection_text(
                app.desktop_locale,
                "配置实验性的 node-control 协议，包括额外鉴权和允许的远端节点 ID。"
            )
        ),
        settings_group_card(
            column![
                node_control_enabled_row,
                settings_divider(),
                node_control_auth_token_row,
                settings_divider(),
                node_control_allowed_ids_row,
            ]
            .spacing(0),
        ),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into();

    let skeys_content: Element<'a, Message> = column![
        settings_group_heading_with_help(app.render_help,
            i18n,
            theme,
            connection_text(app.desktop_locale, "skey 管理"),
            connection_text(
                app.desktop_locale,
                "维护 gateway.skeys 列表。原始 skey 不保存，仅保存哈希、脱敏 skey、名称和过期时间。"
            ),
        ),
        skey_management,
    ]
    .spacing(16)
    .width(Length::Fill)
    .into();

    let active_content = match s.active_tab {
        GatewaySettingsTab::Config => config_content,
        GatewaySettingsTab::Skeys => skeys_content,
    };

    let mut content = column![
        row![
            container(settings_page_heading_with_help(app.render_help,
                i18n,
                theme,
                connection_text(app.desktop_locale, "服务端网关配置"),
                connection_text(
                    app.desktop_locale,
                    "配置 Gateway 的监听地址、skey 鉴权、限流和实验性 node-control 行为。"
                ),
            ))
            .width(Length::Fill),
            container(help_btn).width(Length::Shrink),
        ]
        .spacing(12)
        .align_y(Alignment::Start)
        .width(Length::Fill),
        tab_bar,
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(error) = s.save_error.as_ref() {
        content = content.push(settings_error_banner(error.display(app.desktop_locale)));
    }

    content.push(active_content).into()
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
fn view_overlays<'a>(app: View<'a>, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = app.gateway_settings;
    if let Some(command) = s.pending_service_command_confirmation.as_deref() {
        let label = service_display_label(app.desktop_locale, command);
        let command_text = service_cli_command(command);
        let body = owned_text(
            app.desktop_locale,
            "%s 是危险操作，可能会中断当前本机 gateway 连接、停止后台任务，卸载服务还会移除开机自启动配置。\n\n将执行命令：%s\n\n确认继续？",
            &[label.to_string(), command_text.clone()],
        );
        let cancel_message =
            Message::Gateway(GatewayMessage::CancelDangerServiceCommand);
        let confirm_message = Message::Gateway(
            GatewayMessage::ConfirmDangerServiceCommand(command.to_string()),
        );
        let title = owned_text(app.desktop_locale, "确认%s", &[label.to_string()]);
        let header = settings_modal_header(
            text(title.clone()).size(18),
            settings_close_button(cancel_message.clone()),
        );
        let actions = row![
            Space::new().width(Length::Fill),
            button(text(connection_text(app.desktop_locale, "取消")).size(13))
                .padding([9, 14])
                .style(rounded_action_btn_style)
                .on_press(cancel_message.clone()),
            button(text(title).size(13))
                .padding([9, 14])
                .style(danger_action_btn_style)
                .on_press(confirm_message),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        let card = settings_modal_layout(
            header,
            text(body).size(13).style(settings_muted_text_style),
            Some(actions.into()),
        )
        .width(480)
        .height(310)
        .style(|theme| {
            let mut style = settings_modal_card_style(theme);
            if let Some(iced::Background::Color(color)) = &mut style.background {
                color.a = 1.0;
            }
            style
        });

        return settings_modal_overlay(Some(dialog), cancel_message, container(card).padding(12));
    }

    if !s.show_help_modal {
        return dialog;
    }

    let help_text = desktop_i18n::settings_models::help::gateway(app.desktop_locale);

    crate::help::with_settings_help_modal(
        app.desktop_locale, app.last_copied_code_hash,
        dialog,
        connection_text(app.desktop_locale, "Gateway 配置帮助"),
        help_text,
        Message::Gateway(GatewayMessage::HelpClose),
    )
}


#[cfg(test)]
#[path = "system_settings_gateway_tests.rs"]
mod system_settings_gateway_tests;

/// 返回覆盖层；宿主在已有页面上组合，页面消息无需反向映射。
pub fn overlay<'a>(app: View<'a>) -> Option<Element<'a, Message>> {
    if !(app.gateway_settings.pending_service_command_confirmation.is_some() || app.gateway_settings.show_help_modal) { return None; }
    Some(view_overlays(app, iced::widget::Space::new().width(Length::Fill).height(Length::Fill).into()))
}

mod service;
mod calendar;
use service::*;
use calendar::*;
pub use service::{service_command_copy_button, service_display_label};
pub use calendar::{skey_calendar_popover, skey_date_picker};
