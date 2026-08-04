//! 系统设置中 web search 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_divider, settings_help_button,
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

fn hint_row<'a>(message: &'a str) -> Element<'a, Message> {
    row![
        container(text("")).width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
        text(message).size(12).style(settings_muted_text_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .into()
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
    let s = &app.web_search_settings;

    let help_btn = settings_help_button(Message::Settings(SettingsMessage::WebSearch(
        WebSearchMessage::HelpOpen,
    )));

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

    let provider_row = field_row("搜索服务", "选择搜索提供方。", provider_pick);

    let mut content = column![
        row![
            container(settings_page_intro(
                "网页搜索配置",
                "配置网页搜索工具的提供方、鉴权信息和请求参数。"
            ))
            .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start),
        settings_section_card("基础行为", "控制开关与默认搜索服务。"),
        settings_panel(column![enabled_row, settings_divider(), provider_row].spacing(0))
    ]
    .spacing(16)
    .width(Length::Fill);

    if s.provider == "duckduckgo" {
        content = content.push(hint_row("DuckDuckGo 模式无需 API Key，会直接使用公开搜索页面。"));
    }

    if shows_brave_api_key(&s.provider) {
        content = content
            .push(text_row(
                "Brave 密钥",
                "仅 provider=brave 时生效。",
                "Brave Search API Key",
                &s.brave_api_key_input,
                |value| {
                    Message::Settings(SettingsMessage::WebSearch(
                        WebSearchMessage::BraveApiKeyChanged(value),
                    ))
                },
            ))
            .push(hint_row("Brave 优先使用 `brave_api_key`；留空时会回退到 `api_key`。"));
    }

    if shows_api_key(&s.provider) {
        let placeholder = if s.provider == "brave" {
            "可选，作为 Brave 的备用 API Key"
        } else {
            "搜索服务 API 密钥"
        };
        content = content.push(text_row(
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
        content = content
            .push(text_row(
                "接口地址",
                "可选，自定义兼容接口地址。",
                "可选，自定义搜索服务接口地址",
                &s.api_url_input,
                |value| {
                    Message::Settings(SettingsMessage::WebSearch(WebSearchMessage::ApiUrlChanged(
                        value,
                    )))
                },
            ))
            .push(hint_row("留空时使用默认接口；适合代理层或自托管兼容端点。"));
    }

    content = content
        .push(settings_section_card("请求参数", "控制返回数量、超时和请求头。"))
        .push(settings_panel(
            column![
                text_row(
                    "结果数量",
                    "最终会限制在 1-10 之间。",
                    "1-10",
                    &s.max_results_input,
                    |value| {
                        Message::Settings(SettingsMessage::WebSearch(
                            WebSearchMessage::MaxResultsChanged(value),
                        ))
                    }
                ),
                settings_divider(),
                text_row(
                    "超时时间",
                    "留空时回退到默认值 15 秒。",
                    "15",
                    &s.timeout_secs_input,
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
        ))
        .push(hint_row("仅接受正整数；最终会限制在 1-10 之间。"))
        .push(hint_row("仅接受正整数秒；留空时回退到默认值 15。"));

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
    let s = &app.web_search_settings;
    if !s.show_help_modal {
        return dialog;
    }

    let help_text = r#"网页搜索配置说明

一、作用
- web_search 用于让代理主动检索公开网页搜索结果。
- 当前支持 duckduckgo、brave、serper、google、bing。

二、字段含义
1) enabled
- 是否启用 web_search_tool。

2) provider
- duckduckgo：免 API Key，直接抓取 DuckDuckGo 公共 HTML 搜索结果。
- brave：使用 Brave Search API，优先读取 brave_api_key，留空时回退 api_key。
- serper：使用 Serper Google Search API。
- google：当前通过 Serper 的 Google 兼容端点执行。
- bing：当前通过 Serper 的 Bing 兼容端点执行。

3) api_key
- 通用 API Key。
- 对 serper / google / bing 必填。
- 对 brave 可作为 brave_api_key 的回退值。

4) brave_api_key
- Brave 专用 API Key。
- 仅 provider=brave 时使用。

5) api_url
- 可选，自定义兼容端点。
- 适用于代理网关、自托管兼容层或测试环境。

6) max_results
- 每次搜索返回的最大结果数。
- UI 接受正整数，最终限制在 1-10。

7) timeout_secs
- HTTP 请求超时秒数。
- 留空时回退默认值 15。

8) user_agent
- 搜索请求携带的 User-Agent。
- 留空时回退为桌面浏览器兼容的默认值。

三、示例
{
  "web_search": {
    "enabled": true,
    "provider": "serper",
    "api_key": "your-serper-key",
    "api_url": null,
    "brave_api_key": null,
    "max_results": 5,
    "timeout_secs": 15,
    "user_agent": "Mozilla/5.0 (...) Chrome/148.0.0.0 Safari/537.36"
  }
}
"#;

    crate::app::components::system_settings_common::with_settings_help_modal(
        app,
        dialog,
        "Web Search 配置帮助",
        help_text,
        Message::Settings(SettingsMessage::WebSearch(WebSearchMessage::HelpClose)),
    )
}
#[cfg(test)]
#[path = "system_settings_web_search_tests.rs"]
mod system_settings_web_search_tests;
