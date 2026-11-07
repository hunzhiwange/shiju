//! 系统设置中渠道配置页面的分组界面与配置项渲染。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style, rounded_action_btn_style,
    settings_error_banner, settings_group_card, settings_group_heading, settings_muted_text_style,
};
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::message::settings::{ChannelsMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{Image, button, column, container, row, text};
use iced::{Element, Length};

use super::common::{
    GROUP_REPLY_MODE_OPTIONS, LARK_RECEIVE_MODE_OPTIONS, QQ_RECEIVE_MODE_OPTIONS, add_instance_row,
    bool_row, default_project_selector, group_reply_mode_value, instance_title_row,
    lark_receive_mode_value, number_row, number_row_with_hint, number_row_with_key, panel,
    pick_row, pick_row_with_key, qq_receive_mode_value, text_row, text_row_with_key,
};

fn qr_authorization<'a>(app: &'a App, channel: &'static str) -> Element<'a, Message> {
    let flow = &app.channels_settings.qr_flow;
    let is_current = flow.channel.as_deref() == Some(channel);
    let mut content = column![
        settings_group_heading(
            tr(app.desktop_locale, "扫码授权"),
            tr(app.desktop_locale, "使用手机端扫描二维码；授权成功后请明确保存。")
        ),
        button(
            text(if is_current {
                tr(app.desktop_locale, "重新获取二维码")
            } else {
                tr(app.desktop_locale, "获取二维码")
            })
            .size(SETTINGS_CONTROL_TEXT_SIZE)
        )
        .padding([8, 12])
        .on_press(Message::Settings(SettingsMessage::Channels(ChannelsMessage::QrRequested(
            channel.to_string()
        ),)))
        .style(rounded_action_btn_style),
    ]
    .spacing(10);

    if is_current {
        if let Some(handle) = flow.qrcode_image.as_ref() {
            content = content.push(
                container(Image::new(handle.clone()).width(220).height(220)).center_x(Length::Fill),
            );
        }
        let status = match flow.status {
            Some(vw_gateway_client::ChannelQrStatusDto::Pending) => {
                tr(app.desktop_locale, "等待扫码")
            }
            Some(vw_gateway_client::ChannelQrStatusDto::Scanned) => {
                tr(app.desktop_locale, "已扫码，等待确认")
            }
            Some(vw_gateway_client::ChannelQrStatusDto::Authorized) => {
                tr(app.desktop_locale, "授权成功，尚未保存")
            }
            Some(vw_gateway_client::ChannelQrStatusDto::Expired) => {
                tr(app.desktop_locale, "二维码已过期")
            }
            Some(vw_gateway_client::ChannelQrStatusDto::Failed) => {
                tr(app.desktop_locale, "授权失败")
            }
            None => tr(app.desktop_locale, "正在获取二维码"),
        };
        content = content.push(match flow.status {
            Some(
                vw_gateway_client::ChannelQrStatusDto::Failed
                | vw_gateway_client::ChannelQrStatusDto::Expired,
            ) => settings_error_banner(status),
            _ => text(status).size(13).style(settings_muted_text_style).into(),
        });
        if let Some(expires_at) = flow.expires_at.as_deref() {
            content = content.push(
                row![
                    text(tr(app.desktop_locale, "有效期至：")).size(12),
                    text(expires_at).size(12)
                ]
                .spacing(4),
            );
        }
        let mut actions = row![
            button(text(tr(app.desktop_locale, "关闭")).size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 12])
                .on_press(Message::Settings(SettingsMessage::Channels(ChannelsMessage::QrClosed)))
                .style(rounded_action_btn_style),
        ]
        .spacing(8);
        if flow.credentials_ready {
            actions = actions.push(
                button(
                    text(tr(app.desktop_locale, "保存授权配置")).size(SETTINGS_CONTROL_TEXT_SIZE),
                )
                .padding([8, 12])
                .on_press(Message::Settings(SettingsMessage::Channels(
                    ChannelsMessage::QrSaveConfirmed,
                )))
                .style(primary_action_btn_style),
            );
        }
        content = content.push(actions);
    }
    settings_group_card(container(content).padding([14, 0])).into()
}

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
                tr(app.desktop_locale, "可选 encrypt key"),
                cfg.encrypt_key.as_deref().unwrap_or(""),
                "lark.encrypt_key",
                true,
            ),
            text_row(
                app,
                "verification_token",
                tr(app.desktop_locale, "可选 verification token"),
                cfg.verification_token.as_deref().unwrap_or(""),
                "lark.verification_token",
                true,
            ),
            text_row(
                app,
                "allowed_users",
                "逗号或换行分隔，支持 *",
                "",
                "lark.allowed_users",
                false,
            ),
            bool_row(
                app.desktop_locale,
                "mention_only",
                cfg.mention_only,
                tr(app.desktop_locale, "群聊仅 @ 时回复"),
                "lark.mention_only"
            ),
            bool_row(
                app.desktop_locale,
                "use_feishu",
                cfg.use_feishu,
                tr(app.desktop_locale, "使用飞书中国区端点"),
                "lark.use_feishu"
            ),
            pick_row(
                app.desktop_locale,
                "group_reply.mode",
                group_reply_mode_value(&cfg.group_reply),
                &GROUP_REPLY_MODE_OPTIONS,
                "lark.group_reply.mode",
            ),
            text_row(
                app,
                "group_reply.allowed_sender_ids",
                "逗号或换行分隔，支持 *",
                "",
                "lark.group_reply.allowed_sender_ids",
                false,
            ),
            pick_row(
                app.desktop_locale,
                "receive_mode",
                lark_receive_mode_value(cfg.receive_mode.clone()),
                &LARK_RECEIVE_MODE_OPTIONS,
                "lark.receive_mode",
            ),
            number_row(
                app.desktop_locale,
                "port",
                cfg.port.unwrap_or(3000) as u32,
                1,
                u16::MAX as u32,
                "",
                "lark.port",
            ),
            number_row(
                app.desktop_locale,
                "draft_update_interval_ms",
                cfg.draft_update_interval_ms as u32,
                100,
                60_000,
                "ms",
                "lark.draft_update_interval_ms",
            ),
            number_row(
                app.desktop_locale,
                "max_draft_edits",
                cfg.max_draft_edits,
                1,
                200,
                "",
                "lark.max_draft_edits"
            ),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };

    panel(
        app,
        "lark",
        "Lark",
        tr(app.desktop_locale, "国际版 Lark 的 app、接收模式与草稿编辑节流。"),
        s.lark.is_some(),
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
    let mut body = column![qr_authorization(app, "feishu")].spacing(12);

    if let Some(cfg) = s.feishu.as_ref() {
        body = body
            .push(default_project_selector(app, "feishu:default".into()))
            .push(settings_group_heading(
                tr(app.desktop_locale, "飞书详细配置"),
                tr(
                    app.desktop_locale,
                    "覆盖 app_id、app_secret、加密字段、接收模式、端口和草稿编辑参数。",
                ),
            ))
            .push(text_row(
                app,
                "app_id",
                tr(app.desktop_locale, "飞书 App ID"),
                &cfg.app_id,
                "feishu.app_id",
                true,
            ))
            .push(text_row(
                app,
                "app_secret",
                tr(app.desktop_locale, "飞书 App Secret"),
                &cfg.app_secret,
                "feishu.app_secret",
                true,
            ))
            .push(text_row(
                app,
                "encrypt_key",
                tr(app.desktop_locale, "可选 encrypt key"),
                cfg.encrypt_key.as_deref().unwrap_or(""),
                "feishu.encrypt_key",
                true,
            ))
            .push(text_row(
                app,
                "verification_token",
                tr(app.desktop_locale, "可选 verification token"),
                cfg.verification_token.as_deref().unwrap_or(""),
                "feishu.verification_token",
                true,
            ))
            .push(text_row(
                app,
                "allowed_users",
                "逗号或换行分隔，支持 *",
                "",
                "feishu.allowed_users",
                false,
            ))
            .push(pick_row(
                app.desktop_locale,
                "group_reply.mode",
                group_reply_mode_value(&cfg.group_reply),
                &GROUP_REPLY_MODE_OPTIONS,
                "feishu.group_reply.mode",
            ))
            .push(text_row(
                app,
                "group_reply.allowed_sender_ids",
                "逗号或换行分隔，支持 *",
                "",
                "feishu.group_reply.allowed_sender_ids",
                false,
            ))
            .push(pick_row(
                app.desktop_locale,
                "receive_mode",
                lark_receive_mode_value(cfg.receive_mode.clone()),
                &LARK_RECEIVE_MODE_OPTIONS,
                "feishu.receive_mode",
            ))
            .push(number_row_with_hint(
                app.desktop_locale,
                "port",
                cfg.port.unwrap_or(3000) as u32,
                1,
                u16::MAX as u32,
                "",
                "feishu.port",
                tr(
                    app.desktop_locale,
                    "Webhook 模式需要公网 HTTPS 入口；WebSocket 模式会忽略 port。",
                ),
            ))
            .push(number_row(
                app.desktop_locale,
                "draft_update_interval_ms",
                cfg.draft_update_interval_ms as u32,
                100,
                60_000,
                "ms",
                "feishu.draft_update_interval_ms",
            ))
            .push(number_row(
                app.desktop_locale,
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
            crate::__!(
                crate::app::i18n::I18n::new(app.desktop_locale),
                "飞书实例 %s",
                (index + 1).to_string()
            )
            .expect("valid channel instance template")
        } else {
            crate::__!(
                crate::app::i18n::I18n::new(app.desktop_locale),
                "飞书实例 %s",
                instance.id.as_str()
            )
            .expect("valid channel instance template")
        };
        let fields = column![instance_title_row(app.desktop_locale, title, "feishu", index)]
            .push(default_project_selector(app, format!("feishu:{}", instance.id)))
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
                tr(app.desktop_locale, "飞书 App ID"),
                &instance.config.app_id,
                format!("{key_prefix}.app_id"),
                true,
            ))
            .push(text_row_with_key(
                app,
                "app_secret",
                tr(app.desktop_locale, "飞书 App Secret"),
                &instance.config.app_secret,
                format!("{key_prefix}.app_secret"),
                true,
            ))
            .push(text_row_with_key(
                app,
                "encrypt_key",
                tr(app.desktop_locale, "可选 encrypt key"),
                instance.config.encrypt_key.as_deref().unwrap_or(""),
                format!("{key_prefix}.encrypt_key"),
                true,
            ))
            .push(text_row_with_key(
                app,
                "verification_token",
                tr(app.desktop_locale, "可选 verification token"),
                instance.config.verification_token.as_deref().unwrap_or(""),
                format!("{key_prefix}.verification_token"),
                true,
            ))
            .push(text_row_with_key(
                app,
                "allowed_users",
                "逗号或换行分隔，支持 *",
                "",
                format!("{key_prefix}.allowed_users"),
                false,
            ))
            .push(pick_row_with_key(
                app.desktop_locale,
                "group_reply.mode",
                group_reply_mode_value(&instance.config.group_reply),
                &GROUP_REPLY_MODE_OPTIONS,
                format!("{key_prefix}.group_reply.mode"),
            ))
            .push(text_row_with_key(
                app,
                "group_reply.allowed_sender_ids",
                "逗号或换行分隔，支持 *",
                "",
                format!("{key_prefix}.group_reply.allowed_sender_ids"),
                false,
            ))
            .push(pick_row_with_key(
                app.desktop_locale,
                "receive_mode",
                lark_receive_mode_value(instance.config.receive_mode.clone()),
                &LARK_RECEIVE_MODE_OPTIONS,
                format!("{key_prefix}.receive_mode"),
            ))
            .push(number_row_with_key(
                app.desktop_locale,
                "port",
                instance.config.port.unwrap_or(3000) as u32,
                1,
                u16::MAX as u32,
                "",
                format!("{key_prefix}.port"),
            ))
            .push(number_row_with_key(
                app.desktop_locale,
                "draft_update_interval_ms",
                instance.config.draft_update_interval_ms as u32,
                100,
                60_000,
                "ms",
                format!("{key_prefix}.draft_update_interval_ms"),
            ))
            .push(number_row_with_key(
                app.desktop_locale,
                "max_draft_edits",
                instance.config.max_draft_edits,
                1,
                200,
                "",
                format!("{key_prefix}.max_draft_edits"),
            ));
        body = body.push(settings_group_card(container(fields.spacing(12)).padding([14, 0])));
    }

    body = body.push(add_instance_row("feishu", tr(app.desktop_locale, "新增飞书实例")));

    panel(
        app,
        "feishu",
        tr(app.desktop_locale, "飞书"),
        tr(app.desktop_locale, "国内版飞书详细配置。"),
        enabled,
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
    let mut body = column![qr_authorization(app, "dingtalk")].spacing(12);

    if let Some(cfg) = s.dingtalk.as_ref() {
        body = body
            .push(default_project_selector(app, "dingtalk:default".into()))
            .push(text_row(
                app,
                "client_id",
                tr(app.desktop_locale, "钉钉 AppKey"),
                &cfg.client_id,
                "dingtalk.client_id",
                true,
            ))
            .push(text_row(
                app,
                "client_secret",
                tr(app.desktop_locale, "钉钉 AppSecret"),
                &cfg.client_secret,
                "dingtalk.client_secret",
                true,
            ))
            .push(text_row(
                app,
                "allowed_users",
                "逗号或换行分隔，支持 *",
                "",
                "dingtalk.allowed_users",
                false,
            ));
    }

    for (index, instance) in s.dingtalk_instances.iter().enumerate() {
        let key_prefix = format!("dingtalk_instances.{index}");
        let title = if instance.id.trim().is_empty() {
            crate::__!(
                crate::app::i18n::I18n::new(app.desktop_locale),
                "钉钉实例 %s",
                (index + 1).to_string()
            )
            .expect("valid channel instance template")
        } else {
            crate::__!(
                crate::app::i18n::I18n::new(app.desktop_locale),
                "钉钉实例 %s",
                instance.id.as_str()
            )
            .expect("valid channel instance template")
        };
        let fields = column![instance_title_row(app.desktop_locale, title, "dingtalk", index)]
            .push(default_project_selector(app, format!("dingtalk:{}", instance.id)))
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
                tr(app.desktop_locale, "钉钉 AppKey"),
                &instance.config.client_id,
                format!("{key_prefix}.client_id"),
                true,
            ))
            .push(text_row_with_key(
                app,
                "client_secret",
                tr(app.desktop_locale, "钉钉 AppSecret"),
                &instance.config.client_secret,
                format!("{key_prefix}.client_secret"),
                true,
            ))
            .push(text_row_with_key(
                app,
                "allowed_users",
                "逗号或换行分隔，支持 *",
                "",
                format!("{key_prefix}.allowed_users"),
                false,
            ));
        body = body.push(settings_group_card(container(fields.spacing(12)).padding([14, 0])));
    }

    body = body.push(add_instance_row("dingtalk", tr(app.desktop_locale, "新增钉钉实例")));

    panel(
        app,
        "dingtalk",
        tr(app.desktop_locale, "钉钉"),
        tr(
            app.desktop_locale,
            "钉钉 Stream 模式鉴权；白名单为空时拒绝所有用户，* 表示允许所有用户。",
        ),
        enabled,
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
            qr_authorization(app, "qq"),
            default_project_selector(app, "qq:default".into()),
            text_row(app, "app_id", "QQ app id", &cfg.app_id, "qq.app_id", true),
            text_row(app, "app_secret", "QQ app secret", &cfg.app_secret, "qq.app_secret", true),
            text_row(
                app,
                "allowed_users",
                "逗号或换行分隔，支持 *",
                "",
                "qq.allowed_users",
                false,
            ),
            pick_row(app.desktop_locale,
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
        app,
        "qq",
        "QQ",
        tr(app.desktop_locale, "QQ 官方机器人 app 信息与接收模式。"),
        s.qq.is_some(),
        body,
    )
}

