//! 处理渠道设置子模块的状态变更、字段转换和持久化。

use crate::app::{App, Message};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use iced::Task;
use iced::widget::text_editor;
use serde_json::{Map, Value};
use std::time::Duration;
use vw_config_types::channels::{QQConfig, QQReceiveMode, WeChatConfig, WeComConfig};
use vw_gateway_client::{ChannelQrStatusDto, GatewayClient};

use super::super::messages::{ChannelsMessage, SettingsMessage};
use super::changes::{
    apply_bool_change, apply_number_change, apply_receive_mode_change, apply_text_change,
};
use super::persist::persist_channels_settings;
use super::toggles::{add_channel_instance, remove_channel_instance, toggle_enabled};

const MAX_QR_PNG_BYTES: usize = 1024 * 1024;

fn qr_client() -> Result<GatewayClient, String> {
    crate::app::config::gateway_client()
}

fn qr_fetch_task(channel: String, generation: u64) -> Task<Message> {
    let client = qr_client();
    let request_channel = channel.clone();
    Task::perform(
        async move {
            match client {
                Ok(client) => client.channel_qr_fetch(&request_channel).await,
                Err(error) => Err(error),
            }
        },
        move |result| {
            Message::Settings(SettingsMessage::Channels(ChannelsMessage::QrFetched {
                channel: channel.clone(),
                generation,
                result,
            }))
        },
    )
}

fn qr_poll_task(channel: String, handle: String, generation: u64) -> Task<Message> {
    let client = qr_client();
    let request_channel = channel.clone();
    Task::perform(
        async move {
            match client {
                Ok(client) => client.channel_qr_status(&request_channel, &handle).await,
                Err(error) => Err(error),
            }
        },
        move |result| {
            Message::Settings(SettingsMessage::Channels(ChannelsMessage::QrPolled {
                channel: channel.clone(),
                generation,
                result,
            }))
        },
    )
}

