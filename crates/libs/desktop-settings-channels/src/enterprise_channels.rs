//! 系统设置中渠道配置页面的分组界面与配置项渲染。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::help::{settings_group_heading_with_help};
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style, rounded_action_btn_style,
    settings_error_banner, settings_group_card, settings_group_heading,
    settings_muted_text_style, settings_text_input,
};
use desktop_i18n::{I18n, settings_extensions::text as tr};
use crate::messages::{ChannelsMessage, SettingsMessage};
use crate::{ViewState, messages::Message};
use iced::widget::{Image, button, column, container, row, text};
use iced::{Alignment, Element, Length};

use super::common::{
    GROUP_REPLY_MODE_OPTIONS, LARK_RECEIVE_MODE_OPTIONS, QQ_RECEIVE_MODE_OPTIONS, add_instance_row,
    bool_row, channel_header, default_project_selector, group_reply_mode_value, hint_row,
    lark_receive_mode_value, number_row, number_row_with_hint, number_row_with_key, panel,
    pick_row, pick_row_with_key, qq_receive_mode_value, text_row, text_row_with_key,
};

fn enterprise_instance_panel<'a>(
    app: ViewState<'a>,
    channel: &'static str,
    title: &'static str,
    description: &'static str,
    enabled: bool,
    default: iced::widget::Column<'a, Message>,
    instances: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let mut sections = column![settings_group_card(
        container(channel_header(app, channel, title, description, enabled, std::convert::identity))
            .padding([16, 0])
    )]
    .spacing(12);
    if enabled {
        let state = app.channels_settings;
        let toggle_all = |expanded, label| {
            let can_toggle = state.instance_cards(channel)
                .any(|index| state.instance_collapsed(channel, index) == expanded);
            button(text(label).size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe(can_toggle.then(|| Message::Settings(SettingsMessage::Channels(
                    ChannelsMessage::AllInstancesExpandedToggled(channel.to_owned(), expanded),
                ))))
        };
        let expand_controls = row![
            toggle_all(true, i18n.__("展开所有实例")),
            toggle_all(false, i18n.__("折叠所有实例")),
        ]
        .spacing(8)
        .width(Length::Fill)
        .wrap()
        .vertical_spacing(8);
        let toolbar = row![expand_controls, add_instance_row(channel, i18n.__("新增实例"))]
            .spacing(8)
            .align_y(Alignment::Center);
        sections = sections
            .push(toolbar)
            .push(enterprise_instance_card(app, channel, None, i18n.__("默认实例").into(), default))
            .extend(instances);
    }
    sections.into()
}

