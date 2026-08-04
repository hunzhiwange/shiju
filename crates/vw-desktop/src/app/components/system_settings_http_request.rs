//! 系统设置中 http request 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::number_input::NumberInput;
use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, rounded_action_btn_style, settings_checkbox_style,
    settings_muted_text_style, settings_page_intro, settings_panel, settings_section_card,
    settings_text_input_style, settings_value_badge,
};
use crate::app::message::settings::{HttpRequestMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

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
            .padding([10, 12])
            .size(13)
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

fn hint_row<'a>(message: &'a str) -> Element<'a, Message> {
    row![
        container(text("")).width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
        text(message).size(12).style(settings_muted_text_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .into()
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
    on_change: impl Fn(u32) -> Message + 'a,
) -> Element<'a, Message> {
    let display = number_display(value, suffix);

    field_row(
        label,
        description,
        row![
            NumberInput::new(value as f32, min as f32, max as f32, 1.0, 0, 0.15, move |raw| {
                on_change(normalize_number_value(raw, min, max))
            })
            .settings_style(),
            settings_value_badge(display),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    )
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
    let s = &app.http_request_settings;

    let enabled_row = bool_row(
        "启用",
        "控制是否启用网络请求工具。",
        s.enabled,
        "启用网络请求工具",
        |value| {
            Message::Settings(SettingsMessage::HttpRequest(HttpRequestMessage::EnabledToggled(
                value,
            )))
        },
    );

    let max_response_size_row = number_row(
        "响应大小上限",
        "限制单次请求可读取的最大响应体大小。",
        s.max_response_size,
        0,
        u32::MAX,
        "字节",
        |value| {
            Message::Settings(SettingsMessage::HttpRequest(
                HttpRequestMessage::MaxResponseSizeChanged(value),
            ))
        },
    );

    let timeout_secs_row = number_row(
        "超时时间",
        "单次请求的超时时间。",
        s.timeout_secs,
        0,
        86_400,
        "秒",
        |value| {
            Message::Settings(SettingsMessage::HttpRequest(HttpRequestMessage::TimeoutSecsChanged(
                value,
            )))
        },
    );

    let user_agent_row = text_row(
        "User-Agent",
        "请求默认携带的 User-Agent。",
        vw_config_types::tools::DEFAULT_USER_AGENT,
        &s.user_agent,
        |value| {
            Message::Settings(SettingsMessage::HttpRequest(HttpRequestMessage::UserAgentChanged(
                value,
            )))
        },
    );

    let allowed_domains_section = settings_panel(
        column![field_row(
            "新增域名",
            "支持精确域名、子域模式或 *。",
            row![
                text_input("example.com 或 *.example.org 或 *", &s.new_allowed_domain_input)
                    .on_input(|value| Message::Settings(SettingsMessage::HttpRequest(
                        HttpRequestMessage::NewAllowedDomainChanged(value),
                    )))
                    .padding([10, 12])
                    .size(13)
                    .style(settings_text_input_style)
                    .width(Length::Fill),
                button(text("添加"))
                    .padding([6, 12])
                    .on_press(Message::Settings(SettingsMessage::HttpRequest(
                        HttpRequestMessage::AddAllowedDomain,
                    )))
                    .style(rounded_action_btn_style),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )]
        .spacing(0),
    );

    let allowed_domains_list: Element<'_, Message> = if s.allowed_domains.is_empty() {
        settings_panel(
            column![
                text("当前未配置允许域名；启用工具后仍会因为白名单为空而拒绝所有请求。")
                    .size(12)
                    .style(settings_muted_text_style)
            ]
            .spacing(0),
        )
        .into()
    } else {
        let list = s.allowed_domains.iter().enumerate().fold(
            column![].spacing(10),
            |column, (index, domain)| {
                column.push(
                    row![
                        text(format!("域名 {}", index + 1))
                            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
                        text(domain).width(Length::Fill),
                        button(text("删除"))
                            .padding([6, 12])
                            .on_press(Message::Settings(SettingsMessage::HttpRequest(
                                HttpRequestMessage::RemoveAllowedDomain(index),
                            )))
                            .style(rounded_action_btn_style),
                    ]
                    .spacing(20)
                    .align_y(Alignment::Center),
                )
            },
        );

        settings_panel(list).into()
    };

    let content = column![
        settings_page_intro(
            "网络请求配置",
            "配置网络请求工具的开关、超时、响应大小限制和域名白名单。"
        ),
        settings_section_card(
            "基础行为",
            "控制 http_request 工具的启用状态、超时、最大响应体大小和默认 User-Agent。",
        ),
        settings_panel(
            column![enabled_row, max_response_size_row, timeout_secs_row, user_agent_row]
                .spacing(0)
        ),
        hint_row("`0` 表示不限制响应体大小；较大的响应会增加内存占用与模型上下文成本。"),
        hint_row("`0` 会在运行时回退到安全默认值 30 秒。"),
        settings_section_card(
            "允许域名",
            "维护 http_request.allowed_domains 白名单。列表为空时默认拒绝所有外部请求。",
        ),
        allowed_domains_section,
        allowed_domains_list,
        hint_row("支持精确域名、子域模式或 `*`；建议只放行业务 API 域名。"),
    ]
    .spacing(16)
    .width(Length::Fill);

    container(content).width(Length::Fill).into()
}
#[cfg(test)]
#[path = "system_settings_http_request_tests.rs"]
mod system_settings_http_request_tests;
