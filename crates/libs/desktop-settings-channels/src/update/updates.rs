//! 处理渠道设置子模块的状态变更、字段转换和持久化。

use crate::state::ChannelsSettingsState;
use super::{Effect as Message, Services};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use iced::Task;
use iced::widget::text_editor;
use serde_json::{Map, Value};
use std::time::Duration;
use config_channel_services::{QQConfig, QQReceiveMode, WeChatConfig, WeComConfig};
use gateway_channel_qr::{ChannelQrStatusDto, GatewayChannelQrClient};

use crate::messages::ChannelsMessage;
use super::changes::{
    apply_bool_change, apply_number_change, apply_receive_mode_change, apply_text_change,
};
use super::persist::persist_channels_settings;
use super::toggles::{add_channel_instance, remove_channel_instance, toggle_enabled};

const MAX_QR_PNG_BYTES: usize = 1024 * 1024;

fn qr_client(services: &Services) -> Result<GatewayChannelQrClient, String> {
    services.client.clone()
        .map(|client| GatewayChannelQrClient::new(client.transport().clone()))
}

fn qr_fetch_task(services: &Services, channel: String, generation: u64) -> Task<Message> {
    let client = qr_client(services);
    let request_channel = channel.clone();
    Task::perform(
        async move {
            match client {
                Ok(client) => client.channel_qr_fetch(&request_channel).await,
                Err(error) => Err(error),
            }
        },
        move |result| {
            Message::Channels(ChannelsMessage::QrFetched {
                channel: channel.clone(),
                generation,
                result,
            })
        },
    )
}

fn qr_poll_task(services: &Services, channel: String, handle: String, generation: u64) -> Task<Message> {
    let client = qr_client(services);
    let request_channel = channel.clone();
    Task::perform(
        async move {
            match client {
                Ok(client) => client.channel_qr_status(&request_channel, &handle).await,
                Err(error) => Err(error),
            }
        },
        move |result| {
            Message::Channels(ChannelsMessage::QrPolled {
                channel: channel.clone(),
                generation,
                result,
            })
        },
    )
}

fn schedule_qr_poll(channel: String, generation: u64, delay: Duration) -> Task<Message> {
    Task::done(Message::After(delay, ChannelsMessage::QrPollTick { channel, generation }))
}

fn credential(credentials: &Map<String, Value>, key: &str) -> Result<String, String> {
    credentials
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| format!("扫码授权未返回所需字段：{key}"))
}

fn fill_confirmed_credentials(
    app: &mut ChannelsSettingsState,
    channel: &str,
    credentials: &Map<String, Value>,
) -> Result<(), String> {
    if channel == "wecom" {
        if let Some(draft) = app.wecom_delivery.as_mut() {
            let bot_id = credential(credentials, "bot_id")?;
            let secret = credential(credentials, "secret")?;
            draft.bot_id = bot_id;
            draft.secret = secret;
            draft.clear_secret = false;
            return Ok(());
        }
    }
    match channel {
        "dingtalk" => {
            let cfg = app
                .dingtalk
                .get_or_insert_with(super::helpers::default_dingtalk_config);
            cfg.client_id = credential(credentials, "client_id")?;
            cfg.client_secret = credential(credentials, "client_secret")?;
            if cfg.allowed_users.is_empty() {
                cfg.allowed_users.push("*".to_string());
            }
        }
        "feishu" => {
            let cfg = app
                .feishu
                .get_or_insert_with(super::helpers::default_feishu_config);
            cfg.app_id = credential(credentials, "app_id")?;
            cfg.app_secret = credential(credentials, "app_secret")?;
            if cfg.allowed_users.is_empty() {
                cfg.allowed_users.push("*".to_string());
            }
        }
        "qq" => {
            let app_id = credential(credentials, "app_id")?;
            let app_secret = credential(credentials, "app_secret")?;
            let cfg = app.qq.get_or_insert_with(|| QQConfig {
                app_id: String::new(),
                app_secret: String::new(),
                allowed_users: Vec::new(),
                receive_mode: QQReceiveMode::Webhook,
            });
            cfg.app_id = app_id;
            cfg.app_secret = app_secret;
            if cfg.allowed_users.is_empty() {
                cfg.allowed_users.push("*".to_string());
            }
        }
        "wecom" => {
            app.wecom = Some(WeComConfig {
                bot_id: credential(credentials, "bot_id")?,
                secret: credential(credentials, "secret")?,
            });
        }
        "wechat" => {
            app.wechat = Some(WeChatConfig {
                bot_token: credential(credentials, "bot_token")?,
                base_url: credential(credentials, "base_url")?,
            });
        }
        _ => return Err("该通道不支持扫码授权".to_string()),
    }
    app.refresh_text_inputs();
    Ok(())
}

