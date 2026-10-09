//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::config::update_gateway_config_async;
use crate::app::message::settings::util::parse_comma_or_newline_list;
use crate::app::{App, Message};
use chrono::{Datelike, NaiveDate, Utc};
use iced::Task;
use sha2::{Digest, Sha256};
use std::time::Duration;
use vw_config_types::gateway::GatewaySkey;

use super::messages::{GatewayMessage, SettingsMessage};

fn normalize_host(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() { "127.0.0.1".to_string() } else { trimmed.to_string() }
}

#[cfg(test)]
#[path = "gateway_tests.rs"]
mod gateway_tests;

fn clamp_token_limit(value: u32) -> u32 {
    value.clamp(1, 100_000)
}

fn hash_skey(skey: &str) -> String {
    hex::encode(Sha256::digest(skey.trim().as_bytes()))
}

fn generate_skey() -> String {
    let mut bytes = [0_u8; 24];
    rand::fill(&mut bytes);
    format!("sk-{}", hex::encode(bytes))
}

fn mask_skey_for_display(skey: &str) -> String {
    let trimmed = skey.trim();
    let chars = trimmed.chars().collect::<Vec<_>>();
    if chars.len() <= 25 {
        return trimmed.to_string();
    }
    let prefix = chars.iter().take(16).collect::<String>();
    let suffix =
        chars.iter().rev().take(9).collect::<Vec<_>>().into_iter().rev().collect::<String>();
    format!("{prefix}{}{suffix}", "*".repeat(15))
}

fn normalize_skey_name(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("skey 名称不能为空".to_string());
    }
    if trimmed.chars().count() > 80 {
        return Err("skey 名称不能超过 80 个字符".to_string());
    }
    Ok(trimmed.to_string())
}

fn parse_date_input(raw: &str) -> Option<NaiveDate> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").ok().or_else(|| {
        chrono::DateTime::parse_from_rfc3339(trimmed).ok().map(|value| value.date_naive())
    })
}

fn month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date)
}

fn parse_calendar_month(raw: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(&format!("{}-01", raw.trim()), "%Y-%m-%d").ok()
}

fn current_calendar_month(app: &App) -> NaiveDate {
    parse_calendar_month(&app.gateway_settings.new_skey_calendar_month)
        .or_else(|| {
            parse_date_input(&app.gateway_settings.new_skey_expires_at_input).map(month_start)
        })
        .unwrap_or_else(|| month_start(Utc::now().date_naive()))
}

fn shift_calendar_month(month: NaiveDate, delta: i32) -> NaiveDate {
    let month_index = month.year() * 12 + month.month0() as i32 + delta;
    let year = month_index.div_euclid(12);
    let month = month_index.rem_euclid(12) as u32 + 1;
    NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(month_start(Utc::now().date_naive()))
}

fn set_calendar_month(app: &mut App, month: NaiveDate) {
    app.gateway_settings.new_skey_calendar_month = month.format("%Y-%m").to_string();
}

fn normalize_expires_at(raw: &str) -> Result<Option<String>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if chrono::DateTime::parse_from_rfc3339(trimmed).is_ok() {
        return Ok(Some(trimmed.to_string()));
    }
    let date = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .map_err(|_| "过期日期必须从日历选择，或使用 2026-12-31 格式".to_string())?;
    Ok(Some(format!("{}T23:59:59Z", date.format("%Y-%m-%d"))))
}

fn add_skey(app: &mut App) -> Task<Message> {
    let name = match normalize_skey_name(&app.gateway_settings.new_skey_name_input) {
        Ok(value) => value,
        Err(err) => {
            app.gateway_settings.save_error = Some(err);
            return Task::none();
        }
    };

    let expires_at = match normalize_expires_at(&app.gateway_settings.new_skey_expires_at_input) {
        Ok(value) => value,
        Err(err) => {
            app.gateway_settings.save_error = Some(err);
            return Task::none();
        }
    };

    let (skey, skey_hash) = loop {
        let candidate = generate_skey();
        let candidate_hash = hash_skey(&candidate);
        if !app.gateway_settings.skeys.iter().any(|existing| existing.skey_hash == candidate_hash) {
            break (candidate, candidate_hash);
        }
    };

    app.gateway_settings.skeys.push(GatewaySkey {
        enabled: true,
        skey: None,
        skey_hash,
        masked_skey: mask_skey_for_display(&skey),
        name,
        expires_at,
    });
    app.gateway_settings.new_skey_name_input.clear();
    app.gateway_settings.new_skey_expires_at_input.clear();
    app.gateway_settings.new_skey_calendar_open = false;
    app.gateway_settings.last_created_skey = Some(skey);
    app.gateway_settings.last_created_skey_copied = false;
    app.gateway_settings.save_error = None;
    persist_gateway_settings(app)
}

