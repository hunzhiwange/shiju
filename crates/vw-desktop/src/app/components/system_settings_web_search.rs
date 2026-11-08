//! 系统设置中 web search 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    settings_checkbox_style, settings_control_with_hint, settings_divider,
    settings_flexible_row_with_help, settings_group_card, settings_group_heading_with_help,
    settings_muted_text_style, settings_page_heading_with_help, settings_select,
    settings_text_input,
};
use crate::app::i18n::I18n;
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::message::settings::{SettingsMessage, WebSearchMessage};
use crate::app::{App, Message};
use iced::widget::{checkbox, column, container, text};
use iced::{Element, Length};

fn field_row<'a>(
    app: &'a App,
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_flexible_row_with_help(
        I18n::new(app.desktop_locale),
        &app.app_theme,
        label,
        description,
        360.0,
        control,
    )
    .into()
}

fn bool_row<'a>(
    app: &'a App,
    label: &'static str,
    description: &'static str,
    checked: bool,
    checkbox_label: &'static str,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        app,
        label,
        description,
        checkbox(checked).label(checkbox_label).on_toggle(on_toggle).style(settings_checkbox_style),
    )
}

fn text_row<'a>(
    app: &'a App,
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        app,
        label,
        description,
        settings_text_input(placeholder, value).on_input(on_input).width(Length::Fill),
    )
}

