//! 系统设置中渠道配置页面的分组界面与配置项渲染。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::settings_section_card;
use crate::app::{App, Message};
use iced::Element;
use iced::widget::column;

use super::common::{
    GROUP_REPLY_MODE_OPTIONS, LARK_RECEIVE_MODE_OPTIONS, QQ_RECEIVE_MODE_OPTIONS, add_instance_row,
    bool_row, group_reply_mode_value, hint_row, instance_title_row, lark_receive_mode_value,
    number_row, number_row_with_key, panel, pick_row, pick_row_with_key, qq_receive_mode_value,
    text_row, text_row_with_key,
};

/// 构建或处理 `lark_panel` 对应的界面片段与交互数据。
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
pub(super) fn lark_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.lark.as_ref() {
        column![
            text_row(app, "app_id", "Lark App ID", &cfg.app_id, "lark.app_id", true),
            text_row(
                app,
                "app_secret",
                "Lark App Secret",
                &cfg.app_secret,
                "lark.app_secret",
                true,
            ),
            text_row(
                app,
                "encrypt_key",
                "可选 encrypt key",
                cfg.encrypt_key.as_deref().unwrap_or(""),
                "lark.encrypt_key",
                true,
            ),
            text_row(
                app,
                "verification_token",
                "可选 verification token",
                cfg.verification_token.as_deref().unwrap_or(""),
                "lark.verification_token",
                true,
            ),
            text_row(app, "allowed_users", "逗号或换行分隔", "", "lark.allowed_users", false),
            bool_row("mention_only", cfg.mention_only, "群聊仅 @ 时回复", "lark.mention_only"),
            bool_row("use_feishu", cfg.use_feishu, "使用 Feishu 中国区端点", "lark.use_feishu"),
            pick_row(
                "group_reply.mode",
                group_reply_mode_value(&cfg.group_reply),
                &GROUP_REPLY_MODE_OPTIONS,
                "lark.group_reply.mode",
            ),
            text_row(
                app,
                "group_reply.allowed_sender_ids",
                "逗号或换行分隔",
                "",
                "lark.group_reply.allowed_sender_ids",
                false,
            ),
            pick_row(
                "receive_mode",
                lark_receive_mode_value(cfg.receive_mode.clone()),
                &LARK_RECEIVE_MODE_OPTIONS,
                "lark.receive_mode",
            ),
            number_row(
                "port",
                cfg.port.unwrap_or(3000) as u32,
                1,
                u16::MAX as u32,
                "",
                "lark.port",
            ),
            number_row(
                "draft_update_interval_ms",
                cfg.draft_update_interval_ms as u32,
                100,
                60_000,
                "ms",
                "lark.draft_update_interval_ms",
            ),
            number_row("max_draft_edits", cfg.max_draft_edits, 1, 200, "", "lark.max_draft_edits"),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel(
        "lark",
        "Lark",
        "国际版 Lark 的 app、接收模式与草稿编辑节流。",
        s.lark.is_some(),
        s.expanded_panels.contains("lark"),
        body,
    )
}

/// 构建或处理 `feishu_panel` 对应的界面片段与交互数据。
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
pub(super) fn feishu_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let enabled = s.feishu.is_some() || !s.feishu_instances.is_empty();
    let mut body = column![].spacing(12);

    if let Some(cfg) = s.feishu.as_ref() {
        body = body
            .push(settings_section_card(
                "Feishu 详细配置",
                "覆盖 app_id、app_secret、加密字段、接收模式、端口和草稿编辑参数。",
            ))
            .push(text_row(app, "app_id", "Feishu App ID", &cfg.app_id, "feishu.app_id", true))
            .push(text_row(
                app,
                "app_secret",
                "Feishu App Secret",
                &cfg.app_secret,
                "feishu.app_secret",
                true,
            ))
            .push(text_row(
                app,
                "encrypt_key",
                "可选 encrypt key",
                cfg.encrypt_key.as_deref().unwrap_or(""),
                "feishu.encrypt_key",
                true,
            ))
            .push(text_row(
                app,
                "verification_token",
                "可选 verification token",
                cfg.verification_token.as_deref().unwrap_or(""),
                "feishu.verification_token",
                true,
            ))
            .push(text_row(
                app,
                "allowed_users",
                "逗号或换行分隔",
                "",
                "feishu.allowed_users",
                false,
            ))
            .push(pick_row(
                "group_reply.mode",
                group_reply_mode_value(&cfg.group_reply),
                &GROUP_REPLY_MODE_OPTIONS,
                "feishu.group_reply.mode",
            ))
            .push(text_row(
                app,
                "group_reply.allowed_sender_ids",
                "逗号或换行分隔",
                "",
                "feishu.group_reply.allowed_sender_ids",
                false,
            ))
            .push(pick_row(
                "receive_mode",
                lark_receive_mode_value(cfg.receive_mode.clone()),
                &LARK_RECEIVE_MODE_OPTIONS,
                "feishu.receive_mode",
            ))
            .push(number_row(
                "port",
                cfg.port.unwrap_or(3000) as u32,
                1,
                u16::MAX as u32,
                "",
                "feishu.port",
            ))
            .push(hint_row("Webhook 模式需要公网 HTTPS 入口；WebSocket 模式会忽略 port。"))
            .push(number_row(
                "draft_update_interval_ms",
                cfg.draft_update_interval_ms as u32,
                100,
                60_000,
                "ms",
                "feishu.draft_update_interval_ms",
            ))
            .push(number_row(
                "max_draft_edits",
                cfg.max_draft_edits,
                1,
                200,
                "",
                "feishu.max_draft_edits",
            ));
    }

    for (index, instance) in s.feishu_instances.iter().enumerate() {
        let key_prefix = format!("feishu_instances.{index}");
        let title = if instance.id.trim().is_empty() {
            format!("Feishu 实例 {}", index + 1)
        } else {
            format!("Feishu 实例 {}", instance.id)
        };
        body = body
            .push(instance_title_row(title, "feishu", index))
            .push(text_row_with_key(
                app,
                "instance_id",
                "cn",
                &instance.id,
                format!("{key_prefix}.id"),
                false,
            ))
            .push(text_row_with_key(
                app,
                "app_id",
                "Feishu App ID",
                &instance.config.app_id,
                format!("{key_prefix}.app_id"),
                true,
            ))
            .push(text_row_with_key(
                app,
                "app_secret",
                "Feishu App Secret",
                &instance.config.app_secret,
                format!("{key_prefix}.app_secret"),
                true,
            ))
            .push(text_row_with_key(
                app,
                "encrypt_key",
                "可选 encrypt key",
                instance.config.encrypt_key.as_deref().unwrap_or(""),
                format!("{key_prefix}.encrypt_key"),
                true,
            ))
            .push(text_row_with_key(
                app,
                "verification_token",
                "可选 verification token",
                instance.config.verification_token.as_deref().unwrap_or(""),
                format!("{key_prefix}.verification_token"),
                true,
            ))
            .push(text_row_with_key(
                app,
                "allowed_users",
                "逗号或换行分隔",
                "",
                format!("{key_prefix}.allowed_users"),
                false,
            ))
            .push(pick_row_with_key(
                "group_reply.mode",
                group_reply_mode_value(&instance.config.group_reply),
                &GROUP_REPLY_MODE_OPTIONS,
                format!("{key_prefix}.group_reply.mode"),
            ))
            .push(text_row_with_key(
                app,
                "group_reply.allowed_sender_ids",
                "逗号或换行分隔",
                "",
                format!("{key_prefix}.group_reply.allowed_sender_ids"),
                false,
            ))
            .push(pick_row_with_key(
                "receive_mode",
                lark_receive_mode_value(instance.config.receive_mode.clone()),
                &LARK_RECEIVE_MODE_OPTIONS,
                format!("{key_prefix}.receive_mode"),
            ))
            .push(number_row_with_key(
                "port",
                instance.config.port.unwrap_or(3000) as u32,
                1,
                u16::MAX as u32,
                "",
                format!("{key_prefix}.port"),
            ))
            .push(number_row_with_key(
                "draft_update_interval_ms",
                instance.config.draft_update_interval_ms as u32,
                100,
                60_000,
                "ms",
                format!("{key_prefix}.draft_update_interval_ms"),
            ))
            .push(number_row_with_key(
                "max_draft_edits",
                instance.config.max_draft_edits,
                1,
                200,
                "",
                format!("{key_prefix}.max_draft_edits"),
            ));
    }

    body = body.push(add_instance_row("feishu", "新增 Feishu 实例"));

    panel(
        "feishu",
        "Feishu",
        "国内版飞书详细配置。",
        enabled,
        s.expanded_panels.contains("feishu"),
        body.into(),
    )
}

