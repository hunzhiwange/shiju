//! 系统设置中 gateway 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::assets::Icon;
use crate::app::components::number_input::NumberInput;
use crate::app::components::overlays::BelowOverlay;
use crate::app::components::system_settings_common::{
    danger_action_btn_style, icon_svg, primary_action_btn_style, rounded_action_btn_style,
    settings_checkbox_style, settings_close_button, settings_control_with_hint, settings_divider,
    settings_error_banner, settings_field_label, settings_group_card, settings_group_heading,
    settings_help_button_with_locale, settings_modal_card_style, settings_modal_header,
    settings_modal_layout, settings_modal_overlay, settings_muted_text_style,
    settings_page_heading, settings_row_with_flexible_control, settings_tab_button,
    settings_tab_row, settings_text_input, settings_value_badge,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::i18n::settings_models::connection_text::text as connection_text;
use crate::app::i18n::settings_models::messages::render as owned_text;
use crate::app::message::settings::{GatewayMessage, SettingsMessage};
use crate::app::state::{GatewaySettingsState, GatewaySettingsTab};
use crate::app::{App, Message};
use chrono::{Datelike, NaiveDate, Utc};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, checkbox, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

pub(super) const GATEWAY_PAIRED_TOKEN_VISIBLE_ROWS: usize = 10;
pub(super) const GATEWAY_PAIRED_TOKEN_ROW_HEIGHT: f32 = 36.0;
pub(super) const GATEWAY_PAIRED_TOKEN_ROW_SPACING: f32 = 10.0;
pub(super) const GATEWAY_PAIRED_TOKEN_SCROLLBAR_WIDTH: u32 = 4;
const GATEWAY_SKEY_CALENDAR_CELL_SIZE: f32 = 34.0;

pub(super) fn paired_token_list_max_height(token_count: usize) -> f32 {
    let visible_rows = token_count.min(GATEWAY_PAIRED_TOKEN_VISIBLE_ROWS);
    let row_spacing_count = visible_rows.saturating_sub(1);

    visible_rows as f32 * GATEWAY_PAIRED_TOKEN_ROW_HEIGHT
        + row_spacing_count as f32 * GATEWAY_PAIRED_TOKEN_ROW_SPACING
}

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_row_with_flexible_control(label, description, 460.0, control)
}

fn skey_field<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        column![settings_field_label(label), settings_control_with_hint(control, description)]
            .spacing(6),
    )
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

const GATEWAY_TAB_LABELS: &[(&str, GatewaySettingsTab)] =
    &[("配置", GatewaySettingsTab::Config), ("skey 管理", GatewaySettingsTab::Skeys)];
pub(super) const GATEWAY_TABS: &[SettingsTabDescriptor<GatewaySettingsTab>] = &[
    SettingsTabDescriptor::new(
        GATEWAY_TAB_LABELS[0].1,
        GATEWAY_TAB_LABELS[0].0,
        &[
            "configuration",
            "監聽地址",
            "监听地址",
            "端口",
            "auth_enabled",
            "公网绑定",
            "反向代理",
            "webhook",
            "幂等性",
            "node-control",
        ],
    ),
    SettingsTabDescriptor::new(
        GATEWAY_TAB_LABELS[1].1,
        GATEWAY_TAB_LABELS[1].0,
        &["skey management", "skey 管理", "密鑰", "skey", "密钥", "过期时间", "鉴权"],
    ),
];

fn gateway_tab_labels() -> &'static [(&'static str, GatewaySettingsTab)] {
    GATEWAY_TAB_LABELS
}

