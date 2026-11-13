//! 分离 Agent 对话接入与定时任务群通知；仅内容区滚动。
use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_error_banner, settings_modal_header, settings_modal_layout, settings_modal_overlay,
    settings_muted_text_style, settings_text_input,
};
use crate::app::message::settings::{ChannelsMessage, SettingsMessage};
use crate::app::state::WeComDeliveryMode;
use crate::app::{App, Message};
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

fn message(value: ChannelsMessage) -> Message {
    Message::Settings(SettingsMessage::Channels(value))
}

pub(crate) fn view_overlays<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(draft) = app.channels_settings.wecom_delivery.as_ref() else {
        return base;
    };
    let mut body = column![].spacing(12).width(Length::Fill);
    let modes = if draft.mode == WeComDeliveryMode::Webhook {
        vec![]
    } else {
        vec![(WeComDeliveryMode::Qr, "扫码连接"), (WeComDeliveryMode::Manual, "手动连接")]
    };
    for (mode, title) in modes {
        let mut control = button(text(title))
            .padding([8, 12])
            .width(Length::Fill)
            .style(rounded_action_btn_style);
        if !draft.saving && mode != draft.mode {
            control = control.on_press(message(ChannelsMessage::WeComDeliveryOpened(mode)));
        }
        body = body.push(control);
    }
    body = body.push(
        text(if draft.mode == WeComDeliveryMode::Webhook {
            "仅用于发送任务结果通知，不接收指令，无需连接 Agent 对话通道。"
        } else {
            "用于接收消息并与 Agent 对话；连接状态请查看运行诊断。"
        })
        .size(13)
        .style(settings_muted_text_style),
    );
    match draft.mode {
        WeComDeliveryMode::Qr => {
            body = body.push(super::enterprise_channels::qr_authorization(app, "wecom"));
        }
        WeComDeliveryMode::Manual => {
            body = body.push(text("Bot ID"));
            let input = settings_text_input("Bot ID", &draft.bot_id).width(Length::Fill);
            body = body.push(if draft.saving {
                input
            } else {
                input.on_input(|v| {
                    message(ChannelsMessage::WeComDeliveryChanged("bot_id".into(), v))
                })
            });
            body = body.push(text("Secret"));
            let placeholder = if app.channels_settings.wecom.is_some() {
                "已配置，留空保留；输入新值替换"
            } else {
                "输入 Secret"
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
            body = body
                .push(text("使用企业微信官方长连接地址 wss://openws.work.weixin.qq.com").size(12));
        }
        WeComDeliveryMode::Webhook => {
            body = body.push(text("完整群机器人 Webhook"));
            let placeholder = if app.channels_settings.wecom_webhook.is_some() {
                "已配置，留空保留；输入新值替换"
            } else {
                "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=…"
            };
            let input = settings_text_input(placeholder, &draft.webhook_url)
                .secure(true)
                .width(Length::Fill);
            body = body.push(if draft.saving {
                input
            } else {
                input.on_input(|v| {
                    message(ChannelsMessage::WeComDeliveryChanged("webhook_url".into(), v))
                })
            });
            body = body.push(text("仅配置一个默认群机器人，无需 Bot ID 或 Secret。").size(13));
        }
    }
    let mut clear = button(text("清空已存凭据")).padding([8, 12]).style(rounded_action_btn_style);
    if !draft.saving {
        clear = clear.on_press(message(ChannelsMessage::WeComDeliveryClearSecret));
    }
    body = body.push(clear);
    if draft.clear_secret || draft.clear_webhook {
        body = body.push(text("保存后将移除当前入口的凭据；取消可放弃。").size(13));
    }
    if let Some(error) = &draft.error {
        body = body.push(settings_error_banner(error.as_str()));
    }
    let mut save = button(text(if draft.saving { "正在保存…" } else { "保存凭据" }))
        .padding([8, 16])
        .style(primary_action_btn_style);
    let mut cancel = button(text("取消")).padding([8, 16]).style(rounded_action_btn_style);
    if !draft.saving {
        save = save.on_press(message(ChannelsMessage::WeComDeliverySave));
        cancel = cancel.on_press(message(ChannelsMessage::WeComDeliveryClosed));
    }
    let header = settings_modal_header(
        text(if draft.mode == WeComDeliveryMode::Webhook {
            "定时任务群通知"
        } else {
            "企业微信 Agent 对话接入"
        })
        .size(20),
        settings_close_button(message(ChannelsMessage::WeComDeliveryClosed)),
    );
    let card = settings_modal_layout(header, body, Some(row![cancel, save].spacing(10).into()))
        .width(Length::Fill)
        .max_width(560)
        .height(Length::Fill)
        .max_height(680);
    settings_modal_overlay(
        Some(base),
        message(ChannelsMessage::WeComDeliveryClosed),
        container(card).padding(12),
    )
}
