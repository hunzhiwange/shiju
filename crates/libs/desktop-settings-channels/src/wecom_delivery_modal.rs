//! 智能机器人连接弹窗，仅内容区滚动。
use desktop_ui_controls::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_error_banner, settings_modal_header, settings_modal_layout, settings_modal_overlay,
    settings_muted_text_style, settings_tab_button, settings_text_input,
};
use crate::messages::{ChannelsMessage, SettingsMessage};
use crate::state::WeComDeliveryMode;
use crate::{ViewState, messages::Message};
use iced::widget::{button, column, row, text};
use iced::{Element, Length};

fn message(value: ChannelsMessage) -> Message {
    Message::Settings(SettingsMessage::Channels(value))
}

pub fn overlay<'a, M: Clone + 'a>(
    app: ViewState<'a>,
    base: Element<'a, M>,
    map: fn(Message) -> M,
) -> Element<'a, M> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let Some(draft) = app.channels_settings.wecom_delivery.as_ref() else {
        return base;
    };
    let cron = draft.instance_id.as_deref().is_some_and(config_channel_services::WeComConfig::is_cron_instance);
    let mut body = column![].spacing(12).width(Length::Fill);
    let bot = app.channels_settings.wecom_connection(draft.instance_id.as_deref());
    let mode_tab = |mode, title| {
        let selected = mode == draft.mode;
        let on_press = (!draft.saving && !selected).then(|| {
            message(match &draft.instance_id {
                Some(id) => ChannelsMessage::WeComInstanceOpened(id.clone(), mode),
                None => ChannelsMessage::WeComDeliveryOpened(mode),
            })
        });
        settings_tab_button(title, selected, on_press).width(Length::Fill)
    };
    body = body
        .push(row![
            mode_tab(WeComDeliveryMode::Qr, i18n.__("扫码连接")),
            mode_tab(WeComDeliveryMode::Manual, i18n.__("手动连接")),
        ].spacing(8))
        .push(text(i18n.__(if cron { "保存后在通道中绑定接收会话。" } else { "用于接收消息并与 Agent 对话；连接状态请查看运行诊断。" }))
            .size(13)
            .style(settings_muted_text_style));
    match draft.mode {
        WeComDeliveryMode::Qr => {
            body = body.push(super::enterprise_channels::qr_authorization(app, "wecom"));
        }
        WeComDeliveryMode::Manual => {
            body = body.push(text(i18n.__("机器人 ID")));
            let input = settings_text_input(i18n.__("机器人 ID"), &draft.bot_id).width(Length::Fill);
            body = body.push(if draft.saving {
                input
            } else {
                input.on_input(|v| {
                    message(ChannelsMessage::WeComDeliveryChanged("bot_id".into(), v))
                })
            });
            body = body.push(text(i18n.__("机器人密钥")));
            let placeholder = if bot.is_some_and(|bot| !bot.secret.is_empty()) {
                i18n.__("已配置，留空保留；输入新值替换")
            } else {
                i18n.__("输入 Secret")
            };
            let input =
                settings_text_input(placeholder, &draft.secret).secure(true).width(Length::Fill);
            body = body.push(if draft.saving {
                input
            } else {
                input.on_input(|v| {
                    message(ChannelsMessage::WeComDeliveryChanged("secret".into(), v))
                })
            });
            body = body.push(
                text(i18n.__("使用企业微信官方长连接地址 wss://openws.work.weixin.qq.com"))
                    .size(12),
            );
        }
    }
    let has_saved_credentials = bot.is_some_and(|bot| !bot.secret.is_empty());
    if has_saved_credentials {
        let clear = button(text(i18n.__(if cron { "断开连接" } else { "清空已存凭据" })))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!draft.saving).then(|| message(ChannelsMessage::WeComDeliveryClearSecret)));
        body = body.push(clear);
    }
    if draft.clear_secret {
        body = body.push(text(i18n.__("保存后将移除当前入口的凭据；取消可放弃。")).size(13));
    }
    if let Some(error) = &draft.error {
        body = body.push(settings_error_banner(i18n.__(error)));
    }
    let mut save = button(text(if draft.saving {
        i18n.__("正在保存…")
    } else {
        i18n.__("保存凭据")
    }))
    .padding([8, 16])
    .style(primary_action_btn_style);
    let mut cancel = button(text(i18n.__("取消"))).padding([8, 16]).style(rounded_action_btn_style);
    if !draft.saving {
        if draft.mode != WeComDeliveryMode::Qr || !draft.secret.is_empty() || draft.clear_secret {
            save = save.on_press(message(ChannelsMessage::WeComDeliverySave));
        }
        cancel = cancel.on_press(message(ChannelsMessage::WeComDeliveryClosed));
    }
    let title = match draft.instance_id.as_deref() {
        Some(_) if cron => i18n.__("定时任务企业微信通知").to_owned(),
        Some(id) => desktop_i18n::__!(i18n, "配置企业微信实例 %s", id).expect("valid WeCom instance title"),
        None => i18n.__("配置默认智能机器人").to_owned(),
    };
    let header = settings_modal_header(
        text(title).size(20),
        settings_close_button(message(ChannelsMessage::WeComDeliveryClosed)),
    );
    let card = settings_modal_layout(header, body, Some(row![cancel, save].spacing(10).into()))
        .width(560)
        .height(680);
    let card: Element<'a, Message> = card.into();
    settings_modal_overlay(
        Some(base),
        map(message(ChannelsMessage::WeComDeliveryClosed)),
        card.map(map),
    )
}