fn service_action_button(
    label: &'static str,
    command: &'static str,
    running_command: Option<&str>,
) -> Element<'static, Message> {
    let is_running = running_command == Some(command);
    let text_label = if is_running { format!("{label}...") } else { label.to_string() };
    let label_content =
        text(text_label).size(14).height(Length::Fill).align_y(iced::alignment::Vertical::Center);
    let btn = button(
        container(label_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .padding([6, 8])
    .width(Length::Fill)
    .height(Length::Fixed(44.0));
    let btn = if is_danger_service_command(command) {
        btn.style(danger_action_btn_style)
    } else {
        btn.style(rounded_action_btn_style)
    };

    if running_command.is_none() {
        btn.on_press(Message::Settings(SettingsMessage::Gateway(
            GatewayMessage::ServiceCommandRequested(command.to_string()),
        )))
        .into()
    } else {
        btn.into()
    }
}

fn service_cli_command(command: &str) -> String {
    format!("shiju server {command}")
}

fn is_danger_service_command(command: &str) -> bool {
    matches!(command, "stop" | "uninstall")
}

fn service_command_copy_button(
    command: &'static str,
    copied_command: Option<&str>,
) -> Element<'static, Message> {
    let cli_command = service_cli_command(command);
    let copied = copied_command == Some(cli_command.as_str());
    let label = if copied { "✓".to_string() } else { cli_command.clone() };
    let icon = if copied { Icon::Check } else { Icon::Copy };
    let content = row![
        text(label)
            .size(11)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
        Space::new().width(Length::Fill),
        icon_svg(icon, 13.0).style(|theme: &iced::Theme, _status| iced::widget::svg::Style {
            color: Some(theme.palette().text),
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    button(container(content).width(Length::Fill).height(Length::Fill).center_y(Length::Fill))
        .padding([4, 8])
        .width(Length::Fill)
        .height(Length::Fixed(40.0))
        .on_press(Message::Settings(SettingsMessage::Gateway(GatewayMessage::CopyServiceCommand(
            cli_command,
        ))))
        .style(rounded_action_btn_style)
        .into()
}

fn service_action_cell(
    label: &'static str,
    command: &'static str,
    running_command: Option<&str>,
    copied_command: Option<&str>,
) -> Element<'static, Message> {
    column![
        service_action_button(label, command, running_command),
        service_command_copy_button(command, copied_command),
    ]
    .spacing(6)
    .width(Length::Fill)
    .into()
}

fn service_controls_content(app: &App) -> Element<'_, Message> {
    let s = &app.gateway_settings;
    let running = s.service_action_running.as_deref();
    let copied_command = s.copied_service_command.as_deref();
    let actions = column![
        row![
            service_action_cell(
                connection_text(app.desktop_locale, "安装服务"),
                "install",
                running,
                copied_command
            ),
            service_action_cell(
                connection_text(app.desktop_locale, "启动服务"),
                "start",
                running,
                copied_command
            ),
            service_action_cell(
                connection_text(app.desktop_locale, "停止服务"),
                "stop",
                running,
                copied_command
            ),
        ]
        .spacing(12),
        row![
            service_action_cell(
                connection_text(app.desktop_locale, "重启服务"),
                "restart",
                running,
                copied_command
            ),
            service_action_cell(
                connection_text(app.desktop_locale, "查询状态"),
                "status",
                running,
                copied_command
            ),
            service_action_cell(
                connection_text(app.desktop_locale, "卸载服务"),
                "uninstall",
                running,
                copied_command
            ),
        ]
        .spacing(12),
    ]
    .spacing(14)
    .width(Length::Fill);

    let mut content = column![actions].spacing(12).width(Length::Fill);
    if let Some(output) = s.service_action_output.as_ref().filter(|value| !value.trim().is_empty())
    {
        content = content.push(
            container(
                text(output.display(app.desktop_locale))
                    .size(13)
                    .style(settings_muted_text_style)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
            )
            .padding([10, 12])
            .width(Length::Fill),
        );
    }

    settings_group_card(content).padding([16, 22]).into()
}

fn parse_skey_date(raw: &str) -> Option<NaiveDate> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").ok().or_else(|| {
        chrono::DateTime::parse_from_rfc3339(trimmed).ok().map(|value| value.date_naive())
    })
}

fn month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date)
}

fn parse_calendar_month(raw: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(&format!("{}-01", raw.trim()), "%Y-%m-%d").ok()
}

fn skey_calendar_month(state: &GatewaySettingsState) -> NaiveDate {
    parse_calendar_month(&state.new_skey_calendar_month)
        .or_else(|| parse_skey_date(&state.new_skey_expires_at_input).map(month_start))
        .unwrap_or_else(|| month_start(Utc::now().date_naive()))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|first_next| first_next.pred_opt())
        .map(|date| date.day())
        .unwrap_or(31)
}