fn text_row_with_hint<'a>(
    app: &'a App,
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    hint: &'static str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        app,
        label,
        description,
        settings_control_with_hint(
            settings_text_input(placeholder, value).on_input(on_input).width(Length::Fill),
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
    let i18n = I18n::new(app.desktop_locale);
    column![
        settings_page_heading_with_help(
            i18n,
            &app.app_theme,
            tr(app.desktop_locale, "网页搜索配置"),
            tr(app.desktop_locale, "配置网页搜索工具的提供方、鉴权信息和请求参数。"),
        ),
        content(app),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

pub(super) fn content(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let s = &app.web_search_settings;

    let enabled_row = bool_row(
        app,
        tr(app.desktop_locale, "启用"),
        tr(app.desktop_locale, "控制是否启用网页搜索工具。"),
        s.enabled,
        tr(app.desktop_locale, "启用网页搜索工具"),
        |value| {
            Message::Settings(SettingsMessage::WebSearch(WebSearchMessage::EnabledToggled(value)))
        },
    );

    let provider_pick = settings_select(
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
    .width(Length::Fixed(220.0));

    let provider_row = field_row(
        app,
        tr(app.desktop_locale, "搜索后端"),
        tr(app.desktop_locale, "选择网页搜索使用的后端。"),
        provider_pick,
    );

    let provider_section: Element<'_, Message> = if s.provider == "duckduckgo" {
        column![
            settings_group_heading_with_help(
                i18n,
                &app.app_theme,
                "DuckDuckGo",
                tr(app.desktop_locale, "公开搜索页面模式，无需额外鉴权配置。"),
            ),
            settings_group_card(column![
                container(
                    text(tr(
                        app.desktop_locale,
                        "当前后端会直接使用 DuckDuckGo 公开搜索页面，无需 API Key。"
                    ))
                    .size(12)
                    .style(settings_muted_text_style)
                )
                .padding([18, 0])
            ]),
        ]
        .spacing(16)
        .into()
    } else {
        let (title, description) = if s.provider == "brave" {
            ("Brave Search", tr(app.desktop_locale, "配置专用密钥、备用密钥和兼容接口地址。"))
        } else {
            (
                tr(app.desktop_locale, "后端连接"),
                tr(app.desktop_locale, "配置当前搜索后端的鉴权信息和兼容接口地址。"),
            )
        };
        let placeholder = if s.provider == "brave" {
            tr(app.desktop_locale, "可选，作为 Brave 的备用 API Key")
        } else {
            tr(app.desktop_locale, "搜索服务 API 密钥")
        };
        let mut fields = column![];

        if shows_brave_api_key(&s.provider) {
            fields = fields
                .push(field_row(
                    app,
                    tr(app.desktop_locale, "Brave 密钥"),
                    tr(app.desktop_locale, "仅 provider=brave 时生效。"),
                    settings_control_with_hint(
                        settings_text_input("Brave Search API Key", &s.brave_api_key_input)
                            .secure(true)
                            .on_input(|value| {
                                Message::Settings(SettingsMessage::WebSearch(
                                    WebSearchMessage::BraveApiKeyChanged(value),
                                ))
                            })
                            .width(Length::Fill),
                        tr(
                            app.desktop_locale,
                            "Brave 优先使用 `brave_api_key`；留空时会回退到 `api_key`。",
                        ),
                    ),
                ))
                .push(settings_divider());
        }

        if shows_api_key(&s.provider) {
            fields = fields.push(field_row(
                app,
                tr(app.desktop_locale, "API 密钥"),
                tr(app.desktop_locale, "搜索服务请求所需的认证密钥。"),
                settings_text_input(placeholder, &s.api_key_input)
                    .secure(true)
                    .on_input(|value| {
                        Message::Settings(SettingsMessage::WebSearch(
                            WebSearchMessage::ApiKeyChanged(value),
                        ))
                    })
                    .width(Length::Fill),
            ));
        }

        if shows_api_url(&s.provider) {
            fields = fields.push(settings_divider()).push(text_row_with_hint(
                app,
                tr(app.desktop_locale, "接口地址"),
                tr(app.desktop_locale, "可选，自定义兼容接口地址。"),
                tr(app.desktop_locale, "可选，自定义搜索服务接口地址"),
                &s.api_url_input,
                tr(app.desktop_locale, "留空时使用默认接口；适合代理层或自托管兼容端点。"),
                |value| {
                    Message::Settings(SettingsMessage::WebSearch(WebSearchMessage::ApiUrlChanged(
                        value,
                    )))
                },
            ));
        }

        column![
            settings_group_heading_with_help(i18n, &app.app_theme, title, description),
            settings_group_card(fields.spacing(0))
        ]
        .spacing(16)
        .into()
    };

    let content = column![
        settings_group_heading_with_help(
            i18n,
            &app.app_theme,
            tr(app.desktop_locale, "基础行为"),
            tr(app.desktop_locale, "控制开关与默认搜索后端。")
        ),
        settings_group_card(column![enabled_row, settings_divider(), provider_row].spacing(0)),
        provider_section,
    ]
    .spacing(16)
    .width(Length::Fill)
    .push(settings_group_heading_with_help(
        i18n,
        &app.app_theme,
        tr(app.desktop_locale, "请求参数"),
        tr(app.desktop_locale, "控制返回数量、超时和请求头。"),
    ))
    .push(settings_group_card(
        column![
            text_row_with_hint(
                app,
                tr(app.desktop_locale, "结果数量"),
                tr(app.desktop_locale, "最终会限制在 1-10 之间。"),
                "1-10",
                &s.max_results_input,
                tr(app.desktop_locale, "仅接受正整数；最终会限制在 1-10 之间。"),
                |value| {
                    Message::Settings(SettingsMessage::WebSearch(
                        WebSearchMessage::MaxResultsChanged(value),
                    ))
                }
            ),
            settings_divider(),
            text_row_with_hint(
                app,
                tr(app.desktop_locale, "超时时间"),
                tr(app.desktop_locale, "留空时回退到默认值 15 秒。"),
                "15",
                &s.timeout_secs_input,
                tr(app.desktop_locale, "仅接受正整数秒；留空时回退到默认值 15。"),
                |value| {
                    Message::Settings(SettingsMessage::WebSearch(
                        WebSearchMessage::TimeoutSecsChanged(value),
                    ))
                }
            ),
            settings_divider(),
            text_row(
                app,
                "User-Agent",
                tr(app.desktop_locale, "搜索请求携带的 User-Agent。"),
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