fn schedule_qr_poll(channel: String, generation: u64, delay: Duration) -> Task<Message> {
    crate::app::message::after(
        delay,
        Message::Settings(SettingsMessage::Channels(ChannelsMessage::QrPollTick {
            channel,
            generation,
        })),
    )
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
    app: &mut App,
    channel: &str,
    credentials: &Map<String, Value>,
) -> Result<(), String> {
    match channel {
        "dingtalk" => {
            let cfg = app
                .channels_settings
                .dingtalk
                .get_or_insert_with(super::helpers::default_dingtalk_config);
            cfg.client_id = credential(credentials, "client_id")?;
            cfg.client_secret = credential(credentials, "client_secret")?;
        }
        "feishu" => {
            let cfg = app
                .channels_settings
                .feishu
                .get_or_insert_with(super::helpers::default_feishu_config);
            cfg.app_id = credential(credentials, "app_id")?;
            cfg.app_secret = credential(credentials, "app_secret")?;
        }
        "qq" => {
            let app_id = credential(credentials, "app_id")?;
            let app_secret = credential(credentials, "app_secret")?;
            let cfg = app.channels_settings.qq.get_or_insert_with(|| QQConfig {
                app_id: String::new(),
                app_secret: String::new(),
                allowed_users: Vec::new(),
                receive_mode: QQReceiveMode::Webhook,
            });
            cfg.app_id = app_id;
            cfg.app_secret = app_secret;
        }
        "wecom" => {
            app.channels_settings.wecom = Some(WeComConfig {
                bot_id: credential(credentials, "bot_id")?,
                secret: credential(credentials, "secret")?,
            });
        }
        "wechat" => {
            app.channels_settings.wechat = Some(WeChatConfig {
                bot_token: credential(credentials, "bot_token")?,
                base_url: credential(credentials, "base_url")?,
            });
        }
        _ => return Err("该通道不支持扫码授权".to_string()),
    }
    app.channels_settings.refresh_text_inputs();
    Ok(())
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub(super) fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Channels(message) = message else {
        return Task::none();
    };

    if matches!(
        &message,
        ChannelsMessage::InstanceAdded(_)
            | ChannelsMessage::InstanceRemoved(_, _)
            | ChannelsMessage::EnabledToggled(_, _)
            | ChannelsMessage::TabSelected(_)
            | ChannelsMessage::Refresh
    ) || matches!(&message, ChannelsMessage::TextChanged(key, _) if key.ends_with(".id"))
    {
        app.channels_settings.invalidate_project_choices();
    }
    match message {
        ChannelsMessage::ProjectChoicesRequested(instance) => super::projects::load(app, instance),
        ChannelsMessage::ProjectChoicesLoaded { tenant, instance, generation, result } => {
            super::projects::loaded(app, tenant, instance, generation, result)
        }
        ChannelsMessage::DefaultProjectSelected { instance, project } => {
            super::projects::select(app, instance, project)
        }
        ChannelsMessage::Refresh => {
            app.channels_settings.save_error = None;
            Task::none()
        }
        ChannelsMessage::TabSelected(tab) => {
            if app.channels_settings.active_tab != tab {
                app.channels_settings.qr_flow.close();
            }
            app.channels_settings.active_tab = tab;
            Task::none()
        }
        ChannelsMessage::CliToggled(value) => {
            app.channels_settings.cli = value;
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::ProjectDirChanged(value) => {
            app.channels_settings.project_dir_input = value;
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::MessageTimeoutSecsChanged(value) => {
            app.channels_settings.message_timeout_secs = value.max(1);
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::EnabledToggled(channel, enabled) => {
            if app.channels_settings.qr_flow.channel.as_deref() == Some(channel.as_str()) {
                app.channels_settings.qr_flow.close();
            }
            toggle_enabled(app, &channel, enabled);
            if let Some(tab) = crate::app::state::ChannelsSettingsTab::from_channel_id(&channel) {
                app.channels_settings.active_tab = tab;
            }
            super::projects::retain_configured_bindings(app);
            app.channels_settings.refresh_text_inputs();
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::InstanceAdded(channel) => {
            add_channel_instance(app, &channel);
            if let Some(tab) = crate::app::state::ChannelsSettingsTab::from_channel_id(&channel) {
                app.channels_settings.active_tab = tab;
            }
            super::projects::retain_configured_bindings(app);
            app.channels_settings.refresh_text_inputs();
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::InstanceRemoved(channel, index) => {
            remove_channel_instance(app, &channel, index);
            super::projects::retain_configured_bindings(app);
            app.channels_settings.refresh_text_inputs();
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::TextChanged(key, value) => {
            app.channels_settings.text_inputs.insert(key.clone(), value.clone());
            apply_text_change(app, &key, value);
            super::projects::retain_configured_bindings(app);
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::TextEditorAction(key, action) => {
            let should_persist = matches!(action, text_editor::Action::Edit(_));
            if let Some(editor) = app.channels_settings.text_editors.get_mut(&key) {
                editor.perform(action);
                let value = editor.text();
                app.channels_settings.text_inputs.insert(key.clone(), value.clone());
                apply_text_change(app, &key, value);
            }
            if should_persist {
                persist_channels_settings(app).unwrap_or_else(Task::none)
            } else {
                Task::none()
            }
        }
        ChannelsMessage::BoolToggled(key, value) => {
            apply_bool_change(app, &key, value);
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::NumberChanged(key, value) => {
            apply_number_change(app, &key, value);
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::ReceiveModeChanged(key, value) => {
            apply_receive_mode_change(app, &key, value);
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::QrRequested(channel) => {
            if !matches!(channel.as_str(), "dingtalk" | "feishu" | "qq" | "wecom" | "wechat") {
                return app.show_error_toast("该通道不支持扫码授权");
            }
            app.channels_settings.qr_flow.close();
            app.channels_settings.qr_flow.channel = Some(channel.clone());
            let generation = app.channels_settings.qr_flow.generation;
            qr_fetch_task(channel, generation)
        }
        ChannelsMessage::QrFetched { channel, generation, result } => {
            if app.channels_settings.qr_flow.generation != generation
                || app.channels_settings.qr_flow.channel.as_deref() != Some(channel.as_str())
            {
                return Task::none();
            }
            match result {
                Ok(response) => {
                    if response.channel != channel {
                        app.channels_settings.qr_flow.close();
                        return app.show_error_toast("网关返回了不匹配的扫码通道");
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
                            app.channels_settings.qr_flow.qrcode_image =
                                Some(iced::widget::image::Handle::from_bytes(bytes));
                            app.channels_settings.qr_flow.handle = Some(response.handle);
                            app.channels_settings.qr_flow.expires_at = Some(response.expires_at);
                            app.channels_settings.qr_flow.status =
                                Some(ChannelQrStatusDto::Pending);
                            app.channels_settings.qr_flow.scan_observed = false;
                            schedule_qr_poll(channel, generation, Duration::from_secs(1))
                        }
                        Err(error) => {
                            app.channels_settings.qr_flow.close();
                            app.show_error_toast(error)
                        }
                    }
                }
                Err(error) => {
                    app.channels_settings.qr_flow.close();
                    app.show_error_toast(format!("获取二维码失败：{error}"))
                }
            }
        }
        ChannelsMessage::QrPollTick { channel, generation } => {
            let flow = &mut app.channels_settings.qr_flow;
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
            qr_poll_task(channel, handle, generation)
        }
        ChannelsMessage::QrPolled { channel, generation, result } => {
            if app.channels_settings.qr_flow.generation != generation
                || app.channels_settings.qr_flow.channel.as_deref() != Some(channel.as_str())
            {
                return Task::none();
            }
            app.channels_settings.qr_flow.poll_in_flight = false;
            match result {
                Ok(response) => match response.status {
                    ChannelQrStatusDto::Pending => {
                        app.channels_settings.qr_flow.status = Some(ChannelQrStatusDto::Pending);
                        schedule_qr_poll(channel, generation, Duration::from_secs(1))
                    }
                    ChannelQrStatusDto::Scanned => {
                        app.channels_settings.qr_flow.status = Some(ChannelQrStatusDto::Scanned);
                        app.channels_settings.qr_flow.scan_observed = true;
                        schedule_qr_poll(channel, generation, Duration::from_secs(1))
                    }
                    ChannelQrStatusDto::Authorized => {
                        app.channels_settings.qr_flow.status = Some(ChannelQrStatusDto::Authorized);
                        let result = response
                            .credentials
                            .as_ref()
                            .ok_or_else(|| "扫码授权确认但未返回凭据".to_string())
                            .and_then(|credentials| {
                                fill_confirmed_credentials(app, &channel, credentials)
                            });
                        match result {
                            Ok(()) => {
                                app.channels_settings.qr_flow.credentials_ready = true;
                                app.show_success_toast("扫码授权成功，请确认后保存")
                            }
                            Err(error) => {
                                app.channels_settings.qr_flow.close();
                                app.show_error_toast(error)
                            }
                        }
                    }
                    ChannelQrStatusDto::Expired => {
                        app.channels_settings.qr_flow.close();
                        app.show_warning_toast("二维码已过期，请重新获取")
                    }
                    ChannelQrStatusDto::Failed => {
                        if app.channels_settings.qr_flow.scan_observed {
                            app.channels_settings.qr_flow.close();
                            app.show_error_toast("扫码授权失败，请重新获取")
                        } else {
                            app.channels_settings.qr_flow.status =
                                Some(ChannelQrStatusDto::Pending);
                            schedule_qr_poll(channel, generation, Duration::from_secs(2))
                        }
                    }
                },
                Err(_) => schedule_qr_poll(channel, generation, Duration::from_secs(2)),
            }
        }
        ChannelsMessage::QrClosed => {
            app.channels_settings.qr_flow.close();
            Task::none()
        }
        ChannelsMessage::QrSaveConfirmed => {
            let task = persist_channels_settings(app).unwrap_or_else(Task::none);
            app.channels_settings.qr_flow.close();
            task
        }
    }
}

#[cfg(test)]
#[path = "updates_tests.rs"]
mod updates_tests;