fn finish_qr(app: &mut ChannelsSettingsState, status: ChannelQrStatusDto) {
    app.qr_flow.close();
    if app.wecom_delivery.is_some() {
        app.qr_flow.channel = Some("wecom".into());
        app.qr_flow.status = Some(status);
    }
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut ChannelsSettingsState, services: &Services, message: ChannelsMessage) -> Task<Message> {

    let reload_bindings = matches!(
        &message,
        ChannelsMessage::InstanceAdded(_)
            | ChannelsMessage::InstanceRemoved(_, _)
            | ChannelsMessage::EnabledToggled(_, _)
            | ChannelsMessage::TabSelected(_)
            | ChannelsMessage::Refresh
    ) || matches!(&message, ChannelsMessage::TextChanged(key, _) if key.ends_with(".id"));
    let reload_execution = reload_bindings
        || matches!(
            &message,
            ChannelsMessage::ProjectDirChanged(_) | ChannelsMessage::QrSaveConfirmed
        );
    if reload_bindings {
        app.invalidate_project_choices();
    }
    let task = match message {
        ChannelsMessage::Effect(effect) => Task::done(*effect),
        message @ (ChannelsMessage::WeComDeliveryOpened(_)
        | ChannelsMessage::WeComDeliveryClosed
        | ChannelsMessage::WeComDeliveryChanged(_, _)
        | ChannelsMessage::WeComDeliveryClearSecret
        | ChannelsMessage::WeComDeliverySave
        | ChannelsMessage::WeComDeliverySaved(_)) => Task::done(Message::WeComDelivery(message)),
        ChannelsMessage::ExecutionChoicesRequested(instance) => {
            super::execution::load(app, services, instance)
        }
        ChannelsMessage::ExecutionChoicesLoaded { tenant, instance, generation, result } => {
            super::execution::loaded(app, services, tenant, instance, generation, result)
        }
        ChannelsMessage::ExecutionAgentSelected { instance, agent } => {
            super::execution::select_agent(app, services, instance, agent)
        }
        ChannelsMessage::ExecutionProviderSelected { instance, provider } => {
            super::execution::select_provider(app, services, instance, provider)
        }
        ChannelsMessage::ExecutionModelSelected { instance, model } => {
            super::execution::select_model(app, services, instance, model)
        }
        ChannelsMessage::ProjectChoicesRequested(instance) => super::projects::load(app, services, instance),
        ChannelsMessage::ProjectChoicesLoaded { tenant, instance, generation, result } => {
            super::projects::loaded(app, services, tenant, instance, generation, result)
        }
        ChannelsMessage::DefaultProjectSelected { instance, project } => {
            super::projects::select(app, services, instance, project)
        }
        ChannelsMessage::Refresh => {
            app.save_error = None;
            Task::none()
        }
        ChannelsMessage::TabSelected(tab) => {
            if app.active_tab != tab {
                app.qr_flow.close();
            }
            app.active_tab = tab;
            super::projects::load_active(app, services)
        }
        ChannelsMessage::CliToggled(value) => {
            app.cli = value;
            persist_channels_settings(app, services).unwrap_or_else(Task::none)
        }
        ChannelsMessage::ProjectDirChanged(value) => {
            app.invalidate_execution_choices();
            app.project_dir_input = value;
            persist_channels_settings(app, services).unwrap_or_else(Task::none)
        }
        ChannelsMessage::MessageTimeoutSecsChanged(value) => {
            app.message_timeout_secs = value.max(1);
            persist_channels_settings(app, services).unwrap_or_else(Task::none)
        }
        ChannelsMessage::EnabledToggled(channel, enabled) => {
            if app.qr_flow.channel.as_deref() == Some(channel.as_str()) {
                app.qr_flow.close();
            }
            toggle_enabled(app, &channel, enabled);
            if let Some(tab) = crate::state::ChannelsSettingsTab::from_channel_id(&channel) {
                app.active_tab = tab;
            }
            super::projects::retain_configured_bindings(app);
            app.refresh_text_inputs();
            persist_channels_settings(app, services).unwrap_or_else(Task::none)
        }
        ChannelsMessage::InstanceAdded(channel) => {
            add_channel_instance(app, &channel);
            if let Some(tab) = crate::state::ChannelsSettingsTab::from_channel_id(&channel) {
                app.active_tab = tab;
            }
            super::projects::retain_configured_bindings(app);
            app.refresh_text_inputs();
            persist_channels_settings(app, services).unwrap_or_else(Task::none)
        }
        ChannelsMessage::InstanceRemoved(channel, index) => {
            remove_channel_instance(app, &channel, index);
            super::projects::retain_configured_bindings(app);
            app.refresh_text_inputs();
            persist_channels_settings(app, services).unwrap_or_else(Task::none)
        }
        ChannelsMessage::TextChanged(key, value) => {
            app.text_inputs.insert(key.clone(), value.clone());
            apply_text_change(app, &key, value);
            super::projects::retain_configured_bindings(app);
            persist_channels_settings(app, services).unwrap_or_else(Task::none)
        }
        ChannelsMessage::TextEditorAction(key, action) => {
            let should_persist = matches!(action, text_editor::Action::Edit(_));
            if let Some(editor) = app.text_editors.get_mut(&key) {
                editor.perform(action);
                let value = editor.text();
                app.text_inputs.insert(key.clone(), value.clone());
                apply_text_change(app, &key, value);
            }
            if should_persist {
                persist_channels_settings(app, services).unwrap_or_else(Task::none)
            } else {
                Task::none()
            }
        }
        ChannelsMessage::BoolToggled(key, value) => {
            apply_bool_change(app, &key, value);
            persist_channels_settings(app, services).unwrap_or_else(Task::none)
        }
        ChannelsMessage::NumberChanged(key, value) => {
            apply_number_change(app, &key, value);
            persist_channels_settings(app, services).unwrap_or_else(Task::none)
        }
        ChannelsMessage::ReceiveModeChanged(key, value) => {
            apply_receive_mode_change(app, &key, value);
            persist_channels_settings(app, services).unwrap_or_else(Task::none)
        }
        ChannelsMessage::QrRequested(channel) => {
            if !matches!(channel.as_str(), "dingtalk" | "feishu" | "qq" | "wecom" | "wechat") {
                return super::localized_toast(
                    super::ToastKind::Error,
                    "该通道不支持扫码授权",
                    vec![],
                );
            }
            app.qr_flow.close();
            app.qr_flow.channel = Some(channel.clone());
            let generation = app.qr_flow.generation;
            qr_fetch_task(services, channel, generation)
        }
        ChannelsMessage::QrFetched { channel, generation, result } => {
            if app.qr_flow.generation != generation
                || app.qr_flow.channel.as_deref() != Some(channel.as_str())
            {
                return Task::none();
            }
            match result {
                Ok(response) => {
                    if response.channel != channel {
                        app.qr_flow.close();
                        return super::localized_toast(
                            super::ToastKind::Error,
                            "网关返回了不匹配的扫码通道",
                            vec![],
                        );
                    }
                    let decoded = BASE64_STANDARD
                        .decode(response.qrcode_png_base64.as_bytes())
                        .map_err(|_| "网关返回了无效的二维码图片".to_string())
                        .and_then(|bytes| {
                            if bytes.is_empty() || bytes.len() > MAX_QR_PNG_BYTES {
                                Err("二维码图片为空或超过 1 MiB 限制".to_string())
                            } else {
                                Ok(bytes)
                            }
                        });
                    match decoded {
                        Ok(bytes) => {
                            app.qr_flow.qrcode_image =
                                Some(iced::widget::image::Handle::from_bytes(bytes));
                            app.qr_flow.handle = Some(response.handle);
                            app.qr_flow.expires_at = Some(response.expires_at);
                            app.qr_flow.status =
                                Some(ChannelQrStatusDto::Pending);
                            app.qr_flow.scan_observed = false;
                            schedule_qr_poll(channel, generation, Duration::from_secs(1))
                        }
                        Err(error) => {
                            app.qr_flow.close();
                            super::error_toast(error)
                        }
                    }
                }
                Err(error) => {
                    finish_qr(app, ChannelQrStatusDto::Failed);
                    super::localized_toast(
                        super::ToastKind::Error,
                        "获取二维码失败：%s",
                        vec![error.into()],
                    )
                }
            }
        }
        ChannelsMessage::QrPollTick { channel, generation } => {
            let flow = &mut app.qr_flow;
            if flow.generation != generation
                || flow.channel.as_deref() != Some(channel.as_str())
                || flow.poll_in_flight
            {
                return Task::none();
            }
            let Some(handle) = flow.handle.clone() else {
                return Task::none();
            };
            flow.poll_in_flight = true;
            qr_poll_task(services, channel, handle, generation)
        }
        ChannelsMessage::QrPolled { channel, generation, result } => {
            if app.qr_flow.generation != generation
                || app.qr_flow.channel.as_deref() != Some(channel.as_str())
            {
                return Task::none();
            }
            app.qr_flow.poll_in_flight = false;
            match result {
                Ok(response) => match response.status {
                    ChannelQrStatusDto::Pending => {
                        app.qr_flow.status = Some(ChannelQrStatusDto::Pending);
                        schedule_qr_poll(channel, generation, Duration::from_secs(1))
                    }
                    ChannelQrStatusDto::Scanned => {
                        app.qr_flow.status = Some(ChannelQrStatusDto::Scanned);
                        app.qr_flow.scan_observed = true;
                        schedule_qr_poll(channel, generation, Duration::from_secs(1))
                    }
                    ChannelQrStatusDto::Authorized => {
                        app.qr_flow.status = Some(ChannelQrStatusDto::Authorized);
                        let result = response
                            .credentials
                            .as_ref()
                            .ok_or_else(|| "扫码授权确认但未返回凭据".to_string())
                            .and_then(|credentials| {
                                fill_confirmed_credentials(app, &channel, credentials)
                            });
                        match result {
                            Ok(()) => {
                                app.qr_flow.credentials_ready = true;
                                super::localized_toast(
                                    super::ToastKind::Success,
                                    "扫码授权成功，请确认后保存",
                                    vec![],
                                )
                            }
                            Err(error) => {
                                app.qr_flow.close();
                                super::error_toast(error)
                            }
                        }
                    }
                    ChannelQrStatusDto::Expired => {
                        finish_qr(app, ChannelQrStatusDto::Expired);
                        super::localized_toast(
                            super::ToastKind::Warning,
                            "二维码已过期，请重新获取",
                            vec![],
                        )
                    }
                    ChannelQrStatusDto::Failed => {
                        if app.qr_flow.scan_observed {
                            finish_qr(app, ChannelQrStatusDto::Failed);
                            super::localized_toast(
                                super::ToastKind::Error,
                                "扫码授权失败，请重新获取",
                                vec![],
                            )
                        } else {
                            app.qr_flow.status =
                                Some(ChannelQrStatusDto::Pending);
                            schedule_qr_poll(channel, generation, Duration::from_secs(2))
                        }
                    }
                },
                Err(_) => schedule_qr_poll(channel, generation, Duration::from_secs(2)),
            }
        }
        ChannelsMessage::QrClosed if app.wecom_delivery.is_some() => {
            Task::done(Message::WeComDelivery(ChannelsMessage::WeComDeliveryClosed))
        }
        ChannelsMessage::QrSaveConfirmed if app.wecom_delivery.is_some() => {
            Task::done(Message::WeComDelivery(ChannelsMessage::WeComDeliverySave))
        }
        ChannelsMessage::QrClosed => {
            app.qr_flow.close();
            Task::none()
        }
        ChannelsMessage::QrSaveConfirmed => {
            let task = persist_channels_settings(app, services).unwrap_or_else(Task::none);
            app.qr_flow.close();
            task
        }
    };
    if reload_execution { Task::batch([task, super::execution::load_active(app, services)]) } else { task }
}