fn skey_calendar_popover(app: &App) -> Element<'_, Message> {
    let s = &app.gateway_settings;
    let month = skey_calendar_month(s);
    let selected_date = parse_skey_date(&s.new_skey_expires_at_input);

    let header = row![
        button(text("<"))
            .padding([6, 10])
            .on_press(Message::Settings(SettingsMessage::Gateway(
                GatewayMessage::NewSkeyExpiresMonthChanged(-1)
            )))
            .style(rounded_action_btn_style),
        text(month.format("%Y-%m").to_string())
            .size(13)
            .width(Length::Fill)
            .align_x(Alignment::Center),
        button(text(">"))
            .padding([6, 10])
            .on_press(Message::Settings(SettingsMessage::Gateway(
                GatewayMessage::NewSkeyExpiresMonthChanged(1)
            )))
            .style(rounded_action_btn_style),
        button(text(connection_text(app.desktop_locale, "永不过期")))
            .padding([6, 12])
            .on_press(Message::Settings(SettingsMessage::Gateway(
                GatewayMessage::NewSkeyExpiresAtCleared
            )))
            .style(rounded_action_btn_style),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    let weekdays = [
        connection_text(app.desktop_locale, "一"),
        connection_text(app.desktop_locale, "二"),
        connection_text(app.desktop_locale, "三"),
        connection_text(app.desktop_locale, "四"),
        connection_text(app.desktop_locale, "五"),
        connection_text(app.desktop_locale, "六"),
        connection_text(app.desktop_locale, "日"),
    ]
    .into_iter()
    .fold(row![].spacing(6), |row, label| {
        row.push(
            container(text(label).size(11).style(settings_muted_text_style))
                .width(Length::Fixed(GATEWAY_SKEY_CALENDAR_CELL_SIZE))
                .center_x(Length::Fixed(GATEWAY_SKEY_CALENDAR_CELL_SIZE)),
        )
    });

    let first_weekday = month.weekday().num_days_from_monday() as i32;
    let days = days_in_month(month.year(), month.month()) as i32;
    let mut calendar = column![header, weekdays].spacing(8);
    for week in 0..(first_weekday + days + 6) / 7 {
        let mut days_row = row![].spacing(6);
        for weekday in 0..7 {
            let day = week * 7 + weekday - first_weekday + 1;
            if day < 1 || day > days {
                days_row = days_row.push(
                    container(text(""))
                        .width(Length::Fixed(GATEWAY_SKEY_CALENDAR_CELL_SIZE))
                        .height(Length::Fixed(GATEWAY_SKEY_CALENDAR_CELL_SIZE)),
                );
                continue;
            }

            let date =
                NaiveDate::from_ymd_opt(month.year(), month.month(), day as u32).unwrap_or(month);
            let is_selected = selected_date == Some(date);
            days_row = days_row.push(
                button(
                    text(day.to_string())
                        .size(12)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center(),
                )
                .padding(0)
                .width(Length::Fixed(GATEWAY_SKEY_CALENDAR_CELL_SIZE))
                .height(Length::Fixed(GATEWAY_SKEY_CALENDAR_CELL_SIZE))
                .on_press(Message::Settings(SettingsMessage::Gateway(
                    GatewayMessage::NewSkeyExpiresDateSelected(date.format("%Y-%m-%d").to_string()),
                )))
                .style(move |theme, status| {
                    if is_selected {
                        primary_action_btn_style(theme, status)
                    } else {
                        rounded_action_btn_style(theme, status)
                    }
                }),
            );
        }
        calendar = calendar.push(days_row);
    }

    settings_group_card(calendar.align_x(Alignment::Center))
        .padding([14, 18])
        .width(Length::Fixed(340.0))
        .into()
}

