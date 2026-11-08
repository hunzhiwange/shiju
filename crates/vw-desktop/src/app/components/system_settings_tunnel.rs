//! 系统设置中 tunnel 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    settings_checkbox_style, settings_divider, settings_flexible_row_with_help,
    settings_group_card, settings_group_heading, settings_group_heading_with_help,
    settings_muted_text_style, settings_page_heading, settings_row_with_flexible_control,
    settings_select, settings_text_input,
};
use crate::app::i18n::{I18n, settings_extensions::text as tr};
use crate::app::message::settings::{SettingsMessage, TunnelMessage};
use crate::app::{App, Message};
use iced::widget::{checkbox, column, container, text};
use iced::{Element, Length};

#[derive(Clone, Copy, PartialEq, Eq)]
struct LabeledOption {
    value: &'static str,
    label: &'static str,
}

impl std::fmt::Display for LabeledOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

const PROVIDER_OPTIONS: [LabeledOption; 5] = [
    LabeledOption { value: "none", label: "未启用" },
    LabeledOption { value: "cloudflare", label: "Cloudflare" },
    LabeledOption { value: "tailscale", label: "Tailscale" },
    LabeledOption { value: "ngrok", label: "ngrok" },
    LabeledOption { value: "custom", label: "自定义" },
];

fn selected_provider(provider: &str) -> Option<LabeledOption> {
    PROVIDER_OPTIONS
        .iter()
        .find(|option| option.value == provider)
        .copied()
        .or_else(|| PROVIDER_OPTIONS.iter().find(|option| option.value == "none").copied())
}

fn tunnel_message(message: TunnelMessage) -> Message {
    Message::Settings(SettingsMessage::Tunnel(message))
}

fn provider_changed(value: LabeledOption) -> Message {
    tunnel_message(TunnelMessage::ProviderChanged(value.value.to_string()))
}

fn cloudflare_token_changed(value: String) -> Message {
    tunnel_message(TunnelMessage::CloudflareTokenChanged(value))
}

fn tailscale_funnel_toggled(value: bool) -> Message {
    tunnel_message(TunnelMessage::TailscaleFunnelToggled(value))
}

fn tailscale_hostname_changed(value: String) -> Message {
    tunnel_message(TunnelMessage::TailscaleHostnameChanged(value))
}

fn ngrok_auth_token_changed(value: String) -> Message {
    tunnel_message(TunnelMessage::NgrokAuthTokenChanged(value))
}

fn ngrok_domain_changed(value: String) -> Message {
    tunnel_message(TunnelMessage::NgrokDomainChanged(value))
}

fn custom_start_command_changed(value: String) -> Message {
    tunnel_message(TunnelMessage::CustomStartCommandChanged(value))
}

fn custom_health_url_changed(value: String) -> Message {
    tunnel_message(TunnelMessage::CustomHealthUrlChanged(value))
}

fn custom_url_pattern_changed(value: String) -> Message {
    tunnel_message(TunnelMessage::CustomUrlPatternChanged(value))
}

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_row_with_flexible_control(label, description, 360.0, control)
}

fn text_row<'a>(
    app: &App,
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    settings_flexible_row_with_help(
        I18n::new(app.desktop_locale),
        &app.app_theme,
        label,
        description,
        360.0,
        settings_text_input(placeholder, value).on_input(on_input).width(Length::Fill),
    )
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
            tr(app.desktop_locale, "隧道配置"),
            tr(app.desktop_locale, "配置网关的暴露方式与 provider 细节。修改后需重启网关生效。"),
        ),
        content(app),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

