//! Transactional connection editor for WeCom smart bots.
use crate::messages::ChannelsMessage;
use crate::state::ChannelsSettingsState;
use super::{Effect as Message, Services, ToastKind};
use crate::state::{WeComCronNameDraft, WeComDeliveryDraft, WeComDeliveryMode};
use iced::Task;
use config_channel_services::WeComConfig;

pub(super) fn credentials(s: &ChannelsSettingsState) -> Result<Option<WeComConfig>, String> {
    let draft = s.wecom_delivery.as_ref().ok_or("企微配置弹窗未打开")?;
    if draft.clear_secret {
        return Ok(None);
    }
    let secret = if draft.secret.is_empty() {
        s.wecom_connection(draft.instance_id.as_deref())
            .map(|cfg| cfg.secret.clone())
            .unwrap_or_default()
    } else {
        draft.secret.clone()
    };
    if draft.bot_id.trim().is_empty() || secret.trim().is_empty() {
        return Err("请填写 Bot ID 和 Secret，或完成扫码授权".into());
    }
    Ok(Some(WeComConfig { bot_id: draft.bot_id.trim().into(), secret }))
}

fn instance_credentials(config: Option<WeComConfig>) -> WeComConfig {
    config.unwrap_or_else(|| WeComConfig {
        bot_id: String::new(),
        secret: String::new(),
    })
}

fn open(
    app: &mut ChannelsSettingsState,
    services: &Services,
    mode: WeComDeliveryMode,
    instance_id: Option<String>,
) -> Task<Message> {
    if app.wecom_delivery.as_ref().is_some_and(|draft| draft.saving) {
        return Task::none();
    }
    let bot = app.wecom_connection(instance_id.as_deref());
    if instance_id.as_deref().is_some_and(|id| !WeComConfig::is_cron_instance(id)) && bot.is_none() {
        return Task::none();
    }
    let bot_id = bot.map(|config| config.bot_id.clone()).unwrap_or_default();
    app.qr_flow.close();
    app.wecom_delivery = Some(WeComDeliveryDraft {
        mode,
        instance_id,
        bot_id,
        ..Default::default()
    });
    if mode == WeComDeliveryMode::Qr {
        return super::updates::update(app, services, ChannelsMessage::QrRequested("wecom".into()));
    }
    Task::none()
}

