//! 系统设置中 web search 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_control_with_hint, settings_divider,
    settings_muted_text_style, settings_page_intro, settings_panel, settings_pick_list_menu_style,
    settings_pick_list_style, settings_section_card, settings_text_input_style,
};
use crate::app::message::settings::{SettingsMessage, WebSearchMessage};
use crate::app::{App, Message};
use iced::widget::{checkbox, column, container, pick_list, row, text, text_input};
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
            text_input(placeholder, value)
                .on_input(on_input)
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill),
            hint,
        ),
    )
}

fn shows_api_key(provider: &str) -> bool {
    !matches!(provider, "duckduckgo")
}

fn shows_api_url(provider: &str) -> bool {
    !matches!(provider, "duckduckgo")
}

fn shows_brave_api_key(provider: &str) -> bool {
    matches!(provider, "brave")
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
        settings_page_intro("网页搜索配置", "配置网页搜索工具的提供方、鉴权信息和请求参数。",),
        content(app),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

pub(super) fn content(app: &App) -> Element<'_, Message> {
    let s = &app.web_search_settings;

    let enabled_row = bool_row(
        "启用",
        "控制是否启用网页搜索工具。",
        s.enabled,
        "启用网页搜索工具",
        |value| {
            Message::Settings(SettingsMessage::WebSearch(WebSearchMessage::EnabledToggled(value)))
        },
    );

    let provider_pick = pick_list(
        [
            "duckduckgo".to_string(),
            "brave".to_string(),
            "serper".to_string(),
            "google".to_string(),
            "bing".to_string(),
        ],
        Some(s.provider.clone()),
        |value| {
            Message::Settings(SettingsMessage::WebSearch(WebSearchMessage::ProviderChanged(value)))
        },
    )
    .padding([10, 14])
    .text_size(13)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(220.0));

    let provider_row = field_row("搜索后端", "选择网页搜索使用的后端。", provider_pick);

    let provider_section: Element<'_, Message> = if s.provider == "duckduckgo" {
        column![
            settings_section_card("DuckDuckGo", "公开搜索页面模式，无需额外鉴权配置。",),
            settings_panel(column![
                text("当前后端会直接使用 DuckDuckGo 公开搜索页面，无需 API Key。")
                    .size(12)
                    .style(settings_muted_text_style)
            ]),
        ]
        .spacing(16)
        .into()
    } else {
        let (title, description) = if s.provider == "brave" {
            ("Brave Search", "配置专用密钥、备用密钥和兼容接口地址。")
        } else {
            ("后端连接", "配置当前搜索后端的鉴权信息和兼容接口地址。")
        };
        let placeholder = if s.provider == "brave" {
            "可选，作为 Brave 的备用 API Key"
        } else {
            "搜索服务 API 密钥"
        };
        let mut fields = column![];

        if shows_brave_api_key(&s.provider) {
            fields = fields
                .push(text_row_with_hint(
                    "Brave 密钥",
                    "仅 provider=brave 时生效。",
                    "Brave Search API Key",
                    &s.brave_api_key_input,
                    "Brave 优先使用 `brave_api_key`；留空时会回退到 `api_key`。",
                    |value| {
                        Message::Settings(SettingsMessage::WebSearch(
                            WebSearchMessage::BraveApiKeyChanged(value),
                        ))
                    },
                ))
                .push(settings_divider());
        }

        if shows_api_key(&s.provider) {
            fields = fields.push(text_row(
                "API 密钥",
                "搜索服务请求所需的认证密钥。",
                placeholder,
                &s.api_key_input,
                |value| {
                    Message::Settings(SettingsMessage::WebSearch(WebSearchMessage::ApiKeyChanged(
                        value,
                    )))
                },
            ));
        }

        if shows_api_url(&s.provider) {
            fields = fields.push(settings_divider()).push(text_row_with_hint(
                "接口地址",
                "可选，自定义兼容接口地址。",
                "可选，自定义搜索服务接口地址",
                &s.api_url_input,
                "留空时使用默认接口；适合代理层或自托管兼容端点。",
                |value| {
                    Message::Settings(SettingsMessage::WebSearch(WebSearchMessage::ApiUrlChanged(
                        value,
                    )))
                },
            ));
        }

        column![settings_section_card(title, description), settings_panel(fields.spacing(0))]
            .spacing(16)
            .into()
    };

    let content = column![
        settings_section_card("基础行为", "控制开关与默认搜索后端。"),
        settings_panel(column![enabled_row, settings_divider(), provider_row].spacing(0)),
        provider_section,
    ]
    .spacing(16)
    .width(Length::Fill)
    .push(settings_section_card("请求参数", "控制返回数量、超时和请求头。"))
    .push(settings_panel(
        column![
            text_row_with_hint(
                "结果数量",
                "最终会限制在 1-10 之间。",
                "1-10",
                &s.max_results_input,
                "仅接受正整数；最终会限制在 1-10 之间。",
                |value| {
                    Message::Settings(SettingsMessage::WebSearch(
                        WebSearchMessage::MaxResultsChanged(value),
                    ))
                }
            ),
            settings_divider(),
            text_row_with_hint(
                "超时时间",
                "留空时回退到默认值 15 秒。",
                "15",
                &s.timeout_secs_input,
                "仅接受正整数秒；留空时回退到默认值 15。",
                |value| {
                    Message::Settings(SettingsMessage::WebSearch(
                        WebSearchMessage::TimeoutSecsChanged(value),
                    ))
                }
            ),
            settings_divider(),
            text_row(
                "User-Agent",
                "搜索请求携带的 User-Agent。",
                vw_config_types::tools::DEFAULT_USER_AGENT,
                &s.user_agent,
                |value| {
                    Message::Settings(SettingsMessage::WebSearch(
                        WebSearchMessage::UserAgentChanged(value),
                    ))
                }
            )
        ]
        .spacing(0),
    ));

    content.into()
}

#[cfg(test)]
#[path = "system_settings_web_search_tests.rs"]
mod system_settings_web_search_tests;
