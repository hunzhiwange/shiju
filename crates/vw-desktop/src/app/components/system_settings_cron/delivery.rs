//! 定时任务结果通知；群 Webhook 与 Agent 对话接入使用独立凭据。
use super::*;
use crate::app::message::settings::{ChannelsMessage, SettingsMessage as S};
use crate::app::state::{CronJobDraft, WeComDeliveryMode};
use iced::widget::column;

#[derive(Clone, PartialEq)]
enum DeliveryChoice {
    Bot,
    Webhook,
    Existing(String),
}
impl DeliveryChoice {
    fn channel(&self) -> &str {
        match self {
            Self::Bot => "wecom",
            Self::Webhook => "wecom_webhook",
            Self::Existing(channel) => channel,
        }
    }
}
impl std::fmt::Display for DeliveryChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bot => f.write_str("企业微信智能机器人"),
            Self::Webhook => f.write_str("企业微信群机器人（Webhook）"),
            Self::Existing(channel) => write!(f, "当前通道：{channel}"),
        }
    }
}

pub(super) fn view<'a>(app: &'a App, draft: &'a CronJobDraft) -> Element<'a, Message> {
    if !draft.delivery_enabled {
        return column![].into();
    }
    let selected = match draft.delivery_channel.as_str() {
        "wecom" => Some(DeliveryChoice::Bot),
        "wecom_webhook" => Some(DeliveryChoice::Webhook),
        "" => None,
        channel => Some(DeliveryChoice::Existing(channel.into())),
    };
    let mut options = vec![DeliveryChoice::Bot, DeliveryChoice::Webhook];
    if let Some(choice @ DeliveryChoice::Existing(_)) = &selected {
        options.push(choice.clone());
    }
    let mut content = column![field_row(
        "投递方式",
        "智能机器人通过已认证连接发送到指定会话；群机器人通过 Webhook 发送到群。",
        settings_select(options, selected, |choice| Message::Settings(
            S::CronJobEditDeliveryChannelChanged(choice.channel().into())
        ))
        .width(Length::Fill),
    )];
    let mode = match draft.delivery_channel.as_str() {
        "wecom" => Some((
            WeComDeliveryMode::Qr,
            app.channels_settings.wecom.as_ref().is_some_and(|c| c.has_required_credentials()),
        )),
        "wecom_webhook" => {
            Some((WeComDeliveryMode::Webhook, app.channels_settings.wecom_webhook.is_some()))
        }
        _ => None,
    };
    if let Some((mode, configured)) = mode {
        content = content.push(field_row(
            "通知配置",
            if configured {
                "已保存凭据；连接与投递状态请查看运行记录。"
            } else if mode == WeComDeliveryMode::Webhook {
                "尚未配置，请先保存通知机器人的 Webhook。"
            } else {
                "请扫码或填写 Bot ID 和 Secret 连接智能机器人，再指定接收用户或会话。"
            },
            secondary_button(
                if configured { "修改配置" } else { "配置" },
                Message::Settings(S::Channels(ChannelsMessage::WeComDeliveryOpened(mode))),
            ),
        ));
    }
    if draft.delivery_channel == "wecom_webhook" {
        content =
            content.push(text("接收目标：Webhook 对应的企业微信群，无需用户或会话 ID").size(13));
    } else if !draft.delivery_channel.is_empty() {
        content = content.push(field_row(
            "投递目标",
            "填写接收用户 ID / 会话 ID；不会自动选择最近联系人。旧通道沿用原目标格式。",
            text_field("接收目标 ID", &draft.delivery_to, |v| {
                Message::Settings(S::CronJobEditDeliveryToChanged(v))
            }),
        ));
    }
    content
        .push(field_row(
            "尽力投递",
            "开启后失败记入历史但不改变任务状态；关闭后投递失败会将任务标记为失败。",
            checkbox(draft.delivery_best_effort)
                .label(I18n::new(app.desktop_locale).__("投递失败不影响任务状态"))
                .on_toggle(|v| Message::Settings(S::CronJobEditDeliveryBestEffortToggled(v)))
                .style(settings_checkbox_style),
        ))
        .into()
}

#[cfg(test)]
#[path = "delivery_tests.rs"]
mod tests;