/// 构建或处理 `dingtalk_panel` 对应的界面片段与交互数据。
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
pub(super) fn dingtalk_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let enabled = s.dingtalk.is_some() || !s.dingtalk_instances.is_empty();
    let mut body = column![].spacing(12);

    if let Some(cfg) = s.dingtalk.as_ref() {
        body = body
            .push(text_row(
                app,
                "client_id",
                "DingTalk AppKey",
                &cfg.client_id,
                "dingtalk.client_id",
                true,
            ))
            .push(text_row(
                app,
                "client_secret",
                "DingTalk AppSecret",
                &cfg.client_secret,
                "dingtalk.client_secret",
                true,
            ))
            .push(text_row(
                app,
                "allowed_users",
                "逗号或换行分隔",
                "",
                "dingtalk.allowed_users",
                false,
            ));
    }

    for (index, instance) in s.dingtalk_instances.iter().enumerate() {
        let key_prefix = format!("dingtalk_instances.{index}");
        let title = if instance.id.trim().is_empty() {
            format!("DingTalk 实例 {}", index + 1)
        } else {
            format!("DingTalk 实例 {}", instance.id)
        };
        body = body
            .push(instance_title_row(title, "dingtalk", index))
            .push(text_row_with_key(
                app,
                "instance_id",
                "ops",
                &instance.id,
                format!("{key_prefix}.id"),
                false,
            ))
            .push(text_row_with_key(
                app,
                "client_id",
                "DingTalk AppKey",
                &instance.config.client_id,
                format!("{key_prefix}.client_id"),
                true,
            ))
            .push(text_row_with_key(
                app,
                "client_secret",
                "DingTalk AppSecret",
                &instance.config.client_secret,
                format!("{key_prefix}.client_secret"),
                true,
            ))
            .push(text_row_with_key(
                app,
                "allowed_users",
                "逗号或换行分隔",
                "",
                format!("{key_prefix}.allowed_users"),
                false,
            ));
    }

    body = body.push(add_instance_row("dingtalk", "新增 DingTalk 实例"));

    panel(
        "dingtalk",
        "DingTalk",
        "钉钉 Stream 模式鉴权与白名单。",
        enabled,
        s.expanded_panels.contains("dingtalk"),
        body.into(),
    )
}