pub(super) fn content(app: &App) -> Element<'_, Message> {
    let s = &app.tunnel_settings;
    let i18n = I18n::new(app.desktop_locale);

    let provider_pick = settings_select(
        PROVIDER_OPTIONS
            .map(|option| LabeledOption { label: tr(app.desktop_locale, option.label), ..option }),
        selected_provider(&s.provider)
            .map(|option| LabeledOption { label: tr(app.desktop_locale, option.label), ..option }),
        provider_changed,
    )
    .width(Length::Fill);

    let provider_row = field_row(
        tr(app.desktop_locale, "提供者"),
        tr(app.desktop_locale, "选择自动暴露网关到公网或私有网络的方式。"),
        provider_pick,
    );

    let provider_section: Element<'_, Message> = match s.provider.as_str() {
        "cloudflare" => column![
            settings_group_heading_with_help(
                i18n,
                &app.app_theme,
                "Cloudflare Tunnel",
                tr(app.desktop_locale, "使用 cloudflared token 将本地网关安全暴露到公网。"),
            ),
            settings_group_card(
                column![text_row(
                    app,
                    tr(app.desktop_locale, "令牌"),
                    tr(app.desktop_locale, "Zero Trust 侧下发的 tunnel token。"),
                    "Cloudflare Zero Trust token",
                    &s.cloudflare_token,
                    cloudflare_token_changed
                )]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
        "tailscale" => column![
            settings_group_heading(
                "Tailscale",
                tr(app.desktop_locale, "配置 tailscale serve 或 funnel，将网关发布到 tailnet 或公网。保存后需重启网关生效。"),
            ),
            settings_group_card(
                column![
                    bool_row(
                        tr(app.desktop_locale, "漏斗"),
                        tr(app.desktop_locale, "关闭时使用 serve，仅在 tailnet 内可见。"),
                        s.tailscale_funnel,
                        tr(app.desktop_locale, "启用 funnel（关闭时使用 serve）"),
                        tailscale_funnel_toggled,
                    ),
                    settings_divider(),
                    text_row(
                        app,
                        tr(app.desktop_locale, "主机名"),
                        tr(app.desktop_locale, "留空时将使用当前 Tailscale 节点名。"),
                        tr(app.desktop_locale, "可选：my-node.tailnet.ts.net"),
                        &s.tailscale_hostname,
                        tailscale_hostname_changed,
                    ),
                    settings_divider(),
                    field_row(
                        tr(app.desktop_locale, "访问方式"),
                        tr(app.desktop_locale, "serve 仅 tailnet 内可访问，funnel 才会暴露到公网。访问地址以启动日志为准，通常不要使用本地监听端口。"),
                        text(tr(app.desktop_locale, "保存配置后重启网关。公网 funnel 常见入口是 https://<主机名>/，tailnet-only 常见入口是 https://<主机名>:8443/。"))
                            .size(12)
                            .style(settings_muted_text_style),
                    ),
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
        "ngrok" => column![
            settings_group_heading_with_help(
                i18n,
                &app.app_theme,
                "ngrok",
                tr(app.desktop_locale, "配置 ngrok token，并可选绑定自定义 domain。"),
            ),
            settings_group_card(
                column![
                    text_row(
                        app,
                        tr(app.desktop_locale, "认证令牌"),
                        tr(app.desktop_locale, "ngrok 控制台生成的 auth token。"),
                        "ngrok auth token",
                        &s.ngrok_auth_token,
                        ngrok_auth_token_changed,
                    ),
                    settings_divider(),
                    text_row(
                        app,
                        tr(app.desktop_locale, "域名"),
                        tr(app.desktop_locale, "可选，绑定预留的自定义域名。"),
                        tr(app.desktop_locale, "可选：my-app.ngrok.app"),
                        &s.ngrok_domain,
                        ngrok_domain_changed,
                    ),
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
        "custom" => column![
            settings_group_heading_with_help(
                i18n,
                &app.app_theme,
                tr(app.desktop_locale, "自定义隧道"),
                tr(app.desktop_locale, "当前运行时使用命令型自定义隧道，因此这里配置 start_command / health_url / url_pattern。"),
            ),
            settings_group_card(
                column![
                    text_row(
                        app,
                        tr(app.desktop_locale, "启动命令"),
                        tr(app.desktop_locale, "支持在命令中使用 {host} 与 {port} 占位符。"),
                        tr(app.desktop_locale, "例如：bore local {port} --to bore.pub"),
                        &s.custom_start_command,
                        custom_start_command_changed,
                    ),
                    settings_divider(),
                    text_row(
                        app,
                        tr(app.desktop_locale, "健康检查 URL"),
                        tr(app.desktop_locale, "可选，用于确认隧道服务是否已准备就绪。"),
                        tr(app.desktop_locale, "可选：http://127.0.0.1:4040/api/tunnels"),
                        &s.custom_health_url,
                        custom_health_url_changed,
                    ),
                    settings_divider(),
                    text_row(
                        app,
                        tr(app.desktop_locale, "URL 模式"),
                        tr(app.desktop_locale, "可选，可填前缀或正则片段用于提取公网地址。"),
                        tr(app.desktop_locale, "可选：https:// 或正则片段"),
                        &s.custom_url_pattern,
                        custom_url_pattern_changed,
                    ),
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
        _ => column![
            settings_group_heading_with_help(
                i18n,
                &app.app_theme,
                tr(app.desktop_locale, "未启用隧道"),
                tr(app.desktop_locale, "保持 provider=none 时，网关不会自动创建公网隧道。"),
            ),
            settings_group_card(column![
                container(
                    text(tr(app.desktop_locale, "当前仅监听本地网关地址，不会主动创建公网或 tailnet 暴露。"))
                        .size(13)
                        .style(settings_muted_text_style),
                )
                .padding([20, 0])
            ]),
        ]
        .spacing(16)
        .into(),
    };

    column![
        settings_group_heading_with_help(
            i18n,
            &app.app_theme,
            tr(app.desktop_locale, "接入方式"),
            tr(app.desktop_locale, "选择隧道 provider，并根据 provider 展开对应参数。")
        ),
        settings_group_card(column![provider_row].spacing(0)),
        provider_section,
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}
#[cfg(test)]
#[path = "system_settings_tunnel_tests.rs"]
mod system_settings_tunnel_tests;
