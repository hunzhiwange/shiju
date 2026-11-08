//! 系统设置中 http request 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::number_input::NumberInput;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_checkbox_style, settings_control_with_hint,
    settings_divider, settings_group_card, settings_group_heading, settings_muted_text_style,
    settings_page_heading, settings_row_with_flexible_control, settings_text_input_style,
    settings_value_badge,
};
use crate::app::i18n::settings_extensions::{domain_label, text as tr};
use crate::app::message::settings::{HttpRequestMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, row, text, text::Wrapping, text_input};
use iced::{Alignment, Element, Length};

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_row_with_flexible_control(label, description, 360.0, control)
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

fn number_display(value: u32, suffix: &str) -> String {
    if suffix.is_empty() { value.to_string() } else { format!("{value} {suffix}") }
}

fn normalize_number_value(raw: f32, min: u32, max: u32) -> u32 {
    raw.round().clamp(min as f32, max as f32) as u32
}

fn number_row<'a>(
    label: &'static str,
    description: &'static str,
    value: u32,
    min: u32,
    max: u32,
    suffix: &'static str,
    hint: Option<&'static str>,
    on_change: impl Fn(u32) -> Message + 'a,
) -> Element<'a, Message> {
    let display = number_display(value, suffix);
    let control = row![
        NumberInput::new(value as f32, min as f32, max as f32, 1.0, 0, 0.15, move |raw| {
            on_change(normalize_number_value(raw, min, max))
        })
        .settings_style(),
        settings_value_badge(display),
    ]
    .spacing(16)
    .align_y(Alignment::Center);
    let control: Element<'_, Message> = if let Some(hint) = hint {
        settings_control_with_hint(control, hint)
    } else {
        control.into()
    };

    field_row(label, description, control)
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
    column![
        settings_page_heading(
            tr(app.desktop_locale, "网络请求配置"),
            tr(app.desktop_locale, "配置网络请求工具的开关、超时、响应大小限制和域名白名单。"),
        ),
        content(app),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

pub(super) fn content(app: &App) -> Element<'_, Message> {
    let s = &app.http_request_settings;

    let enabled_row = bool_row(
        tr(app.desktop_locale, "启用"),
        tr(app.desktop_locale, "控制是否启用网络请求工具。"),
        s.enabled,
        tr(app.desktop_locale, "启用网络请求工具"),
        |value| {
            Message::Settings(SettingsMessage::HttpRequest(HttpRequestMessage::EnabledToggled(
                value,
            )))
        },
    );

    let max_response_size_row = number_row(
        tr(app.desktop_locale, "响应大小上限"),
        tr(app.desktop_locale, "限制单次请求可读取的最大响应体大小。"),
        s.max_response_size,
        0,
        u32::MAX,
        tr(app.desktop_locale, "字节"),
        Some(tr(
            app.desktop_locale,
            "`0` 表示不限制响应体大小；较大的响应会增加内存占用与模型上下文成本。",
        )),
        |value| {
            Message::Settings(SettingsMessage::HttpRequest(
                HttpRequestMessage::MaxResponseSizeChanged(value),
            ))
        },
    );

    let timeout_secs_row = number_row(
        tr(app.desktop_locale, "超时时间"),
        tr(app.desktop_locale, "单次请求的超时时间。"),
        s.timeout_secs,
        0,
        86_400,
        tr(app.desktop_locale, "秒"),
        Some(tr(app.desktop_locale, "`0` 会在运行时回退到安全默认值 30 秒。")),
        |value| {
            Message::Settings(SettingsMessage::HttpRequest(HttpRequestMessage::TimeoutSecsChanged(
                value,
            )))
        },
    );

    let user_agent_row = text_row(
        "User-Agent",
        tr(app.desktop_locale, "请求默认携带的 User-Agent。"),
        vw_config_types::tools::DEFAULT_USER_AGENT,
        &s.user_agent,
        |value| {
            Message::Settings(SettingsMessage::HttpRequest(HttpRequestMessage::UserAgentChanged(
                value,
            )))
        },
    );

    let add_domain_row = field_row(
        tr(app.desktop_locale, "新增域名"),
        tr(app.desktop_locale, "支持精确域名、子域模式或 *。"),
        settings_control_with_hint(
            row![
                text_input(
                    tr(app.desktop_locale, "例如：example.com 或 *.example.org 或 *"),
                    &s.new_allowed_domain_input
                )
                .on_input(|value| Message::Settings(SettingsMessage::HttpRequest(
                    HttpRequestMessage::NewAllowedDomainChanged(value),
                )))
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_text_input_style)
                .width(Length::Fill),
                button(text(tr(app.desktop_locale, "添加")).size(SETTINGS_CONTROL_TEXT_SIZE))
                    .padding([8, 14])
                    .on_press(Message::Settings(SettingsMessage::HttpRequest(
                        HttpRequestMessage::AddAllowedDomain,
                    )))
                    .style(primary_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            tr(app.desktop_locale, "支持精确域名和子域模式。默认的 `*` 允许所有公网域名。"),
        ),
    );
    let mut allowed_domains = column![add_domain_row, settings_divider()];
    if s.allowed_domains.is_empty() {
        allowed_domains = allowed_domains.push(
            container(
                text(tr(
                    app.desktop_locale,
                    "当前未配置允许域名；启用工具后仍会因为白名单为空而拒绝所有请求。",
                ))
                .size(13)
                .style(settings_muted_text_style),
            )
            .padding([18, 0]),
        );
    } else {
        for (index, domain) in s.allowed_domains.iter().enumerate() {
            if index != 0 {
                allowed_domains = allowed_domains.push(settings_divider());
            }
            allowed_domains = allowed_domains.push(
                container(
                    row![
                        text(domain_label(app.desktop_locale, index + 1))
                            .size(13)
                            .style(settings_muted_text_style),
                        text(domain)
                            .size(SETTINGS_CONTROL_TEXT_SIZE)
                            .width(Length::Fill)
                            .wrapping(Wrapping::WordOrGlyph),
                        button(
                            text(tr(app.desktop_locale, "删除")).size(SETTINGS_CONTROL_TEXT_SIZE)
                        )
                        .padding([8, 14])
                        .on_press(Message::Settings(SettingsMessage::HttpRequest(
                            HttpRequestMessage::RemoveAllowedDomain(index),
                        )))
                        .style(rounded_action_btn_style),
                    ]
                    .spacing(16)
                    .align_y(Alignment::Center),
                )
                .padding([12, 0]),
            );
        }
    }

    let content = column![
        settings_group_heading(
            tr(app.desktop_locale, "基础行为"),
            tr(
                app.desktop_locale,
                "控制 http_request 工具的启用状态、超时、最大响应体大小和默认 User-Agent。"
            ),
        ),
        settings_group_card(column![
            enabled_row,
            settings_divider(),
            max_response_size_row,
            settings_divider(),
            timeout_secs_row,
            settings_divider(),
            user_agent_row,
        ]),
        settings_group_heading(
            tr(app.desktop_locale, "允许域名"),
            tr(
                app.desktop_locale,
                "维护 http_request.allowed_domains 白名单。列表为空时默认拒绝所有外部请求。"
            ),
        ),
        settings_group_card(allowed_domains),
    ]
    .spacing(16)
    .width(Length::Fill);

    container(content).width(Length::Fill).into()
}
#[cfg(test)]
#[path = "system_settings_http_request_tests.rs"]
mod system_settings_http_request_tests;

#[cfg(test)]
#[path = "system_settings_http_request_ui_refresh_25_tests.rs"]
mod ui_refresh_25_tests;