fn enterprise_instance_card<'a>(
    app: ViewState<'a>,
    channel: &'static str,
    index: Option<usize>,
    title: String,
    fields: iced::widget::Column<'a, Message>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let collapsed = app.channels_settings.instance_collapsed(channel, index);
    let mut header = row![
        text(title).size(16).width(Length::Fill),
        button(text(if collapsed { i18n.__("展开") } else { i18n.__("折叠") })
            .size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(Message::Settings(SettingsMessage::Channels(
                ChannelsMessage::InstanceExpandedToggled(channel.to_owned(), index, collapsed),
            ))),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if let Some(index) = index {
        header = header.push(button(text(i18n.__("删除")).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(Message::Settings(SettingsMessage::Channels(
                ChannelsMessage::InstanceRemoved(channel.to_owned(), index),
            ))));
    }
    let mut content = column![header].spacing(12);
    if !collapsed {
        content = content.push(fields);
    }
    settings_group_card(container(content).padding([14, 0])).into()
}

fn configure_default_instance<'a>(app: ViewState<'a>, channel: &'static str) -> Element<'a, Message> {
    button(text(I18n::new(app.desktop_locale).__("配置默认实例")))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press(Message::Settings(SettingsMessage::Channels(
            ChannelsMessage::EnabledToggled(channel.to_owned(), true),
        )))
        .into()
}

pub(crate) fn qr_authorization<'a>(app: ViewState<'a>, channel: &'static str) -> Element<'a, Message> {
    qr_authorization_for(app, channel, None)
}

fn qr_authorization_for<'a>(
    app: ViewState<'a>,
    channel: &'static str,
    instance_id: Option<&str>,
) -> Element<'a, Message> {
    let flow = &app.channels_settings.qr_flow;
    let is_current =
        flow.channel.as_deref() == Some(channel) && flow.instance_id.as_deref() == instance_id;
    let target = match instance_id {
        Some(id) => format!("{channel}:{id}"),
        None => channel.to_owned(),
    };
    let wecom_modal = app.channels_settings.wecom_delivery.as_ref().filter(|_| channel == "wecom");
    let request = !wecom_modal.is_some_and(|draft| draft.saving);
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
        .on_press_maybe(request.then(|| {
            Message::Settings(SettingsMessage::Channels(ChannelsMessage::QrRequested(target)))
        }))
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
            Some(gateway_channel_qr::ChannelQrStatusDto::Pending) => {
                tr(app.desktop_locale, "等待扫码")
            }
            Some(gateway_channel_qr::ChannelQrStatusDto::Scanned) => {
                tr(app.desktop_locale, "已扫码，等待确认")
            }
            Some(gateway_channel_qr::ChannelQrStatusDto::Authorized) => {
                tr(app.desktop_locale, "授权成功，尚未保存")
            }
            Some(gateway_channel_qr::ChannelQrStatusDto::Expired) => {
                tr(app.desktop_locale, "二维码已过期")
            }
            Some(gateway_channel_qr::ChannelQrStatusDto::Failed) => {
                tr(app.desktop_locale, "授权失败")
            }
            None => tr(app.desktop_locale, "正在获取二维码"),
        };
        content = content.push(match flow.status {
            Some(
                gateway_channel_qr::ChannelQrStatusDto::Failed
                | gateway_channel_qr::ChannelQrStatusDto::Expired,
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
        if wecom_modal.is_none() {
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
pub fn lark_panel(app: ViewState<'_>) -> Element<'_, Message> {
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
pub fn feishu_panel(app: ViewState<'_>) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let mut body = column![qr_authorization(app, "feishu")].spacing(12);

    if let Some(cfg) = s.feishu.as_ref() {
        body = body
            .push(default_project_selector(app, "feishu:default".into()))
            .push(settings_group_heading_with_help(
                I18n::new(app.desktop_locale),
                app.app_theme,
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
                app,
                "port",
                cfg.port.unwrap_or(3000) as u32,
                1,
                u16::MAX as u32,
                "feishu.port",
                ("WebSocket 模式会忽略端口。", "Webhook 模式需要公网 HTTPS 入口。"),
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
    } else {
        body = body.push(configure_default_instance(app, "feishu"));
    }

    let mut instances = Vec::new();
    for (index, instance) in s.feishu_instances.iter().enumerate() {
        let key_prefix = format!("feishu_instances.{index}");
        let title = if instance.id.trim().is_empty() {
            desktop_i18n::__!(
                desktop_i18n::I18n::new(app.desktop_locale),
                "飞书实例 %s",
                (index + 1).to_string()
            )
            .expect("valid channel instance template")
        } else {
            desktop_i18n::__!(
                desktop_i18n::I18n::new(app.desktop_locale),
                "飞书实例 %s",
                instance.id.as_str()
            )
            .expect("valid channel instance template")
        };
        let fields = column![default_project_selector(app, format!("feishu:{}", instance.id))]
            .spacing(12)
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
        instances.push(enterprise_instance_card(app, "feishu", Some(index), title, fields));
    }

    enterprise_instance_panel(
        app,
        "feishu",
        tr(app.desktop_locale, "飞书"),
        tr(app.desktop_locale, "国内版飞书详细配置。"),
        s.feishu.is_some() || !s.feishu_instances.is_empty(),
        body,
        instances,
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
pub fn dingtalk_panel(app: ViewState<'_>) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let mut body = column![
        hint_row(tr(app.desktop_locale, "白名单为空时拒绝所有用户。")),
        qr_authorization(app, "dingtalk")
    ]
    .spacing(12);

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
    } else {
        body = body.push(configure_default_instance(app, "dingtalk"));
    }

    let mut instances = Vec::new();
    for (index, instance) in s.dingtalk_instances.iter().enumerate() {
        let key_prefix = format!("dingtalk_instances.{index}");
        let title = if instance.id.trim().is_empty() {
            desktop_i18n::__!(
                desktop_i18n::I18n::new(app.desktop_locale),
                "钉钉实例 %s",
                (index + 1).to_string()
            )
            .expect("valid channel instance template")
        } else {
            desktop_i18n::__!(
                desktop_i18n::I18n::new(app.desktop_locale),
                "钉钉实例 %s",
                instance.id.as_str()
            )
            .expect("valid channel instance template")
        };
        let fields = column![default_project_selector(app, format!("dingtalk:{}", instance.id))]
            .spacing(12)
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
        instances.push(enterprise_instance_card(app, "dingtalk", Some(index), title, fields));
    }

    enterprise_instance_panel(
        app,
        "dingtalk",
        tr(app.desktop_locale, "钉钉"),
        tr(app.desktop_locale, "钉钉 Stream 模式鉴权。"),
        s.dingtalk.is_some() || !s.dingtalk_instances.is_empty(),
        body,
        instances,
    )
}

fn named_bot_fields<'a>(
    app: ViewState<'a>,
    channel: &'static str,
    id: &str,
) -> iced::widget::Column<'a, Message> {
    let fields = column![].spacing(12);
    let project = default_project_selector(app, format!("{channel}:{id}"));
    if channel == "wecom" {
        let i18n = I18n::new(app.desktop_locale);
        let configured = app.channels_settings
            .wecom_connection(Some(id))
            .is_some_and(|bot| bot.has_required_credentials());
        fields
            .push(text(if configured { i18n.__("已配置") } else { i18n.__("未配置") }).size(13))
            .push(wecom_connection_button(app, Some(id)))
            .push(project)
    } else {
        fields.push(project).push(qr_authorization_for(app, channel, Some(id)))
    }
}

fn named_bot_card<'a>(
    app: ViewState<'a>,
    channel: &'static str,
    index: usize,
    id: &str,
    fields: iced::widget::Column<'a, Message>,
) -> Element<'a, Message> {
    let title = desktop_i18n::__!(I18n::new(app.desktop_locale), "实例 %s", id)
        .expect("valid channel instance template");
    enterprise_instance_card(app, channel, Some(index), title, fields)
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
pub fn qq_panel(app: ViewState<'_>) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body = if let Some(cfg) = s.qq.as_ref() {
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
    } else {
        column![
            qr_authorization(app, "qq"),
            configure_default_instance(app, "qq"),
        ]
        .spacing(12)
    };

    let mut instances = Vec::new();
    for (index, instance) in app.channels_settings.qq_instances.iter().enumerate() {
        let key_prefix = format!("qq_instances.{index}");
        let fields = named_bot_fields(app, "qq", &instance.id)
            .push(text_row_with_key(
                app,
                "app_id",
                "QQ app id",
                &instance.config.app_id,
                format!("{key_prefix}.app_id"),
                true,
            ))
            .push(text_row_with_key(
                app,
                "app_secret",
                "QQ app secret",
                &instance.config.app_secret,
                format!("{key_prefix}.app_secret"),
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
            .push(hint_row(tr(app.desktop_locale, "新增 QQ 实例使用 WebSocket 接收消息。")));
        instances.push(named_bot_card(app, "qq", index, &instance.id, fields));
    }

    enterprise_instance_panel(
        app,
        "qq",
        tr(app.desktop_locale, "QQ"),
        tr(app.desktop_locale, "QQ 官方机器人 app 信息与接收模式。"),
        s.qq.is_some() || !s.qq_instances.is_empty(),
        body,
        instances,
    )
}

fn wecom_connection_button<'a>(app: ViewState<'a>, instance_id: Option<&str>) -> Element<'a, Message> {
    use crate::state::WeComDeliveryMode;
    let message = match instance_id {
        Some(id) => ChannelsMessage::WeComInstanceOpened(id.to_owned(), WeComDeliveryMode::Qr),
        None => ChannelsMessage::WeComDeliveryOpened(WeComDeliveryMode::Qr),
    };
    button(text(I18n::new(app.desktop_locale).__("配置机器人连接")))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press(Message::Settings(SettingsMessage::Channels(message)))
        .into()
}

pub fn wecom_panel(app: ViewState<'_>) -> Element<'_, Message> {
    let s = app.channels_settings;
    let mut body = column![
        text(if app.channels_settings.wecom.as_ref().is_some_and(|bot| bot.has_required_credentials()) {
            tr(app.desktop_locale, "默认智能机器人：已配置")
        } else {
            tr(app.desktop_locale, "默认智能机器人：未配置")
        })
        .size(13),
        wecom_connection_button(app, None),
    ]
    .spacing(12);
    if app.channels_settings.wecom.is_some() {
        body = body.push(default_project_selector(app, "wecom:default".into()));
    }
    let mut instances = Vec::new();
    let mut has_chat_instance = false;
    for (index, instance) in s.wecom_instances.iter().enumerate() {
        if config_channel_services::WeComConfig::is_cron_instance(&instance.id) {
            continue;
        }
        has_chat_instance = true;
        let fields = named_bot_fields(app, "wecom", &instance.id);
        instances.push(named_bot_card(app, "wecom", index, &instance.id, fields));
    }

    enterprise_instance_panel(
        app,
        "wecom",
        tr(app.desktop_locale, "企业微信"),
        tr(app.desktop_locale, "企业微信 AI 机器人长连接凭据。"),
        s.wecom.is_some() || has_chat_instance,
        body,
        instances,
    )
}

pub(super) fn wecom_cron_card(app: ViewState<'_>) -> Element<'_, Message> {
    use crate::state::WeComDeliveryMode;
    use config_channel_services::WeComConfig;

    let s = app.channels_settings;
    let i18n = I18n::new(app.desktop_locale);
    let event = |event| Message::Settings(SettingsMessage::Channels(event));
    let action = |label, action| {
        button(text(i18n.__(label)).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe(action)
    };
    let mut cards = column![
        row![
            text(i18n.__("定时任务企业微信通知")).size(17).width(Length::Fill),
            action("新增通知实例", Some(event(ChannelsMessage::WeComCronInstanceAdded))),
        ].spacing(8).align_y(Alignment::Center),
        hint_row(i18n.__("连接机器人并绑定接收会话，然后在任务里勾选要通知的实例。")),
    ].spacing(12);
    for instance in s.wecom_instances.iter().filter(|item| WeComConfig::is_cron_instance(&item.id)) {
        let id = &instance.id;
        let status = s.wecom_cron_bindings.get(id);
        let request = |start| {
            (!s.wecom_cron_binding_requests.contains_key(id)).then(|| event(ChannelsMessage::WeComCronBindingRequested {
                instance: id.clone(), start,
            }))
        };
        let bound = status.and_then(|status| status.bound);
        let rename = (instance.config.has_required_credentials() && s.wecom_cron_name.is_none())
            .then(|| event(ChannelsMessage::WeComCronNameOpened(id.clone())));
        let mut content = column![
            text(s.wecom_cron_label(instance)).size(16),
            row![
                action(if bound == Some(true) { "重新绑定接收会话" } else { "绑定接收会话" }, request(true)),
                action("检查连接状态", request(false)),
                action("管理连接", Some(event(ChannelsMessage::WeComInstanceOpened(id.clone(), WeComDeliveryMode::Manual)))),
                action("重命名", rename),
            ].spacing(8).wrap(),
        ].spacing(8);
        if let Some(draft) = s.wecom_cron_name.as_ref().filter(|draft| &draft.instance == id) {
            let save = event(ChannelsMessage::WeComCronNameSave);
            let input = settings_text_input(&instance.config.bot_id, &draft.value).width(Length::Fill);
            let input = if draft.saving {
                input
            } else {
                input
                    .on_input(move |value| event(ChannelsMessage::WeComCronNameChanged(value)))
                    .on_submit(save.clone())
            };
            content = content
                .push(text(i18n.__("显示名称")).size(SETTINGS_CONTROL_TEXT_SIZE))
                .push(input)
                .push(row![
                    button(text(i18n.__(if draft.saving { "正在保存…" } else { "保存名称" }))
                        .size(SETTINGS_CONTROL_TEXT_SIZE))
                        .padding([8, 12])
                        .style(primary_action_btn_style)
                        .on_press_maybe((!draft.saving).then_some(save)),
                    action("取消", (!draft.saving).then(|| event(ChannelsMessage::WeComCronNameClosed))),
                ].spacing(8).wrap());
        }
        if let Some(connected) = status.and_then(|status| status.connected) {
            content = content.push(text(i18n.__(match (connected, bound) {
                (false, _) => "定时任务机器人尚未连接",
                (true, Some(true)) => "接收会话已绑定",
                (true, _) => "接收会话尚未绑定",
            })).size(13));
        }
        if let Some(code) = status.and_then(|status| status.pairing_code.as_ref()) {
            content = content
                .push(text(i18n.__("请在 5 分钟内私聊专用机器人，发送下方绑定指令，然后检查连接状态。")).size(13))
                .push(text(code.as_str()).size(13))
                .push(action("复制绑定指令", Some(Message::CopyCode(code.clone()))));
        }
        cards = cards.push(settings_group_card(container(content).padding([14, 0])));
    }
    cards.into()
}

pub fn wechat_panel(app: ViewState<'_>) -> Element<'_, Message> {
    let s = &app.channels_settings;
    let body = if let Some(cfg) = s.wechat.as_ref() {
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
    } else {
        column![
            qr_authorization(app, "wechat"),
            configure_default_instance(app, "wechat"),
        ]
        .spacing(12)
    };
    let mut instances = Vec::new();
    for (index, instance) in app.channels_settings.wechat_instances.iter().enumerate() {
        let key_prefix = format!("wechat_instances.{index}");
        let fields = named_bot_fields(app, "wechat", &instance.id)
            .push(text_row_with_key(
                app,
                "bot_token",
                "微信 Bot Token",
                &instance.config.bot_token,
                format!("{key_prefix}.bot_token"),
                true,
            ))
            .push(text_row_with_key(
                app,
                "base_url",
                "可信 iLink 地址",
                &instance.config.base_url,
                format!("{key_prefix}.base_url"),
                false,
            ));
        instances.push(named_bot_card(app, "wechat", index, &instance.id, fields));
    }

    enterprise_instance_panel(
        app,
        "wechat",
        tr(app.desktop_locale, "微信"),
        tr(app.desktop_locale, "微信 iLink 机器人凭据与可信平台地址。"),
        s.wechat.is_some() || !s.wechat_instances.is_empty(),
        body,
        instances,
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
pub fn nostr_panel(app: ViewState<'_>) -> Element<'_, Message> {
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
pub fn clawdtalk_panel(app: ViewState<'_>) -> Element<'_, Message> {
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
