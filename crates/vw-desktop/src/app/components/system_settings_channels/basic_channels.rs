//! 系统设置中渠道配置页面的分组界面与配置项渲染。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::{App, Message};
use iced::Element;
use iced::widget::column;

use super::common::{
    GROUP_REPLY_MODE_OPTIONS, bool_row, group_reply_mode_value, number_row, panel, pick_row, text_row,
};

/// 构建或处理 `telegram_panel` 对应的界面片段与交互数据。
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
pub(super) fn telegram_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.telegram.as_ref() {
        column![
            text_row(
                app,
                "bot_token",
                "Telegram 机器人令牌",
                &cfg.bot_token,
                "telegram.bot_token",
                true,
            ),
            text_row(app, "allowed_users", "逗号或换行分隔", "", "telegram.allowed_users", false,),
            text_row(
                app,
                "接口地址",
                "可选兼容 API 地址",
                cfg.base_url.as_deref().unwrap_or(""),
                "telegram.base_url",
                false,
            ),
            number_row(
                "草稿刷新间隔",
                cfg.draft_update_interval_ms as u32,
                100,
                60_000,
                "毫秒",
                "telegram.draft_update_interval_ms",
            ),
            bool_row(
                "新消息打断",
                cfg.interrupt_on_new_message,
                "新消息到达时中断当前响应",
                "telegram.interrupt_on_new_message",
            ),
            bool_row("仅提及时回复", cfg.mention_only, "群聊仅 @ 时回复", "telegram.mention_only",),
            pick_row(
                "群聊回复模式",
                group_reply_mode_value(&cfg.group_reply),
                &GROUP_REPLY_MODE_OPTIONS,
                "telegram.group_reply.mode",
            ),
            text_row(
                app,
                "允许发送者 ID",
                "逗号或换行分隔",
                "",
                "telegram.group_reply.allowed_sender_ids",
                false,
            ),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel("telegram", "Telegram", "机器人令牌、群聊行为与白名单控制。", s.telegram.is_some(), body)
}

/// 构建或处理 `discord_panel` 对应的界面片段与交互数据。
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
pub(super) fn discord_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.discord.as_ref() {
        column![
            text_row(
                app,
                "bot_token",
                "Discord 机器人令牌",
                &cfg.bot_token,
                "discord.bot_token",
                true,
            ),
            text_row(
                app,
                "服务器 ID",
                "可选服务器 ID",
                cfg.guild_id.as_deref().unwrap_or(""),
                "discord.guild_id",
                false,
            ),
            text_row(app, "allowed_users", "逗号或换行分隔", "", "discord.allowed_users", false,),
            bool_row(
                "监听机器人消息",
                cfg.listen_to_bots,
                "监听其他机器人消息",
                "discord.listen_to_bots",
            ),
            bool_row(
                "仅提及时回复",
                cfg.mention_only,
                "仅在 @ 机器人时回复",
                "discord.mention_only",
            ),
            pick_row(
                "群聊回复模式",
                group_reply_mode_value(&cfg.group_reply),
                &GROUP_REPLY_MODE_OPTIONS,
                "discord.group_reply.mode",
            ),
            text_row(
                app,
                "允许发送者 ID",
                "逗号或换行分隔",
                "",
                "discord.group_reply.allowed_sender_ids",
                false,
            ),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel("discord", "Discord", "机器人令牌、服务器限制与群聊回复控制。", s.discord.is_some(), body)
}

/// 构建或处理 `slack_panel` 对应的界面片段与交互数据。
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
pub(super) fn slack_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.slack.as_ref() {
        column![
            text_row(app, "机器人令牌", "xoxb-...", &cfg.bot_token, "slack.bot_token", true),
            text_row(
                app,
                "应用令牌",
                "xapp-...",
                cfg.app_token.as_deref().unwrap_or(""),
                "slack.app_token",
                true,
            ),
            text_row(
                app,
                "频道 ID",
                "可选频道 ID",
                cfg.channel_id.as_deref().unwrap_or(""),
                "slack.channel_id",
                false,
            ),
            text_row(app, "allowed_users", "逗号或换行分隔", "", "slack.allowed_users", false,),
            pick_row(
                "群聊回复模式",
                group_reply_mode_value(&cfg.group_reply),
                &GROUP_REPLY_MODE_OPTIONS,
                "slack.group_reply.mode",
            ),
            text_row(
                app,
                "允许发送者 ID",
                "逗号或换行分隔",
                "",
                "slack.group_reply.allowed_sender_ids",
                false,
            ),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel("slack", "Slack", "机器人令牌、Socket Mode 应用令牌与频道限制。", s.slack.is_some(), body)
}

/// 构建或处理 `mattermost_panel` 对应的界面片段与交互数据。
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
pub(super) fn mattermost_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.mattermost.as_ref() {
        column![
            text_row(
                app,
                "服务器地址",
                "https://mattermost.example.com",
                &cfg.url,
                "mattermost.url",
                false,
            ),
            text_row(
                app,
                "bot_token",
                "Mattermost 机器人令牌",
                &cfg.bot_token,
                "mattermost.bot_token",
                true,
            ),
            text_row(
                app,
                "频道 ID",
                "可选频道 ID",
                cfg.channel_id.as_deref().unwrap_or(""),
                "mattermost.channel_id",
                false,
            ),
            text_row(
                app,
                "allowed_users",
                "逗号或换行分隔",
                "",
                "mattermost.allowed_users",
                false,
            ),
            bool_row(
                "线程回复",
                cfg.thread_replies.unwrap_or(true),
                "在线程中回复",
                "mattermost.thread_replies",
            ),
            bool_row(
                "仅提及时回复",
                cfg.mention_only.unwrap_or(false),
                "仅 @ 时回复",
                "mattermost.mention_only",
            ),
            pick_row(
                "群聊回复模式",
                group_reply_mode_value(&cfg.group_reply),
                &GROUP_REPLY_MODE_OPTIONS,
                "mattermost.group_reply.mode",
            ),
            text_row(
                app,
                "允许发送者 ID",
                "逗号或换行分隔",
                "",
                "mattermost.group_reply.allowed_sender_ids",
                false,
            ),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel(
        "mattermost",
        "Mattermost",
        "服务器地址、机器人令牌、线程回复与仅提及时回复。",
        s.mattermost.is_some(),
        body,
    )
}

/// 构建或处理 `webhook_panel` 对应的界面片段与交互数据。
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
pub(super) fn webhook_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.webhook.as_ref() {
        column![
            number_row("端口", cfg.port as u32, 1, u16::MAX as u32, "", "webhook.port"),
            text_row(
                app,
                "共享密钥",
                "可选 Webhook 共享密钥",
                cfg.secret.as_deref().unwrap_or(""),
                "webhook.secret",
                true,
            ),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel("webhook", "Webhook", "接收入站网络事件的轻量端点。", s.webhook.is_some(), body)
}

/// 构建或处理 `imessage_panel` 对应的界面片段与交互数据。
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
pub(super) fn imessage_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if s.imessage.is_some() {
        column![text_row(
            app,
            "allowed_contacts",
            "电话号码或邮箱，逗号或换行分隔",
            "",
            "imessage.allowed_contacts",
            false,
        )]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel("imessage", "iMessage", "macOS 下允许接入的联系人列表。", s.imessage.is_some(), body)
}

/// 构建或处理 `signal_panel` 对应的界面片段与交互数据。
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
pub(super) fn signal_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.signal.as_ref() {
        column![
            text_row(
                app,
                "http_url",
                "http://127.0.0.1:8686",
                &cfg.http_url,
                "signal.http_url",
                false,
            ),
            text_row(app, "account", "+1234567890", &cfg.account, "signal.account", false),
            text_row(
                app,
                "group_id",
                "dm 或特定 group id",
                cfg.group_id.as_deref().unwrap_or(""),
                "signal.group_id",
                false,
            ),
            text_row(app, "allowed_from", "逗号或换行分隔", "", "signal.allowed_from", false,),
            bool_row(
                "ignore_attachments",
                cfg.ignore_attachments,
                "忽略纯附件消息",
                "signal.ignore_attachments",
            ),
            bool_row(
                "ignore_stories",
                cfg.ignore_stories,
                "忽略 Signal Story",
                "signal.ignore_stories",
            ),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel("signal", "Signal", "signal-cli HTTP daemon 地址与来源过滤。", s.signal.is_some(), body)
}
