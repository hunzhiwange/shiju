//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。
use desktop_i18n::settings_models::messages::Feedback;
use crate::Message as SettingsMessage;
use crate::state::GatewaySettingsState;
use super::parse_comma_or_newline_list;

use chrono::{Datelike, NaiveDate, Utc};

use sha2::{Digest, Sha256};

use config_gateway::GatewaySkey;

use crate::messages::{GatewayMessage};

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

fn normalize_skey_name(raw: &str) -> Result<String, Feedback> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(Feedback::owned("skey 名称不能为空", vec![]));
    }
    if trimmed.chars().count() > 80 {
        return Err(Feedback::owned("skey 名称不能超过 80 个字符", vec![]));
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

fn current_calendar_month(app: &GatewaySettingsState) -> NaiveDate {
    parse_calendar_month(&app.new_skey_calendar_month)
        .or_else(|| {
            parse_date_input(&app.new_skey_expires_at_input).map(month_start)
        })
        .unwrap_or_else(|| month_start(Utc::now().date_naive()))
}

fn shift_calendar_month(month: NaiveDate, delta: i32) -> NaiveDate {
    let month_index = month.year() * 12 + month.month0() as i32 + delta;
    let year = month_index.div_euclid(12);
    let month = month_index.rem_euclid(12) as u32 + 1;
    NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(month_start(Utc::now().date_naive()))
}

fn set_calendar_month(app: &mut GatewaySettingsState, month: NaiveDate) {
    app.new_skey_calendar_month = month.format("%Y-%m").to_string();
}

fn normalize_expires_at(raw: &str) -> Result<Option<String>, Feedback> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if chrono::DateTime::parse_from_rfc3339(trimmed).is_ok() {
        return Ok(Some(trimmed.to_string()));
    }
    let date = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .map_err(|_| Feedback::owned("过期日期必须从日历选择，或使用 2026-12-31 格式", vec![]))?;
    Ok(Some(format!("{}T23:59:59Z", date.format("%Y-%m-%d"))))
}

fn add_skey(app: &mut GatewaySettingsState) -> Effect {
    let name = match normalize_skey_name(&app.new_skey_name_input) {
        Ok(value) => value,
        Err(err) => {
            app.save_error = Some((err).into());
            return Effect::None;
        }
    };

    let expires_at = match normalize_expires_at(&app.new_skey_expires_at_input) {
        Ok(value) => value,
        Err(err) => {
            app.save_error = Some((err).into());
            return Effect::None;
        }
    };

    let (skey, skey_hash) = loop {
        let candidate = generate_skey();
        let candidate_hash = hash_skey(&candidate);
        if !app.skeys.iter().any(|existing| existing.skey_hash == candidate_hash) {
            break (candidate, candidate_hash);
        }
    };

    app.skeys.push(GatewaySkey {
        enabled: true,
        skey: None,
        skey_hash,
        masked_skey: mask_skey_for_display(&skey),
        name,
        expires_at,
    });
    app.new_skey_name_input.clear();
    app.new_skey_expires_at_input.clear();
    app.new_skey_calendar_open = false;
    app.last_created_skey = Some(skey);
    app.last_created_skey_copied = false;
    app.save_error = None;
    persist_gateway_settings(app)
}

fn persist_gateway_settings(app: &mut GatewaySettingsState) -> Effect {
    let s = &app;
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

    {
        let mut gateway = config_gateway::GatewayConfig::default();
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

        Effect::Persist(gateway)
    }
}

fn should_recover_after_service_command(command: &str) -> bool {
    matches!(command, "start" | "restart")
}

