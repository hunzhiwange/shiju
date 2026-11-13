//! Transactional configuration draft shared by cron delivery and channel settings.
use super::super::messages::{ChannelsMessage, SettingsMessage};
use crate::app::state::{WeComDeliveryDraft, WeComDeliveryMode};
use crate::app::{App, Message};
use iced::Task;
use vw_config_types::channels::{WeComConfig, WeComWebhookConfig};

type Credentials = (Option<WeComConfig>, Option<WeComWebhookConfig>);

pub(super) fn credentials(app: &App) -> Result<Credentials, String> {
    let s = &app.channels_settings;
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
        let url = reqwest::Url::parse(value).map_err(|_| invalid())?;
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

pub(super) fn update(app: &mut App, message: ChannelsMessage) -> Task<Message> {
    match message {
        ChannelsMessage::WeComDeliveryOpened(mode) => {
            if app.channels_settings.wecom_delivery.as_ref().is_some_and(|d| d.saving) {
                return Task::none();
            }
            app.channels_settings.qr_flow.close();
            app.channels_settings.wecom_delivery = Some(WeComDeliveryDraft {
                mode,
                bot_id: app
                    .channels_settings
                    .wecom
                    .as_ref()
                    .map(|cfg| cfg.bot_id.clone())
                    .unwrap_or_default(),
                ..Default::default()
            });
            if mode == WeComDeliveryMode::Qr {
                return super::updates::update(
                    app,
                    SettingsMessage::Channels(ChannelsMessage::QrRequested("wecom".into())),
                );
            }
        }
        ChannelsMessage::WeComDeliveryClosed => {
            if !app.channels_settings.wecom_delivery.as_ref().is_some_and(|d| d.saving) {
                app.channels_settings.wecom_delivery = None;
                app.channels_settings.qr_flow.close();
            }
        }
        ChannelsMessage::WeComDeliveryChanged(key, value) => {
            if let Some(draft) = app.channels_settings.wecom_delivery.as_mut().filter(|d| !d.saving)
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
            if let Some(draft) = app.channels_settings.wecom_delivery.as_mut().filter(|d| !d.saving)
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
            if app.channels_settings.wecom_delivery.as_ref().is_none_or(|d| d.saving) {
                return Task::none();
            }
            let (wecom, webhook) = match credentials(app) {
                Ok(value) => value,
                Err(error) => {
                    app.channels_settings.wecom_delivery.as_mut().unwrap().error =
                        Some(error.clone());
                    return app.show_error_toast(error);
                }
            };
            let webhook_mode = app.channels_settings.wecom_delivery.as_ref().unwrap().mode
                == WeComDeliveryMode::Webhook;
            app.channels_settings.wecom_delivery.as_mut().unwrap().saving = true;
            app.channels_settings.qr_flow.close();
            return Task::perform(
                crate::app::config::update_channels_config_result_async(move |cfg| {
                    if webhook_mode {
                        cfg.wecom_webhook = webhook;
                    } else {
                        cfg.wecom = wecom;
                        if cfg.wecom.is_none() {
                            cfg.instance_default_projects.remove("wecom:default");
                            cfg.instance_execution.remove("wecom:default");
                        }
                    }
                }),
                |result| {
                    Message::Settings(SettingsMessage::Channels(
                        ChannelsMessage::WeComDeliverySaved(result),
                    ))
                },
            );
        }
        ChannelsMessage::WeComDeliverySaved(result) => {
            if app.channels_settings.wecom_delivery.is_none() {
                return Task::none();
            }
            match result {
                Ok(()) => {
                    let webhook_mode = app.channels_settings.wecom_delivery.as_ref().unwrap().mode
                        == WeComDeliveryMode::Webhook;
                    if let Ok((wecom, webhook)) = credentials(app) {
                        app.channels_settings.wecom = wecom;
                        app.channels_settings.wecom_webhook = webhook;
                        if !webhook_mode && app.channels_settings.wecom.is_none() {
                            app.channels_settings.instance_default_projects.remove("wecom:default");
                            app.channels_settings.instance_execution.remove("wecom:default");
                        }
                    }
                    app.channels_settings.wecom_delivery = None;
                    app.channels_settings.qr_flow.close();
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Success,
                        "已保存凭据",
                        vec![],
                    );
                }
                Err(_) => {
                    let draft = app.channels_settings.wecom_delivery.as_mut().unwrap();
                    draft.saving = false;
                    // The API can return credential-bearing URLs. Do not display its raw payload.
                    draft.error = Some("保存失败，请检查网关连接与配置后重试".into());
                    return app.show_error_toast("保存失败，请检查网关连接与配置后重试");
                }
            }
        }
        _ => {}
    }
    Task::none()
}