pub(super) fn wecom_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.wecom.as_ref() {
        column![
            qr_authorization(app, "wecom"),
            default_project_selector(app, "wecom:default".into()),
            text_row(
                app,
                "bot_id",
                tr(app.desktop_locale, "企业微信 Bot ID"),
                &cfg.bot_id,
                "wecom.bot_id",
                false
            ),
            text_row(
                app,
                "secret",
                tr(app.desktop_locale, "企业微信 Secret"),
                &cfg.secret,
                "wecom.secret",
                true
            ),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };
    panel(
        app,
        "wecom",
        tr(app.desktop_locale, "企业微信"),
        tr(app.desktop_locale, "企业微信 AI 机器人长连接凭据。"),
        s.wecom.is_some(),
        body,
    )
}

pub(super) fn wechat_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body: Element<'_, Message> = if let Some(cfg) = s.wechat.as_ref() {
        column![
            qr_authorization(app, "wechat"),
            default_project_selector(app, "wechat:default".into()),
            text_row(
                app,
                "bot_token",
                tr(app.desktop_locale, "微信 Bot Token"),
                &cfg.bot_token,
                "wechat.bot_token",
                true,
            ),
            text_row(
                app,
                "base_url",
                tr(app.desktop_locale, "可信 iLink 地址"),
                &cfg.base_url,
                "wechat.base_url",
                false,
            ),
        ]
        .spacing(12)
        .into()
    } else {
        column![].into()
    };
    panel(
        app,
        "wechat",
        tr(app.desktop_locale, "微信"),
        tr(app.desktop_locale, "微信 iLink 机器人凭据与可信平台地址。"),
        s.wechat.is_some(),
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
                tr(app.desktop_locale, "例如：nsec1... 或 hex"),
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
        app,
        "nostr",
        "Nostr",
        tr(app.desktop_locale, "私钥、中继列表和允许的公钥白名单。"),
        s.nostr.is_some(),
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
                tr(app.desktop_locale, "可选 webhook 密钥"),
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
        app,
        "clawdtalk",
        "ClawdTalk",
        tr(app.desktop_locale, "Telnyx 语音通道的 API 鉴权与目的地限制。"),
        s.clawdtalk.is_some(),
        body,
    )
}