pub fn update(app: &mut ChannelsSettingsState, services: &Services, message: ChannelsMessage) -> Task<Message> {
    match message {
        ChannelsMessage::WeComCronInstanceAdded => {
            let id = format!("{}-{}", WeComConfig::CRON_INSTANCE, uuid::Uuid::new_v4().simple());
            return open(app, services, WeComDeliveryMode::Qr, Some(id));
        }
        ChannelsMessage::WeComCronNameOpened(instance) => {
            if app.wecom_cron_name.is_some()
                || !WeComConfig::is_cron_instance(&instance)
                || !app.wecom_connection(Some(&instance))
                    .is_some_and(WeComConfig::has_required_credentials)
            {
                return Task::none();
            }
            let value = app.wecom_cron_display_names.get(&instance).cloned().unwrap_or_default();
            app.wecom_cron_name = Some(WeComCronNameDraft { instance, value, saving: false });
        }
        ChannelsMessage::WeComCronNameChanged(value) => {
            if let Some(draft) = app.wecom_cron_name.as_mut().filter(|draft| !draft.saving) {
                draft.value = value;
            }
        }
        ChannelsMessage::WeComCronNameClosed => {
            if app.wecom_cron_name.as_ref().is_some_and(|draft| !draft.saving) {
                app.wecom_cron_name = None;
            }
        }
        ChannelsMessage::WeComCronNameSave => {
            let Some(draft) = app.wecom_cron_name.as_mut().filter(|draft| !draft.saving) else {
                return Task::none();
            };
            let name = draft.value.trim();
            let previous = app.wecom_cron_display_names
                .get(&draft.instance)
                .map_or("", |name| name.trim());
            if name == previous {
                app.wecom_cron_name = None;
                return Task::none();
            }
            let instance = draft.instance.clone();
            let patch = serde_json::json!({
                "channels_config": { "wecom_cron_display_names": { (instance.clone()): name } }
            });
            draft.saving = true;
            let config = services.config.clone();
            return Task::perform(
                async move { config.patch_full_agent_config_async(patch).await },
                move |result| Message::Channels(ChannelsMessage::WeComCronNameSaved {
                    instance: instance.clone(),
                    result,
                }),
            );
        }
        ChannelsMessage::WeComCronNameSaved { instance, result } => {
            let Some(draft) = app.wecom_cron_name.as_mut()
                .filter(|draft| draft.instance == instance && draft.saving)
            else {
                return Task::none();
            };
            if result.is_err() {
                draft.saving = false;
                return super::error_toast("保存失败，请检查网关连接与配置后重试".into());
            }
            let name = draft.value.trim();
            if name.is_empty() {
                app.wecom_cron_display_names.remove(&instance);
            } else {
                app.wecom_cron_display_names.insert(instance, name.to_owned());
            }
            app.wecom_cron_name = None;
            return super::localized_toast(ToastKind::Success, "已保存", vec![]);
        }
        ChannelsMessage::WeComCronBindingRequested { instance, start } => {
            if app.wecom_cron_binding_requests.contains_key(&instance)
                || !WeComConfig::is_cron_instance(&instance)
                || !app.wecom_connection(Some(&instance))
                    .is_some_and(WeComConfig::has_required_credentials)
            {
                return Task::none();
            }
            app.wecom_cron_binding_request += 1;
            let request = app.wecom_cron_binding_request;
            app.wecom_cron_binding_requests.insert(instance.clone(), request);
            let client = services.client.clone();
            let query = [("instance".into(), instance.clone())];
            return Task::perform(async move {
                let client = client?;
                let path = "/v1/cron/wecom-binding";
                if start {
                    client.transport().post_json(path, &query, &()).await
                } else {
                    client.transport().get_json(path, &query).await
                }
            }, move |result| Message::Channels(ChannelsMessage::WeComCronBindingLoaded {
                instance: instance.clone(), request, result,
            }));
        }
        ChannelsMessage::WeComCronBindingLoaded { instance, request, result } => {
            if app.wecom_cron_binding_requests.get(&instance) != Some(&request) {
                return Task::none();
            }
            app.wecom_cron_binding_requests.remove(&instance);
            match result {
                Ok(binding) => {
                    let status = app.wecom_cron_bindings.entry(instance).or_default();
                    if let Some(bound) = binding.bound {
                        status.bound = Some(bound);
                    }
                    if binding.pairing_pending == Some(false) {
                        status.pairing_code = None;
                    }
                    if binding.connected.is_some() {
                        status.connected = binding.connected;
                    }
                    if let Some(code) = binding.pairing_code {
                        status.pairing_code = Some(code);
                    }
                }
                Err(error) => {
                    let disconnected = error
                        .strip_prefix("gateway request failed: 400 Bad Request ")
                        .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
                        .is_some_and(|body| body["error"] == "企微定时任务机器人尚未连接，请稍后重试");
                    if disconnected {
                        let status = app.wecom_cron_bindings.entry(instance).or_default();
                        status.connected = Some(false);
                        status.pairing_code = None;
                        return super::localized_toast(ToastKind::Error, "定时任务机器人尚未连接", vec![]);
                    }
                    return super::error_toast("无法读取企微定时任务连接，请稍后重试".into());
                }
            }
        }
        ChannelsMessage::WeComDeliveryOpened(mode) => return open(app, services, mode, None),
        ChannelsMessage::WeComInstanceOpened(id, mode) => return open(app, services, mode, Some(id)),
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
                    _ => return Task::none(),
                }
                draft.error = None;
            }
        }
        ChannelsMessage::WeComDeliveryClearSecret => {
            if let Some(draft) = app.wecom_delivery.as_mut().filter(|d| !d.saving)
            {
                draft.clear_secret = true;
                draft.secret.clear();
            }
        }
        ChannelsMessage::WeComDeliverySave => {
            if app.wecom_delivery.as_ref().is_none_or(|d| d.saving) {
                return Task::none();
            }
            let wecom = match credentials(app) {
                Ok(value) => value,
                Err(error) => {
                    app.wecom_delivery.as_mut().unwrap().error =
                        Some(error.clone());
                    return super::error_toast(error);
                }
            };
            let draft = app.wecom_delivery.as_mut().unwrap();
            let instance_id = draft.instance_id.clone();
            let cron = instance_id.as_deref().is_some_and(WeComConfig::is_cron_instance);
            if let Some(bot) = &wecom
                && ((cron && app.wecom.iter().any(|chat| chat.bot_id == bot.bot_id))
                    || app.wecom_instances.iter().any(|instance| {
                        instance_id.as_deref() != Some(instance.id.as_str())
                            && (cron || WeComConfig::is_cron_instance(&instance.id))
                            && instance.config.bot_id == bot.bot_id
                    }))
            {
                let error = "定时任务需使用独立的企业微信机器人".to_owned();
                draft.error = Some(error.clone());
                return super::error_toast(error);
            }
            let disconnect = instance_id.as_ref().filter(|_| cron && wecom.is_none()).cloned();
            draft.saving = true;
            app.qr_flow.close();
            let config = services.config.clone();
            let client = services.client.clone();
            return Task::perform(
                async move {
                    if let Some(id) = &disconnect {
                        let query = [("instance".into(), id.clone())];
                        client?.transport().delete_empty("/v1/cron/wecom-binding", &query).await?;
                    }
                    config.update_channels_config_result_async(move |cfg| {
                    if let Some(id) = disconnect {
                        cfg.wecom_instances.retain(|instance| instance.id != id);
                    } else if let Some(id) = instance_id {
                        let config = instance_credentials(wecom);
                        if let Some(instance) = cfg.wecom_instances.iter_mut().find(|instance| instance.id == id) {
                            instance.config = config;
                        } else {
                            cfg.wecom_instances.push(config_channel_services::WeComInstanceConfig { id, config });
                        }
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
                    let draft = app.wecom_delivery.as_ref().unwrap();
                    if let Ok(wecom) = credentials(app) {
                        if let Some(id) = &draft.instance_id {
                            let id = id.clone();
                            let cron = WeComConfig::is_cron_instance(&id);
                            if cron && wecom.is_none() {
                                app.wecom_cron_display_names.remove(&id);
                                if app.wecom_cron_name.as_ref().is_some_and(|draft| draft.instance == id) {
                                    app.wecom_cron_name = None;
                                }
                                while let Some(index) = app.wecom_instances.iter().rposition(|instance| {
                                    instance.id == id
                                }) {
                                    app.wecom_instances.remove(index);
                                    app.remove_instance_card("wecom", index);
                                }
                            } else {
                                let config = instance_credentials(wecom);
                                if let Some(instance) = app.wecom_instances.iter_mut().find(|instance| instance.id == id) {
                                    instance.config = config;
                                } else if cron {
                                    app.wecom_instances.push(config_channel_services::WeComInstanceConfig { id: id.clone(), config });
                                }
                            }
                            if cron {
                                app.wecom_cron_bindings.remove(&id);
                                app.wecom_cron_binding_requests.remove(&id);
                            }
                        } else {
                            app.wecom = wecom;
                            if app.wecom.is_none() {
                                app.instance_default_projects.remove("wecom:default");
                                app.instance_execution.remove("wecom:default");
                            }
                        }
                        app.refresh_text_inputs();
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