fn is_danger_service_command(command: &str) -> bool {
    matches!(command, "stop" | "uninstall")
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回领域效果，由宿主执行持久化或平台动作。
pub fn update(app: &mut GatewaySettingsState, message: SettingsMessage) -> Effect {
    let SettingsMessage::Gateway(message) = message else {
        return Effect::None;
    };

    if matches!(message, GatewayMessage::Refresh) {
        app.save_error = None;
        return Effect::None;
    }

    match message {
        GatewayMessage::TabSelected(tab) => {
            app.active_tab = tab;
            app.new_skey_calendar_open = false;
            return Effect::None;
        }
        GatewayMessage::PortChanged(value) => app.port = value.clamp(1, u16::MAX),
        GatewayMessage::HostChanged(value) => app.host_input = value,
        GatewayMessage::AuthEnabledToggled(value) => app.auth_enabled = value,
        GatewayMessage::NewSkeyNameChanged(value) | GatewayMessage::NewSkeyChanged(value) => {
            app.new_skey_name_input = value;
            return Effect::None;
        }
        GatewayMessage::NewSkeyExpiresAtChanged(value) => {
            if let Some(date) = parse_date_input(&value) {
                set_calendar_month(app, month_start(date));
            }
            app.new_skey_expires_at_input = value;
            return Effect::None;
        }
        GatewayMessage::NewSkeyCalendarToggled => {
            app.new_skey_calendar_open =
                !app.new_skey_calendar_open;
            return Effect::None;
        }
        GatewayMessage::NewSkeyCalendarClosed => {
            app.new_skey_calendar_open = false;
            return Effect::None;
        }
        GatewayMessage::NewSkeyExpiresDateSelected(value) => {
            if let Some(date) = parse_date_input(&value) {
                set_calendar_month(app, month_start(date));
            }
            app.new_skey_expires_at_input = value;
            app.new_skey_calendar_open = false;
            return Effect::None;
        }
        GatewayMessage::NewSkeyExpiresMonthChanged(delta) => {
            let month = shift_calendar_month(current_calendar_month(app), delta);
            set_calendar_month(app, month);
            return Effect::None;
        }
        GatewayMessage::NewSkeyExpiresAtCleared => {
            app.new_skey_expires_at_input.clear();
            app.new_skey_calendar_open = false;
            return Effect::None;
        }
        GatewayMessage::AddSkey => return add_skey(app),
        GatewayMessage::CopyLastCreatedSkey => {
            let Some(skey) = app.last_created_skey.clone() else {
                return Effect::None;
            };
            app.last_created_skey_copied = true;
            return Effect::Copy(skey, GatewayMessage::ClearLastCreatedSkeyCopied);
        }
        GatewayMessage::ClearLastCreatedSkeyCopied => {
            app.last_created_skey_copied = false;
            return Effect::None;
        }
        GatewayMessage::SkeyEnabledToggled(index, enabled) => {
            if let Some(skey) = app.skeys.get_mut(index) {
                skey.enabled = enabled;
            }
        }
        GatewayMessage::RemoveSkey(index) => {
            if index < app.skeys.len() {
                app.skeys.remove(index);
            }
        }
        GatewayMessage::RequirePairingToggled(value) => {
            app.auth_enabled = value;
        }
        GatewayMessage::AllowPublicBindToggled(value) => {
            app.allow_public_bind = value
        }
        GatewayMessage::TrustForwardedHeadersToggled(value) => {
            app.trust_forwarded_headers = value
        }
        GatewayMessage::NewPairedTokenChanged(value) => {
            app.new_skey_name_input = value;
            return Effect::None;
        }
        GatewayMessage::AddPairedToken => return add_skey(app),
        GatewayMessage::RemovePairedToken(index) => {
            if index < app.skeys.len() {
                app.skeys.remove(index);
            }
        }
        GatewayMessage::PairRateLimitPerMinuteChanged(_) => {}
        GatewayMessage::WebhookRateLimitPerMinuteChanged(value) => {
            app.webhook_rate_limit_per_minute = value.clamp(1, 100_000)
        }
        GatewayMessage::RateLimitMaxKeysChanged(value) => {
            app.rate_limit_max_keys = clamp_token_limit(value)
        }
        GatewayMessage::IdempotencyTtlSecsChanged(value) => {
            app.idempotency_ttl_secs = value.clamp(1, 86_400)
        }
        GatewayMessage::IdempotencyMaxKeysChanged(value) => {
            app.idempotency_max_keys = clamp_token_limit(value)
        }
        GatewayMessage::NodeControlEnabledToggled(value) => {
            app.node_control_enabled = value
        }
        GatewayMessage::NodeControlAuthTokenChanged(value) => {
            app.node_control_auth_token_input = value
        }
        GatewayMessage::NodeControlAllowedNodeIdsChanged(value) => {
            app.node_control_allowed_node_ids_input = value
        }
        GatewayMessage::ServiceCommandRequested(command) => {
            let command = command.trim().to_ascii_lowercase();
            if is_danger_service_command(&command) {
                app.pending_service_command_confirmation = Some(command);
                app.save_error = None;
                return Effect::None;
            }

            app.service_action_running = Some(command.clone());
            app.service_action_output = None;
            app.pending_service_command_confirmation = None;
            app.save_error = None;
            return Effect::ServiceCommand(command);
        }
        GatewayMessage::ConfirmDangerServiceCommand(command) => {
            let command = command.trim().to_ascii_lowercase();
            if app.pending_service_command_confirmation.as_deref()
                != Some(command.as_str())
                || !is_danger_service_command(&command)
            {
                return Effect::None;
            }

            app.pending_service_command_confirmation = None;
            app.service_action_running = Some(command.clone());
            app.service_action_output = None;
            app.save_error = None;
            return Effect::ServiceCommand(command);
        }
        GatewayMessage::CancelDangerServiceCommand => {
            app.pending_service_command_confirmation = None;
            return Effect::None;
        }
        GatewayMessage::ServiceCommandCompleted(command, result) => {
            app.service_action_running = None;
            match result {
                Ok(output) => {
                    let output = if output.trim().is_empty() {
                        Feedback::owned("service %s 执行完成", vec![command.clone()])
                    } else {
                        output.into()
                    };
                    app.service_action_output = Some(output);
                    app.save_error = None;
                    if should_recover_after_service_command(&command) {
                        return Effect::ServiceRecovery(command);
                    }
                }
                Err(err) => {
                    app.service_action_output = None;
                    app.save_error =
                        Some(Feedback::owned("service %s 失败: %s", vec![command.clone(), err]));
                }
            }
            return Effect::None;
        }
        GatewayMessage::ServiceRecoveryCompleted(command, result) => {
            match result {
                Ok(()) => {
                    let recovery_message =
                        Feedback::owned("本机 gateway 已恢复并刷新连接状态。", vec![]);
                    app.service_action_output =
                        Some(match app.service_action_output.take() {
                            Some(output) if !output.trim().is_empty() => {
                                output.join(recovery_message)
                            }
                            _ => recovery_message,
                        });
                    app.save_error = None;
                    return Effect::RefreshHealth;
                }
                Err(err) => {
                    app.save_error = Some(Feedback::owned(
                        "service %s 已执行，但本机 gateway 自动恢复失败: %s",
                        vec![command.clone(), err],
                    ));
                }
            }
            return Effect::None;
        }
        GatewayMessage::CopyServiceCommand(command) => {
            app.copied_service_command = Some(command.clone());
            return Effect::Copy(command.clone(), GatewayMessage::ClearCopiedServiceCommand(command));
        }
        GatewayMessage::ClearCopiedServiceCommand(command) => {
            if app.copied_service_command.as_deref() == Some(command.as_str()) {
                app.copied_service_command = None;
            }
            return Effect::None;
        }
        GatewayMessage::HelpOpen => {
            app.show_help_modal = true;
            return Effect::None;
        }
        GatewayMessage::HelpClose => {
            app.show_help_modal = false;
            return Effect::None;
        }
        GatewayMessage::Refresh => unreachable!(),
    }

    app.host_input = normalize_host(&app.host_input);
    app.save_error = None;
    persist_gateway_settings(app)
}

pub enum Effect { None, Persist(config_gateway::GatewayConfig), ServiceCommand(String), ServiceRecovery(String), Copy(String, GatewayMessage), RefreshHealth }

/// Apply only fields owned by this settings page.
pub fn apply_config(target: &mut config_gateway::GatewayConfig, value: config_gateway::GatewayConfig) {
        target.port = value.port;
        target.host = value.host;
        target.auth_enabled = value.auth_enabled;
        target.allow_public_bind = value.allow_public_bind;
        target.skeys = value.skeys;
        target.require_pairing = value.require_pairing;
        target.paired_tokens = value.paired_tokens;
        target.webhook_rate_limit_per_minute = value.webhook_rate_limit_per_minute;
        target.trust_forwarded_headers = value.trust_forwarded_headers;
        target.rate_limit_max_keys = value.rate_limit_max_keys;
        target.idempotency_ttl_secs = value.idempotency_ttl_secs;
        target.idempotency_max_keys = value.idempotency_max_keys;
        target.node_control.enabled = value.node_control.enabled;
        target.node_control.auth_token = value.node_control.auth_token;
        target.node_control.allowed_node_ids = value.node_control.allowed_node_ids;
}