fn persist_gateway_settings(app: &mut App) -> Task<Message> {
    let s = &app.gateway_settings;
    let port = s.port.clamp(1, u16::MAX);
    let auth_enabled = s.auth_enabled;
    let allow_public_bind = s.allow_public_bind;
    let webhook_rate_limit_per_minute = s.webhook_rate_limit_per_minute.clamp(1, 100_000);
    let trust_forwarded_headers = s.trust_forwarded_headers;
    let rate_limit_max_keys = clamp_token_limit(s.rate_limit_max_keys) as usize;
    let idempotency_ttl_secs = s.idempotency_ttl_secs.clamp(1, 86_400) as u64;
    let idempotency_max_keys = clamp_token_limit(s.idempotency_max_keys) as usize;
    let node_control_enabled = s.node_control_enabled;
    let host = normalize_host(&s.host_input);
    let skeys = s
        .skeys
        .iter()
        .filter(|entry| !entry.skey_hash.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    let node_control_auth_token = s.node_control_auth_token_input.trim().to_string();
    let allowed_node_ids = parse_comma_or_newline_list(&s.node_control_allowed_node_ids_input);

    update_gateway_config_async(move |gateway| {
        gateway.port = port;
        gateway.host = host;
        gateway.auth_enabled = auth_enabled;
        gateway.allow_public_bind = allow_public_bind;
        gateway.skeys = skeys;
        gateway.require_pairing = false;
        gateway.paired_tokens.clear();
        gateway.webhook_rate_limit_per_minute = webhook_rate_limit_per_minute;
        gateway.trust_forwarded_headers = trust_forwarded_headers;
        gateway.rate_limit_max_keys = rate_limit_max_keys;
        gateway.idempotency_ttl_secs = idempotency_ttl_secs;
        gateway.idempotency_max_keys = idempotency_max_keys;
        gateway.node_control.enabled = node_control_enabled;
        gateway.node_control.auth_token =
            if node_control_auth_token.is_empty() { None } else { Some(node_control_auth_token) };
        gateway.node_control.allowed_node_ids = allowed_node_ids;
    })
}

fn service_command_task(command: String) -> Task<Message> {
    let request_command = command.clone();
    Task::perform(
        async move {
            let client = crate::app::config::gateway_client()?;
            client.desktop_service_command(&request_command).await.map(|response| response.output)
        },
        move |result| {
            Message::Settings(SettingsMessage::Gateway(GatewayMessage::ServiceCommandCompleted(
                command.clone(),
                result,
            )))
        },
    )
}

fn should_recover_after_service_command(command: &str) -> bool {
    matches!(command, "start" | "restart")
}

fn is_danger_service_command(command: &str) -> bool {
    matches!(command, "stop" | "uninstall")
}

fn service_recovery_task(command: String) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let request_command = command.clone();
        Task::perform(
            async move {
                if request_command == "restart" {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
                crate::app::config::bootstrap_cli_service_async().await
            },
            move |result| {
                Message::Settings(SettingsMessage::Gateway(
                    GatewayMessage::ServiceRecoveryCompleted(command.clone(), result),
                ))
            },
        )
    }

    #[cfg(target_arch = "wasm32")]
    {
        Task::done(Message::Settings(SettingsMessage::Gateway(
            GatewayMessage::ServiceRecoveryCompleted(
                command,
                Err("wasm 环境不支持本机 gateway 自动恢复".to_string()),
            ),
        )))
    }
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Gateway(message) = message else {
        return Task::none();
    };

    if matches!(message, GatewayMessage::Refresh) {
        app.gateway_settings.save_error = None;
        return Task::none();
    }

    match message {
        GatewayMessage::TabSelected(tab) => {
            app.gateway_settings.active_tab = tab;
            app.gateway_settings.new_skey_calendar_open = false;
            return Task::none();
        }
        GatewayMessage::PortChanged(value) => app.gateway_settings.port = value.clamp(1, u16::MAX),
        GatewayMessage::HostChanged(value) => app.gateway_settings.host_input = value,
        GatewayMessage::AuthEnabledToggled(value) => app.gateway_settings.auth_enabled = value,
        GatewayMessage::NewSkeyNameChanged(value) | GatewayMessage::NewSkeyChanged(value) => {
            app.gateway_settings.new_skey_name_input = value;
            return Task::none();
        }
        GatewayMessage::NewSkeyExpiresAtChanged(value) => {
            if let Some(date) = parse_date_input(&value) {
                set_calendar_month(app, month_start(date));
            }
            app.gateway_settings.new_skey_expires_at_input = value;
            return Task::none();
        }
        GatewayMessage::NewSkeyCalendarToggled => {
            app.gateway_settings.new_skey_calendar_open =
                !app.gateway_settings.new_skey_calendar_open;
            return Task::none();
        }
        GatewayMessage::NewSkeyCalendarClosed => {
            app.gateway_settings.new_skey_calendar_open = false;
            return Task::none();
        }
        GatewayMessage::NewSkeyExpiresDateSelected(value) => {
            if let Some(date) = parse_date_input(&value) {
                set_calendar_month(app, month_start(date));
            }
            app.gateway_settings.new_skey_expires_at_input = value;
            app.gateway_settings.new_skey_calendar_open = false;
            return Task::none();
        }
        GatewayMessage::NewSkeyExpiresMonthChanged(delta) => {
            let month = shift_calendar_month(current_calendar_month(app), delta);
            set_calendar_month(app, month);
            return Task::none();
        }
        GatewayMessage::NewSkeyExpiresAtCleared => {
            app.gateway_settings.new_skey_expires_at_input.clear();
            app.gateway_settings.new_skey_calendar_open = false;
            return Task::none();
        }
        GatewayMessage::AddSkey => return add_skey(app),
        GatewayMessage::CopyLastCreatedSkey => {
            let Some(skey) = app.gateway_settings.last_created_skey.clone() else {
                return Task::none();
            };
            app.gateway_settings.last_created_skey_copied = true;
            return Task::batch(vec![
                iced::clipboard::write(skey),
                crate::app::message::after(
                    Duration::from_secs(2),
                    Message::Settings(SettingsMessage::Gateway(
                        GatewayMessage::ClearLastCreatedSkeyCopied,
                    )),
                ),
            ]);
        }
        GatewayMessage::ClearLastCreatedSkeyCopied => {
            app.gateway_settings.last_created_skey_copied = false;
            return Task::none();
        }
        GatewayMessage::SkeyEnabledToggled(index, enabled) => {
            if let Some(skey) = app.gateway_settings.skeys.get_mut(index) {
                skey.enabled = enabled;
            }
        }
        GatewayMessage::RemoveSkey(index) => {
            if index < app.gateway_settings.skeys.len() {
                app.gateway_settings.skeys.remove(index);
            }
        }
        GatewayMessage::RequirePairingToggled(value) => {
            app.gateway_settings.auth_enabled = value;
        }
        GatewayMessage::AllowPublicBindToggled(value) => {
            app.gateway_settings.allow_public_bind = value
        }
        GatewayMessage::TrustForwardedHeadersToggled(value) => {
            app.gateway_settings.trust_forwarded_headers = value
        }
        GatewayMessage::NewPairedTokenChanged(value) => {
            app.gateway_settings.new_skey_name_input = value;
            return Task::none();
        }
        GatewayMessage::AddPairedToken => return add_skey(app),
        GatewayMessage::RemovePairedToken(index) => {
            if index < app.gateway_settings.skeys.len() {
                app.gateway_settings.skeys.remove(index);
            }
        }
        GatewayMessage::PairRateLimitPerMinuteChanged(_) => {}
        GatewayMessage::WebhookRateLimitPerMinuteChanged(value) => {
            app.gateway_settings.webhook_rate_limit_per_minute = value.clamp(1, 100_000)
        }
        GatewayMessage::RateLimitMaxKeysChanged(value) => {
            app.gateway_settings.rate_limit_max_keys = clamp_token_limit(value)
        }
        GatewayMessage::IdempotencyTtlSecsChanged(value) => {
            app.gateway_settings.idempotency_ttl_secs = value.clamp(1, 86_400)
        }
        GatewayMessage::IdempotencyMaxKeysChanged(value) => {
            app.gateway_settings.idempotency_max_keys = clamp_token_limit(value)
        }
        GatewayMessage::NodeControlEnabledToggled(value) => {
            app.gateway_settings.node_control_enabled = value
        }
        GatewayMessage::NodeControlAuthTokenChanged(value) => {
            app.gateway_settings.node_control_auth_token_input = value
        }
        GatewayMessage::NodeControlAllowedNodeIdsChanged(value) => {
            app.gateway_settings.node_control_allowed_node_ids_input = value
        }
        GatewayMessage::ServiceCommandRequested(command) => {
            let command = command.trim().to_ascii_lowercase();
            if is_danger_service_command(&command) {
                app.gateway_settings.pending_service_command_confirmation = Some(command);
                app.gateway_settings.save_error = None;
                return Task::none();
            }

            app.gateway_settings.service_action_running = Some(command.clone());
            app.gateway_settings.service_action_output = None;
            app.gateway_settings.pending_service_command_confirmation = None;
            app.gateway_settings.save_error = None;
            return service_command_task(command);
        }
        GatewayMessage::ConfirmDangerServiceCommand(command) => {
            let command = command.trim().to_ascii_lowercase();
            if app.gateway_settings.pending_service_command_confirmation.as_deref()
                != Some(command.as_str())
                || !is_danger_service_command(&command)
            {
                return Task::none();
            }

            app.gateway_settings.pending_service_command_confirmation = None;
            app.gateway_settings.service_action_running = Some(command.clone());
            app.gateway_settings.service_action_output = None;
            app.gateway_settings.save_error = None;
            return service_command_task(command);
        }
        GatewayMessage::CancelDangerServiceCommand => {
            app.gateway_settings.pending_service_command_confirmation = None;
            return Task::none();
        }
        GatewayMessage::ServiceCommandCompleted(command, result) => {
            app.gateway_settings.service_action_running = None;
            match result {
                Ok(output) => {
                    let output = if output.trim().is_empty() {
                        format!("service {command} 执行完成")
                    } else {
                        output
                    };
                    app.gateway_settings.service_action_output = Some(output);
                    app.gateway_settings.save_error = None;
                    if should_recover_after_service_command(&command) {
                        return service_recovery_task(command);
                    }
                }
                Err(err) => {
                    app.gateway_settings.service_action_output = None;
                    app.gateway_settings.save_error =
                        Some(format!("service {command} 失败: {err}"));
                }
            }
            return Task::none();
        }
        GatewayMessage::ServiceRecoveryCompleted(command, result) => {
            match result {
                Ok(()) => {
                    let recovery_message = "本机 gateway 已恢复并刷新连接状态。";
                    app.gateway_settings.service_action_output =
                        Some(match app.gateway_settings.service_action_output.take() {
                            Some(output) if !output.trim().is_empty() => {
                                format!("{output}\n{recovery_message}")
                            }
                            _ => recovery_message.to_string(),
                        });
                    app.gateway_settings.save_error = None;
                    return Task::done(Message::GatewayHealthTick);
                }
                Err(err) => {
                    app.gateway_settings.save_error = Some(format!(
                        "service {command} 已执行，但本机 gateway 自动恢复失败: {err}"
                    ));
                }
            }
            return Task::none();
        }
        GatewayMessage::CopyServiceCommand(command) => {
            app.gateway_settings.copied_service_command = Some(command.clone());
            return Task::batch(vec![
                iced::clipboard::write(command.clone()),
                crate::app::message::after(
                    Duration::from_secs(2),
                    Message::Settings(SettingsMessage::Gateway(
                        GatewayMessage::ClearCopiedServiceCommand(command),
                    )),
                ),
            ]);
        }
        GatewayMessage::ClearCopiedServiceCommand(command) => {
            if app.gateway_settings.copied_service_command.as_deref() == Some(command.as_str()) {
                app.gateway_settings.copied_service_command = None;
            }
            return Task::none();
        }
        GatewayMessage::HelpOpen => {
            app.gateway_settings.show_help_modal = true;
            return Task::none();
        }
        GatewayMessage::HelpClose => {
            app.gateway_settings.show_help_modal = false;
            return Task::none();
        }
        GatewayMessage::Refresh => unreachable!(),
    }

    app.gateway_settings.host_input = normalize_host(&app.gateway_settings.host_input);
    app.gateway_settings.save_error = None;
    persist_gateway_settings(app)
}