fn skey_date_picker(app: &App) -> Element<'_, Message> {
    let s = &app.gateway_settings;
    let selected_date = parse_skey_date(&s.new_skey_expires_at_input);
    let action_label = if s.new_skey_calendar_open {
        connection_text(app.desktop_locale, "收起")
    } else {
        connection_text(app.desktop_locale, "日历")
    };

    let mut trigger = row![
        settings_text_input(
            connection_text(app.desktop_locale, "YYYY-MM-DD，可手写"),
            &s.new_skey_expires_at_input
        )
        .on_input(|value| Message::Settings(SettingsMessage::Gateway(
            GatewayMessage::NewSkeyExpiresAtChanged(value)
        )))
        .width(Length::Fill),
        button(text(action_label).size(13))
            .padding([10, 12])
            .on_press(Message::Settings(SettingsMessage::Gateway(
                GatewayMessage::NewSkeyCalendarToggled
            )))
            .style(rounded_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    if selected_date.is_some() {
        trigger = trigger.push(
            button(text(connection_text(app.desktop_locale, "永不过期")).size(13))
                .padding([10, 12])
                .on_press(Message::Settings(SettingsMessage::Gateway(
                    GatewayMessage::NewSkeyExpiresAtCleared,
                )))
                .style(rounded_action_btn_style),
        );
    }

    BelowOverlay::new(trigger, skey_calendar_popover(app))
        .show(s.new_skey_calendar_open)
        .gap(6.0)
        .on_close(Message::Settings(SettingsMessage::Gateway(
            GatewayMessage::NewSkeyCalendarClosed,
        )))
        .into()
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
    let s = &app.gateway_settings;

    let help_btn = settings_help_button_with_locale(
        app.desktop_locale,
        Message::Settings(SettingsMessage::Gateway(GatewayMessage::HelpOpen)),
    );

    let port_row = number_row(
        connection_text(app.desktop_locale, "端口"),
        connection_text(app.desktop_locale, "控制服务端 Gateway 的监听端口。"),
        s.port as u32,
        1,
        u16::MAX as u32,
        "",
        |value| {
            Message::Settings(SettingsMessage::Gateway(GatewayMessage::PortChanged(value as u16)))
        },
    );
    let host_row = text_row_with_hint(
        connection_text(app.desktop_locale, "主机地址"),
        connection_text(app.desktop_locale, "控制服务端 Gateway 的监听地址。"),
        "127.0.0.1",
        &s.host_input,
        connection_text(
            app.desktop_locale,
            "若 host 非本地地址，建议仅在可信反向代理或隧道环境下使用。",
        ),
        |value| Message::Settings(SettingsMessage::Gateway(GatewayMessage::HostChanged(value))),
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
            Message::Settings(SettingsMessage::Gateway(GatewayMessage::AuthEnabledToggled(value)))
        },
    );
    let allow_public_bind_row = bool_row(
        connection_text(app.desktop_locale, "允许公网绑定"),
        connection_text(app.desktop_locale, "允许绑定到非本地地址。"),
        s.allow_public_bind,
        connection_text(app.desktop_locale, "允许绑定到非本地地址"),
        |value| {
            Message::Settings(SettingsMessage::Gateway(GatewayMessage::AllowPublicBindToggled(
                value,
            )))
        },
    );
    let trust_forwarded_headers_row = bool_row_with_hint(
        connection_text(app.desktop_locale, "信任转发头"),
        connection_text(app.desktop_locale, "仅在可信反向代理后开启。"),
        s.trust_forwarded_headers,
        connection_text(app.desktop_locale, "信任 X-Forwarded-For / X-Real-IP"),
        connection_text(app.desktop_locale, "仅在你完全信任前置代理时开启 forwarded headers。"),
        |value| {
            Message::Settings(SettingsMessage::Gateway(
                GatewayMessage::TrustForwardedHeadersToggled(value),
            ))
        },
    );

    let skeys_section = settings_group_card(
        column![
            skey_field(
                connection_text(app.desktop_locale, "skey 名称"),
                connection_text(app.desktop_locale, "用于列表展示和识别该 skey。"),
                row![
                    settings_text_input(
                        connection_text(app.desktop_locale, "例如 iPhone / CI Runner"),
                        &s.new_skey_name_input
                    )
                    .on_input(|value| Message::Settings(SettingsMessage::Gateway(
                        GatewayMessage::NewSkeyNameChanged(value)
                    )))
                    .width(Length::Fill),
                    button(text(connection_text(app.desktop_locale, "生成 skey")).size(13))
                        .padding([10, 14])
                        .on_press(Message::Settings(SettingsMessage::Gateway(
                            GatewayMessage::AddSkey
                        )))
                        .style(primary_action_btn_style),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
            ),
            settings_divider(),
            skey_field(
                connection_text(app.desktop_locale, "过期日期"),
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
            connection_text(app.desktop_locale, "原始 skey"),
            connection_text(app.desktop_locale, "仅展示一次，请立即复制。"),
            row![
                text(raw_skey.as_str())
                    .size(13)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                    .width(Length::Fill),
                button(text(copy_label).size(13))
                    .padding([8, 12])
                    .on_press(Message::Settings(SettingsMessage::Gateway(
                        GatewayMessage::CopyLastCreatedSkey
                    )))
                    .style(rounded_action_btn_style),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        ))
    });

    let skeys_list: Element<'_, Message> = if s.skeys.is_empty() {
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
                                    Message::Settings(SettingsMessage::Gateway(
                                        GatewayMessage::SkeyEnabledToggled(index, value),
                                    ))
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
                                .on_press(Message::Settings(SettingsMessage::Gateway(
                                    GatewayMessage::RemoveSkey(index),
                                )))
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
        connection_text(app.desktop_locale, "Webhook速率限制/分钟"),
        connection_text(app.desktop_locale, "限制 /webhook 每分钟可接受的请求数。"),
        s.webhook_rate_limit_per_minute,
        1,
        100_000,
        connection_text(app.desktop_locale, "次/分钟"),
        |value| {
            Message::Settings(SettingsMessage::Gateway(
                GatewayMessage::WebhookRateLimitPerMinuteChanged(value),
            ))
        },
    );
    let rate_limit_max_keys_row = number_row(
        connection_text(app.desktop_locale, "速率限制最大键数"),
        connection_text(app.desktop_locale, "约束网关内部 key map 规模。"),
        s.rate_limit_max_keys,
        1,
        100_000,
        connection_text(app.desktop_locale, "个键"),
        |value| {
            Message::Settings(SettingsMessage::Gateway(GatewayMessage::RateLimitMaxKeysChanged(
                value,
            )))
        },
    );

    let idempotency_ttl_row = number_row(
        connection_text(app.desktop_locale, "幂等性TTL(秒)"),
        connection_text(app.desktop_locale, "控制 webhook 幂等 key 的存活时间。"),
        s.idempotency_ttl_secs,
        1,
        86_400,
        connection_text(app.desktop_locale, "秒"),
        |value| {
            Message::Settings(SettingsMessage::Gateway(GatewayMessage::IdempotencyTtlSecsChanged(
                value,
            )))
        },
    );
    let idempotency_max_keys_row = number_row(
        connection_text(app.desktop_locale, "幂等性最大键数"),
        connection_text(app.desktop_locale, "控制幂等缓存允许保留的 key 数量。"),
        s.idempotency_max_keys,
        1,
        100_000,
        connection_text(app.desktop_locale, "个键"),
        |value| {
            Message::Settings(SettingsMessage::Gateway(GatewayMessage::IdempotencyMaxKeysChanged(
                value,
            )))
        },
    );

    let node_control_enabled_row = bool_row(
        connection_text(app.desktop_locale, "节点控制.已启用"),
        connection_text(app.desktop_locale, "启用实验性的 node-control API。"),
        s.node_control_enabled,
        connection_text(app.desktop_locale, "启用实验性的 node-control API"),
        |value| {
            Message::Settings(SettingsMessage::Gateway(GatewayMessage::NodeControlEnabledToggled(
                value,
            )))
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
            Message::Settings(SettingsMessage::Gateway(
                GatewayMessage::NodeControlAuthTokenChanged(value),
            ))
        },
    );
    let node_control_allowed_ids_row = text_row_with_hint(
        connection_text(app.desktop_locale, "节点控制.允许的节点ID"),
        connection_text(
            app.desktop_locale,
            "留空表示不设置显式 allowlist；可使用 * 允许所有节点。",
        ),
        connection_text(app.desktop_locale, "逗号或换行分隔，支持 *"),
        &s.node_control_allowed_node_ids_input,
        connection_text(
            app.desktop_locale,
            "allowed_node_ids 留空表示不设置显式 allowlist；可使用 * 允许所有节点。",
        ),
        |value| {
            Message::Settings(SettingsMessage::Gateway(
                GatewayMessage::NodeControlAllowedNodeIdsChanged(value),
            ))
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
            Some(Message::Settings(SettingsMessage::Gateway(GatewayMessage::TabSelected(*tab)))),
        )
        .into()
    }))
    .wrap();

    let config_content: Element<'_, Message> = column![
        settings_group_heading(
            connection_text(app.desktop_locale, "服务管理"),
            connection_text(
                app.desktop_locale,
                "通过本机 gateway 调用 shiju service 管理 OS 服务生命周期。"
            )
        ),
        service_controls_content(app),
        settings_group_heading(
            connection_text(app.desktop_locale, "监听地址"),
            connection_text(
                app.desktop_locale,
                "控制网关监听 host / port。默认仅绑定回环地址，避免意外暴露到公网。"
            )
        ),
        settings_group_card(column![port_row, settings_divider(), host_row,].spacing(0)),
        settings_group_heading(
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
        settings_group_heading(
            connection_text(app.desktop_locale, "速率限制"),
            connection_text(
                app.desktop_locale,
                "限制 /webhook 请求频率，并约束网关内部 key map 规模。"
            )
        ),
        settings_group_card(
            column![webhook_rate_row, settings_divider(), rate_limit_max_keys_row].spacing(0),
        ),
        settings_group_heading(
            connection_text(app.desktop_locale, "幂等性"),
            connection_text(app.desktop_locale, "控制 webhook 幂等 key 的存活时间与内存上限。")
        ),
        settings_group_card(
            column![idempotency_ttl_row, settings_divider(), idempotency_max_keys_row].spacing(0)
        ),
        settings_group_heading(
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

    let skeys_content: Element<'_, Message> = column![
        settings_group_heading(
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
            container(settings_page_heading(
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
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.gateway_settings;
    if let Some(command) = s.pending_service_command_confirmation.as_deref() {
        let label = service_display_label(app.desktop_locale, command);
        let command_text = service_cli_command(command);
        let body = owned_text(
            app.desktop_locale,
            "%s 是危险操作，可能会中断当前本机 gateway 连接、停止后台任务，卸载服务还会移除开机自启动配置。\n\n将执行命令：%s\n\n确认继续？",
            &[label.to_string(), command_text.clone()],
        );
        let cancel_message =
            Message::Settings(SettingsMessage::Gateway(GatewayMessage::CancelDangerServiceCommand));
        let confirm_message = Message::Settings(SettingsMessage::Gateway(
            GatewayMessage::ConfirmDangerServiceCommand(command.to_string()),
        ));
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

    let help_text = crate::app::i18n::settings_models::help::gateway(app.desktop_locale);

    crate::app::components::system_settings_common::with_settings_help_modal(
        app,
        dialog,
        connection_text(app.desktop_locale, "Gateway 配置帮助"),
        help_text,
        Message::Settings(SettingsMessage::Gateway(GatewayMessage::HelpClose)),
    )
}
#[cfg(test)]
#[path = "system_settings_gateway_tests.rs"]
mod system_settings_gateway_tests;

fn service_display_label(
    locale: vw_config_types::ui::DesktopLocale,
    command: &str,
) -> &'static str {
    let key = match command {
        "stop" => "停止服务",
        "uninstall" => "卸载服务",
        "install" => "安装服务",
        "start" => "启动服务",
        "restart" => "重启服务",
        "status" => "查询状态",
        _ => "服务命令",
    };
    connection_text(locale, key)
}

#[cfg(test)]
#[path = "system_settings_gateway_ui_refresh_23_tests.rs"]
mod ui_refresh_23_tests;
