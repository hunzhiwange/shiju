//! Transactional configuration draft shared by cron delivery and channel settings.
use crate::messages::ChannelsMessage;
use crate::state::ChannelsSettingsState;
use super::{Effect as Message, Services, ToastKind};
use crate::state::{WeComDeliveryDraft, WeComDeliveryMode};
use iced::Task;
use config_channel_services::{WeComConfig, WeComWebhookConfig};

type Credentials = (Option<WeComConfig>, Option<WeComWebhookConfig>);

pub(super) fn credentials(s: &ChannelsSettingsState) -> Result<Credentials, String> {
    let draft = s.wecom_delivery.as_ref().ok_or("企微配置弹窗未打开")?;
    if draft.mode == WeComDeliveryMode::Webhook {
        if draft.clear_webhook {
            return Ok((s.wecom.clone(), None));
        }
        let value = draft.webhook_url.trim();
        if value.is_empty() {
            return s
                .wecom_webhook
                .clone()
                .map(|cfg| (s.wecom.clone(), Some(cfg)))
                .ok_or_else(|| "请输入完整群机器人 Webhook".into());
        }
        let invalid = || "请输入官方企业微信群机器人 Webhook（HTTPS，包含 key）".to_string();
        let url = url::Url::parse(value).map_err(|_| invalid())?;
        let pairs = url.query_pairs().collect::<Vec<_>>();
        if url.scheme() != "https"
            || url.host_str() != Some("qyapi.weixin.qq.com")
            || url.port_or_known_default() != Some(443)
            || !url.username().is_empty()
            || url.password().is_some()
            || value.split('/').nth(2).is_some_and(|v| v.contains('@'))
            || url.fragment().is_some()
            || url.path() != "/cgi-bin/webhook/send"
            || pairs.len() != 1
            || pairs[0].0 != "key"
            || pairs[0].1.trim().is_empty()
        {
            return Err(invalid());
        }
        return Ok((s.wecom.clone(), Some(WeComWebhookConfig { webhook_url: value.into() })));
    }
    if draft.clear_secret {
        return Ok((None, s.wecom_webhook.clone()));
    }
    let secret = if draft.secret.is_empty() {
        s.wecom.as_ref().map(|cfg| cfg.secret.clone()).unwrap_or_default()
    } else {
        draft.secret.clone()
    };
    if draft.bot_id.trim().is_empty() || secret.trim().is_empty() {
        return Err("请填写 Bot ID 和 Secret，或完成扫码授权".into());
    }
    Ok((Some(WeComConfig { bot_id: draft.bot_id.trim().into(), secret }), s.wecom_webhook.clone()))
}

pub fn update(app: &mut ChannelsSettingsState, services: &Services, message: ChannelsMessage) -> Task<Message> {
    match message {
        ChannelsMessage::WeComDeliveryOpened(mode) => {
            if app.wecom_delivery.as_ref().is_some_and(|d| d.saving) {
                return Task::none();
            }
            app.qr_flow.close();
            app.wecom_delivery = Some(WeComDeliveryDraft {
                mode,
                bot_id: app
                    .wecom
                    .as_ref()
                    .map(|cfg| cfg.bot_id.clone())
                    .unwrap_or_default(),
                ..Default::default()
            });
            if mode == WeComDeliveryMode::Qr {
                return super::updates::update(
                    app,
                    services,
                    ChannelsMessage::QrRequested("wecom".into()),
                );
            }
        }
        ChannelsMessage::WeComDeliveryClosed => {
            if !app.wecom_delivery.as_ref().is_some_and(|d| d.saving) {
                app.wecom_delivery = None;
                app.qr_flow.close();
            }
        }
        ChannelsMessage::WeComDeliveryChanged(key, value) => {
            if let Some(draft) = app.wecom_delivery.as_mut().filter(|d| !d.saving)
            {
                match key.as_str() {
                    "bot_id" => draft.bot_id = value,
                    "secret" => {
                        draft.secret = value;
                        draft.clear_secret = false;
                    }
                    "webhook_url" => {
                        draft.webhook_url = value;
                        draft.clear_webhook = false;
                    }
                    _ => return Task::none(),
                }
                draft.error = None;
            }
        }
        ChannelsMessage::WeComDeliveryClearSecret => {
            if let Some(draft) = app.wecom_delivery.as_mut().filter(|d| !d.saving)
            {
                if draft.mode == WeComDeliveryMode::Webhook {
                    draft.clear_webhook = true;
                    draft.webhook_url.clear();
                } else {
                    draft.clear_secret = true;
                    draft.secret.clear();
                }
            }
        }
        ChannelsMessage::WeComDeliverySave => {
            if app.wecom_delivery.as_ref().is_none_or(|d| d.saving) {
                return Task::none();
            }
            let (wecom, webhook) = match credentials(app) {
                Ok(value) => value,
                Err(error) => {
                    app.wecom_delivery.as_mut().unwrap().error =
                        Some(error.clone());
                    return super::error_toast(error);
                }
            };
            let webhook_mode = app.wecom_delivery.as_ref().unwrap().mode
                == WeComDeliveryMode::Webhook;
            app.wecom_delivery.as_mut().unwrap().saving = true;
            app.qr_flow.close();
            let config = services.config.clone();
            return Task::perform(
                async move { config.update_channels_config_result_async(move |cfg| {
                    if webhook_mode {
                        cfg.wecom_webhook = webhook;
                    } else {
                        cfg.wecom = wecom;
                        if cfg.wecom.is_none() {
                            cfg.instance_default_projects.remove("wecom:default");
                            cfg.instance_execution.remove("wecom:default");
                        }
                    }
                }).await },
                |result| Message::Channels(ChannelsMessage::WeComDeliverySaved(result)),
            );
        }
        ChannelsMessage::WeComDeliverySaved(result) => {
            if app.wecom_delivery.is_none() {
                return Task::none();
            }
            match result {
                Ok(()) => {
                    let webhook_mode = app.wecom_delivery.as_ref().unwrap().mode
                        == WeComDeliveryMode::Webhook;
                    if let Ok((wecom, webhook)) = credentials(app) {
                        app.wecom = wecom;
                        app.wecom_webhook = webhook;
                        if !webhook_mode && app.wecom.is_none() {
                            app.instance_default_projects.remove("wecom:default");
                            app.instance_execution.remove("wecom:default");
                        }
                    }
                    app.wecom_delivery = None;
                    app.qr_flow.close();
                    return super::localized_toast(
                        ToastKind::Success,
                        "已保存凭据",
                        vec![],
                    );
                }
                Err(_) => {
                    let draft = app.wecom_delivery.as_mut().unwrap();
                    draft.saving = false;
                    // The API can return credential-bearing URLs. Do not display its raw payload.
                    draft.error = Some("保存失败，请检查网关连接与配置后重试".into());
                    return super::error_toast("保存失败，请检查网关连接与配置后重试".into());
                }
            }
        }
        _ => {}
    }
    Task::none()
}