/// 构建或处理 `qq_panel` 对应的界面片段与交互数据。
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
pub(super) fn qq_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.qq.as_ref() {
        column![
            text_row(app, "app_id", "QQ app id", &cfg.app_id, "qq.app_id", true),
            text_row(app, "app_secret", "QQ app secret", &cfg.app_secret, "qq.app_secret", true),
            text_row(app, "allowed_users", "逗号或换行分隔", "", "qq.allowed_users", false),
            pick_row(
                "receive_mode",
                qq_receive_mode_value(cfg.receive_mode.clone()),
                &QQ_RECEIVE_MODE_OPTIONS,
                "qq.receive_mode",
            ),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel(
        "qq",
        "QQ",
        "QQ 官方机器人 app 信息与接收模式。",
        s.qq.is_some(),
        s.expanded_panels.contains("qq"),
        body,
    )
}

/// 构建或处理 `nostr_panel` 对应的界面片段与交互数据。
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
pub(super) fn nostr_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.nostr.as_ref() {
        column![
            text_row(
                app,
                "private_key",
                "nsec1... 或 hex",
                &cfg.private_key,
                "nostr.private_key",
                true,
            ),
            text_row(app, "relays", "逗号或换行分隔", "", "nostr.relays", false),
            text_row(app, "allowed_pubkeys", "逗号或换行分隔", "", "nostr.allowed_pubkeys", false,),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel(
        "nostr",
        "Nostr",
        "私钥、中继列表和允许的公钥白名单。",
        s.nostr.is_some(),
        s.expanded_panels.contains("nostr"),
        body,
    )
}

/// 构建或处理 `clawdtalk_panel` 对应的界面片段与交互数据。
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
pub(super) fn clawdtalk_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.clawdtalk.as_ref() {
        column![
            text_row(app, "api_key", "Telnyx API key", &cfg.api_key, "clawdtalk.api_key", true),
            text_row(
                app,
                "connection_id",
                "Telnyx connection id",
                &cfg.connection_id,
                "clawdtalk.connection_id",
                false,
            ),
            text_row(
                app,
                "from_number",
                "+12345678900",
                &cfg.from_number,
                "clawdtalk.from_number",
                false,
            ),
            text_row(
                app,
                "allowed_destinations",
                "逗号或换行分隔，支持 *",
                "",
                "clawdtalk.allowed_destinations",
                false,
            ),
            text_row(
                app,
                "webhook_secret",
                "可选 webhook 密钥",
                cfg.webhook_secret.as_deref().unwrap_or(""),
                "clawdtalk.webhook_secret",
                true,
            ),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel(
        "clawdtalk",
        "ClawdTalk",
        "Telnyx 语音通道的 API 鉴权与目的地限制。",
        s.clawdtalk.is_some(),
        s.expanded_panels.contains("clawdtalk"),
        body,
    )
}
