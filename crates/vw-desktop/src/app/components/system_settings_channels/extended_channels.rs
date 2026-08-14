//! 系统设置中渠道配置页面的分组界面与配置项渲染。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::{App, Message};
use iced::Element;
use iced::widget::column;

use super::common::{bool_row, number_row, panel, text_row};

/// 构建或处理 `whatsapp_panel` 对应的界面片段与交互数据。
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
pub(super) fn whatsapp_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.whatsapp.as_ref() {
        column![
            text_row(
                app,
                "access_token",
                "Cloud API access token",
                cfg.access_token.as_deref().unwrap_or(""),
                "whatsapp.access_token",
                true,
            ),
            text_row(
                app,
                "phone_number_id",
                "Cloud API phone number id",
                cfg.phone_number_id.as_deref().unwrap_or(""),
                "whatsapp.phone_number_id",
                false,
            ),
            text_row(
                app,
                "verify_token",
                "Webhook verify token",
                cfg.verify_token.as_deref().unwrap_or(""),
                "whatsapp.verify_token",
                false,
            ),
            text_row(
                app,
                "app_secret",
                "Meta app secret",
                cfg.app_secret.as_deref().unwrap_or(""),
                "whatsapp.app_secret",
                true,
            ),
            text_row(
                app,
                "session_path",
                "Web 模式会话数据库路径",
                cfg.session_path.as_deref().unwrap_or(""),
                "whatsapp.session_path",
                false,
            ),
            text_row(
                app,
                "pair_phone",
                "可选配对手机号",
                cfg.pair_phone.as_deref().unwrap_or(""),
                "whatsapp.pair_phone",
                false,
            ),
            text_row(
                app,
                "pair_code",
                "可选固定配对码",
                cfg.pair_code.as_deref().unwrap_or(""),
                "whatsapp.pair_code",
                false,
            ),
            text_row(
                app,
                "allowed_numbers",
                "逗号或换行分隔",
                "",
                "whatsapp.allowed_numbers",
                false,
            ),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel(
        "whatsapp",
        "WhatsApp",
        "Cloud API 或 Web 模式字段共用一组表单。",
        s.whatsapp.is_some(),
        body,
    )
}

/// 构建或处理 `linq_panel` 对应的界面片段与交互数据。
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
pub(super) fn linq_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.linq.as_ref() {
        column![
            text_row(app, "api_token", "Linq API token", &cfg.api_token, "linq.api_token", true),
            text_row(app, "from_phone", "+1234567890", &cfg.from_phone, "linq.from_phone", false),
            text_row(
                app,
                "signing_secret",
                "可选签名密钥",
                cfg.signing_secret.as_deref().unwrap_or(""),
                "linq.signing_secret",
                true,
            ),
            text_row(app, "allowed_senders", "逗号或换行分隔", "", "linq.allowed_senders", false,),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel("linq", "Linq", "Partner API、发信号码与签名校验。", s.linq.is_some(), body)
}

/// 构建或处理 `wati_panel` 对应的界面片段与交互数据。
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
pub(super) fn wati_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.wati.as_ref() {
        column![
            text_row(app, "api_token", "WATI API token", &cfg.api_token, "wati.api_token", true),
            text_row(
                app,
                "api_url",
                "https://live-mt-server.wati.io",
                &cfg.api_url,
                "wati.api_url",
                false,
            ),
            text_row(
                app,
                "tenant_id",
                "可选 tenant id",
                cfg.tenant_id.as_deref().unwrap_or(""),
                "wati.tenant_id",
                false,
            ),
            text_row(app, "allowed_numbers", "逗号或换行分隔", "", "wati.allowed_numbers", false,),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel("wati", "WATI", "第三方 WhatsApp Business API 集成。", s.wati.is_some(), body)
}

/// 构建或处理 `nextcloud_talk_panel` 对应的界面片段与交互数据。
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
pub(super) fn nextcloud_talk_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.nextcloud_talk.as_ref() {
        column![
            text_row(
                app,
                "base_url",
                "https://cloud.example.com",
                &cfg.base_url,
                "nextcloud_talk.base_url",
                false,
            ),
            text_row(
                app,
                "app_token",
                "Nextcloud app token",
                &cfg.app_token,
                "nextcloud_talk.app_token",
                true,
            ),
            text_row(
                app,
                "webhook_secret",
                "可选 webhook 密钥",
                cfg.webhook_secret.as_deref().unwrap_or(""),
                "nextcloud_talk.webhook_secret",
                true,
            ),
            text_row(
                app,
                "allowed_users",
                "逗号或换行分隔",
                "",
                "nextcloud_talk.allowed_users",
                false,
            ),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel(
        "nextcloud_talk",
        "Nextcloud Talk",
        "基础地址、应用 token 与 webhook 密钥。",
        s.nextcloud_talk.is_some(),
        body,
    )
}

#[cfg(not(target_arch = "wasm32"))]
/// 构建或处理 `email_panel` 对应的界面片段与交互数据。
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
pub(super) fn email_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.email.as_ref() {
        column![
            text_row(
                app,
                "imap_host",
                "imap.example.com",
                &cfg.imap_host,
                "email.imap_host",
                false,
            ),
            number_row(
                "imap_port",
                cfg.imap_port as u32,
                1,
                u16::MAX as u32,
                "",
                "email.imap_port",
            ),
            text_row(app, "imap_folder", "INBOX", &cfg.imap_folder, "email.imap_folder", false),
            text_row(
                app,
                "smtp_host",
                "smtp.example.com",
                &cfg.smtp_host,
                "email.smtp_host",
                false,
            ),
            number_row(
                "smtp_port",
                cfg.smtp_port as u32,
                1,
                u16::MAX as u32,
                "",
                "email.smtp_port",
            ),
            bool_row("smtp_tls", cfg.smtp_tls, "SMTP 使用 TLS", "email.smtp_tls"),
            text_row(app, "username", "邮箱用户名", &cfg.username, "email.username", false),
            text_row(app, "password", "邮箱密码", &cfg.password, "email.password", true),
            text_row(
                app,
                "from_address",
                "bot@example.com",
                &cfg.from_address,
                "email.from_address",
                false,
            ),
            number_row(
                "idle_timeout_secs",
                cfg.idle_timeout_secs as u32,
                60,
                86_400,
                "secs",
                "email.idle_timeout_secs",
            ),
            text_row(app, "allowed_senders", "逗号或换行分隔", "", "email.allowed_senders", false,),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel("email", "Email", "IMAP / SMTP 收发邮件配置。", s.email.is_some(), body)
}

/// 构建或处理 `irc_panel` 对应的界面片段与交互数据。
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
pub(super) fn irc_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.irc.as_ref() {
        column![
            text_row(app, "server", "irc.example.com", &cfg.server, "irc.server", false),
            number_row("port", cfg.port as u32, 1, u16::MAX as u32, "", "irc.port"),
            text_row(app, "nickname", "vibewindow_bot", &cfg.nickname, "irc.nickname", false),
            text_row(
                app,
                "username",
                "可选 username",
                cfg.username.as_deref().unwrap_or(""),
                "irc.username",
                false,
            ),
            text_row(app, "channels", "#general, #support", "", "irc.channels", false),
            text_row(app, "allowed_users", "逗号或换行分隔", "", "irc.allowed_users", false),
            text_row(
                app,
                "server_password",
                "可选 server password",
                cfg.server_password.as_deref().unwrap_or(""),
                "irc.server_password",
                true,
            ),
            text_row(
                app,
                "nickserv_password",
                "可选 NickServ 密码",
                cfg.nickserv_password.as_deref().unwrap_or(""),
                "irc.nickserv_password",
                true,
            ),
            text_row(
                app,
                "sasl_password",
                "可选 SASL 密码",
                cfg.sasl_password.as_deref().unwrap_or(""),
                "irc.sasl_password",
                true,
            ),
            bool_row(
                "verify_tls",
                cfg.verify_tls.unwrap_or(true),
                "验证 TLS 证书",
                "irc.verify_tls",
            ),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel("irc", "IRC", "IRC over TLS、自动加入频道和认证口令。", s.irc.is_some(), body)
}
