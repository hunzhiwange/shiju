use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

#[test]
fn gateway_helpers_normalize_and_clamp() {
    assert_eq!(normalize_host(""), "127.0.0.1");
    assert_eq!(normalize_host(" 0.0.0.0 "), "0.0.0.0");
    assert_eq!(clamp_token_limit(0), 1);
    assert_eq!(clamp_token_limit(200_000), 100_000);
    assert_eq!(
        mask_skey_for_display("sk-4234324234324abcdefghijklmnop111111111"),
        "sk-4234324234324***************111111111"
    );
}

#[test]
fn gateway_update_generates_skeys_and_handles_help() {
    let mut app = app();
    app.gateway_settings.skeys.clear();
    app.gateway_settings.save_error = Some("old".to_string());
    app.gateway_settings.new_skey_calendar_open = true;

    assert_eq!(app.gateway_settings.active_tab, crate::app::state::GatewaySettingsTab::Config);
    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::TabSelected(
            crate::app::state::GatewaySettingsTab::Skeys,
        )),
    );
    assert_eq!(app.gateway_settings.active_tab, crate::app::state::GatewaySettingsTab::Skeys);
    assert!(!app.gateway_settings.new_skey_calendar_open);

    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::Refresh));
    assert!(app.gateway_settings.save_error.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::NewSkeyNameChanged(" laptop ".to_string())),
    );
    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::NewSkeyCalendarToggled));
    assert!(app.gateway_settings.new_skey_calendar_open);
    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::NewSkeyCalendarClosed));
    assert!(!app.gateway_settings.new_skey_calendar_open);
    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::NewSkeyCalendarToggled));
    assert!(app.gateway_settings.new_skey_calendar_open);
    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::NewSkeyExpiresDateSelected(
            "2026-12-31".to_string(),
        )),
    );
    assert!(!app.gateway_settings.new_skey_calendar_open);
    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::AddSkey));
    assert_eq!(app.gateway_settings.skeys.len(), 1);
    assert!(app.gateway_settings.skeys[0].enabled);
    assert_eq!(app.gateway_settings.skeys[0].name, "laptop");
    assert_eq!(app.gateway_settings.skeys[0].expires_at.as_deref(), Some("2026-12-31T23:59:59Z"));
    assert!(app.gateway_settings.skeys[0].skey.is_none());
    assert!(app.gateway_settings.new_skey_name_input.is_empty());
    let generated = app.gateway_settings.last_created_skey.clone().unwrap();
    assert!(generated.starts_with("sk-"));
    assert_eq!(app.gateway_settings.skeys[0].skey_hash, hash_skey(&generated));
    assert_eq!(app.gateway_settings.skeys[0].masked_skey, mask_skey_for_display(&generated));

    let _ =
        update(&mut app, SettingsMessage::Gateway(GatewayMessage::SkeyEnabledToggled(0, false)));
    assert!(!app.gateway_settings.skeys[0].enabled);

    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::CopyLastCreatedSkey));
    assert!(app.gateway_settings.last_created_skey_copied);
    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::ClearLastCreatedSkeyCopied));
    assert!(!app.gateway_settings.last_created_skey_copied);

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::NewSkeyNameChanged(" ".to_string())),
    );
    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::AddSkey));
    assert!(app.gateway_settings.save_error.as_deref().unwrap_or("").contains("不能为空"));

    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::HostChanged("".to_string())));
    let _ =
        update(&mut app, SettingsMessage::Gateway(GatewayMessage::IdempotencyTtlSecsChanged(0)));
    assert_eq!(app.gateway_settings.host_input, "127.0.0.1");
    assert_eq!(app.gateway_settings.idempotency_ttl_secs, 1);

    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::HelpOpen));
    assert!(app.gateway_settings.show_help_modal);
    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::HelpClose));
    assert!(!app.gateway_settings.show_help_modal);
}

#[test]
fn gateway_service_command_completion_updates_status_fields() {
    let mut app = app();
    app.gateway_settings.service_action_running = Some("status".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::ServiceCommandCompleted(
            "status".to_string(),
            Ok("Service: running".to_string()),
        )),
    );
    assert!(app.gateway_settings.service_action_running.is_none());
    assert_eq!(app.gateway_settings.service_action_output.as_deref(), Some("Service: running"));
    assert!(app.gateway_settings.save_error.is_none());

    app.gateway_settings.service_action_running = Some("start".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::ServiceCommandCompleted(
            "start".to_string(),
            Err("not installed".to_string()),
        )),
    );
    assert!(app.gateway_settings.service_action_running.is_none());
    assert!(app.gateway_settings.service_action_output.is_none());
    assert!(
        app.gateway_settings.save_error.as_deref().unwrap_or_default().contains("not installed")
    );
}

#[test]
fn gateway_service_command_copy_state_clears_after_matching_command() {
    let mut app = app();

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::CopyServiceCommand(
            "vibewindow server restart".to_string(),
        )),
    );
    assert_eq!(
        app.gateway_settings.copied_service_command.as_deref(),
        Some("vibewindow server restart")
    );

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::ClearCopiedServiceCommand(
            "vibewindow server start".to_string(),
        )),
    );
    assert_eq!(
        app.gateway_settings.copied_service_command.as_deref(),
        Some("vibewindow server restart")
    );

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::ClearCopiedServiceCommand(
            "vibewindow server restart".to_string(),
        )),
    );
    assert!(app.gateway_settings.copied_service_command.is_none());
}

#[test]
fn gateway_danger_service_commands_require_confirmation() {
    let mut app = app();

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::ServiceCommandRequested("stop".to_string())),
    );
    assert_eq!(app.gateway_settings.pending_service_command_confirmation.as_deref(), Some("stop"));
    assert!(app.gateway_settings.service_action_running.is_none());

    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::CancelDangerServiceCommand));
    assert!(app.gateway_settings.pending_service_command_confirmation.is_none());

    app.gateway_settings.pending_service_command_confirmation = Some("uninstall".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::ConfirmDangerServiceCommand("stop".to_string())),
    );
    assert_eq!(
        app.gateway_settings.pending_service_command_confirmation.as_deref(),
        Some("uninstall")
    );
    assert!(app.gateway_settings.service_action_running.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::ConfirmDangerServiceCommand(
            "uninstall".to_string(),
        )),
    );
    assert!(app.gateway_settings.pending_service_command_confirmation.is_none());
    assert_eq!(app.gateway_settings.service_action_running.as_deref(), Some("uninstall"));
}

#[test]
fn gateway_skey_helpers_validate_unicode_names_dates_and_mask_boundaries() {
    let max_name = "界".repeat(80);
    assert_eq!(normalize_skey_name(&format!("  {max_name}  ")), Ok(max_name));
    assert_eq!(
        normalize_skey_name(&"界".repeat(81)),
        Err("skey 名称不能超过 80 个字符".to_string())
    );
    assert_eq!(normalize_skey_name(" \n "), Err("skey 名称不能为空".to_string()));

    assert_eq!(mask_skey_for_display(" 1234567890123456789012345 "), "1234567890123456789012345");
    assert_eq!(hash_skey(" secret "), hash_skey("secret"));
    assert_ne!(hash_skey("secret"), hash_skey("different"));

    assert_eq!(normalize_expires_at(" \n "), Ok(None));
    assert_eq!(normalize_expires_at(" 2026-12-31 "), Ok(Some("2026-12-31T23:59:59Z".to_string())));
    assert_eq!(
        normalize_expires_at("2026-12-31T08:30:00+08:00"),
        Ok(Some("2026-12-31T08:30:00+08:00".to_string()))
    );
    assert!(normalize_expires_at("2026-02-30").unwrap_err().contains("2026-12-31"));
}

#[test]
fn gateway_calendar_helpers_parse_rfc3339_and_shift_across_years() {
    assert_eq!(
        parse_date_input("2026-12-31T23:30:00+08:00"),
        NaiveDate::from_ymd_opt(2026, 12, 31)
    );
    assert_eq!(parse_date_input("bad date"), None);
    assert_eq!(parse_calendar_month("2026-12"), NaiveDate::from_ymd_opt(2026, 12, 1));
    assert_eq!(parse_calendar_month("2026-13"), None);

    let december = NaiveDate::from_ymd_opt(2026, 12, 1).unwrap();
    assert_eq!(shift_calendar_month(december, 1), NaiveDate::from_ymd_opt(2027, 1, 1).unwrap());
    assert_eq!(shift_calendar_month(december, -12), NaiveDate::from_ymd_opt(2025, 12, 1).unwrap());
}

#[test]
fn current_calendar_month_falls_back_to_expiration_before_today() {
    let mut app = app();
    app.gateway_settings.new_skey_calendar_month = "invalid".to_string();
    app.gateway_settings.new_skey_expires_at_input = "2031-07-19T08:30:00+08:00".to_string();

    assert_eq!(current_calendar_month(&app), NaiveDate::from_ymd_opt(2031, 7, 1).unwrap());
}

#[test]
fn gateway_expiration_messages_keep_calendar_and_input_consistent() {
    let mut app = app();
    app.gateway_settings.new_skey_calendar_month = "2026-12".to_string();
    app.gateway_settings.new_skey_calendar_open = true;

    let _ =
        update(&mut app, SettingsMessage::Gateway(GatewayMessage::NewSkeyExpiresMonthChanged(1)));
    assert_eq!(app.gateway_settings.new_skey_calendar_month, "2027-01");

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::NewSkeyExpiresAtChanged(
            "2028-03-14T10:00:00Z".to_string(),
        )),
    );
    assert_eq!(app.gateway_settings.new_skey_calendar_month, "2028-03");
    assert_eq!(app.gateway_settings.new_skey_expires_at_input, "2028-03-14T10:00:00Z");

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::NewSkeyExpiresAtChanged("not-a-date".to_string())),
    );
    assert_eq!(app.gateway_settings.new_skey_calendar_month, "2028-03");
    assert_eq!(app.gateway_settings.new_skey_expires_at_input, "not-a-date");

    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::NewSkeyExpiresAtCleared));
    assert!(app.gateway_settings.new_skey_expires_at_input.is_empty());
    assert!(!app.gateway_settings.new_skey_calendar_open);
}

#[test]
fn gateway_config_messages_clamp_values_and_preserve_ignored_pair_limit() {
    let mut app = app();
    app.gateway_settings.save_error = Some("stale".to_string());

    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::PortChanged(0)));
    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::WebhookRateLimitPerMinuteChanged(u32::MAX)),
    );
    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::RateLimitMaxKeysChanged(0)));
    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::IdempotencyTtlSecsChanged(u32::MAX)),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::IdempotencyMaxKeysChanged(u32::MAX)),
    );
    let previous_webhook_limit = app.gateway_settings.webhook_rate_limit_per_minute;
    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::PairRateLimitPerMinuteChanged(7)),
    );

    assert_eq!(app.gateway_settings.port, 1);
    assert_eq!(app.gateway_settings.webhook_rate_limit_per_minute, previous_webhook_limit);
    assert_eq!(app.gateway_settings.rate_limit_max_keys, 1);
    assert_eq!(app.gateway_settings.idempotency_ttl_secs, 86_400);
    assert_eq!(app.gateway_settings.idempotency_max_keys, 100_000);
    assert!(app.gateway_settings.save_error.is_none());

    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::RequirePairingToggled(true)));
    let _ =
        update(&mut app, SettingsMessage::Gateway(GatewayMessage::AllowPublicBindToggled(true)));
    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::TrustForwardedHeadersToggled(true)),
    );
    let _ =
        update(&mut app, SettingsMessage::Gateway(GatewayMessage::NodeControlEnabledToggled(true)));
    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::NodeControlAuthTokenChanged(
            " token ".to_string(),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::NodeControlAllowedNodeIdsChanged(
            "node-a, node-b".to_string(),
        )),
    );

    assert!(app.gateway_settings.auth_enabled);
    assert!(app.gateway_settings.allow_public_bind);
    assert!(app.gateway_settings.trust_forwarded_headers);
    assert!(app.gateway_settings.node_control_enabled);
    assert_eq!(app.gateway_settings.node_control_auth_token_input, " token ");
    assert_eq!(app.gateway_settings.node_control_allowed_node_ids_input, "node-a, node-b");
}

#[test]
fn gateway_alias_and_index_messages_handle_missing_and_existing_skeys() {
    let mut app = app();
    app.gateway_settings.skeys.clear();

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::NewPairedTokenChanged(" legacy ".to_string())),
    );
    assert_eq!(app.gateway_settings.new_skey_name_input, " legacy ");

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::NewSkeyChanged(" alias ".to_string())),
    );
    assert_eq!(app.gateway_settings.new_skey_name_input, " alias ");

    let _ =
        update(&mut app, SettingsMessage::Gateway(GatewayMessage::SkeyEnabledToggled(4, false)));
    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::RemovePairedToken(4)));
    assert!(app.gateway_settings.skeys.is_empty());

    app.gateway_settings.new_skey_name_input = "legacy token".to_string();
    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::AddPairedToken));
    assert_eq!(app.gateway_settings.skeys.len(), 1);
    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::RemoveSkey(0)));
    assert!(app.gateway_settings.skeys.is_empty());
}

#[test]
fn gateway_service_messages_normalize_commands_and_report_recovery_results() {
    let mut app = app();

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::ServiceCommandRequested(" ReStArT ".to_string())),
    );
    assert_eq!(app.gateway_settings.service_action_running.as_deref(), Some("restart"));
    assert!(app.gateway_settings.pending_service_command_confirmation.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::ServiceCommandCompleted(
            "status".to_string(),
            Ok(" \n ".to_string()),
        )),
    );
    assert_eq!(
        app.gateway_settings.service_action_output.as_deref(),
        Some("service status 执行完成")
    );

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::ServiceCommandCompleted(
            "start".to_string(),
            Ok(String::new()),
        )),
    );
    assert_eq!(
        app.gateway_settings.service_action_output.as_deref(),
        Some("service start 执行完成")
    );

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::ServiceRecoveryCompleted(
            "restart".to_string(),
            Ok(()),
        )),
    );
    assert_eq!(
        app.gateway_settings.service_action_output.as_deref(),
        Some("service start 执行完成\n本机 gateway 已恢复并刷新连接状态。")
    );
    assert!(app.gateway_settings.save_error.is_none());

    app.gateway_settings.service_action_output = None;
    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::ServiceRecoveryCompleted(
            "start".to_string(),
            Ok(()),
        )),
    );
    assert_eq!(
        app.gateway_settings.service_action_output.as_deref(),
        Some("本机 gateway 已恢复并刷新连接状态。")
    );

    let _ = update(
        &mut app,
        SettingsMessage::Gateway(GatewayMessage::ServiceRecoveryCompleted(
            "start".to_string(),
            Err("health check timeout".to_string()),
        )),
    );
    assert!(
        app.gateway_settings
            .save_error
            .as_deref()
            .unwrap_or_default()
            .contains("service start 已执行，但本机 gateway 自动恢复失败: health check timeout")
    );
}

#[test]
fn gateway_update_ignores_non_gateway_messages_and_copy_without_a_skey() {
    let mut app = app();
    app.gateway_settings.save_error = Some("keep".to_string());

    let _ = update(&mut app, SettingsMessage::ProvidersRefresh);
    assert_eq!(app.gateway_settings.save_error.as_deref(), Some("keep"));

    app.gateway_settings.last_created_skey = None;
    app.gateway_settings.last_created_skey_copied = false;
    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::CopyLastCreatedSkey));
    assert!(!app.gateway_settings.last_created_skey_copied);

    app.gateway_settings.new_skey_name_input = "invalid expiration".to_string();
    app.gateway_settings.new_skey_expires_at_input = "2026-02-30".to_string();
    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::AddSkey));
    assert!(app.gateway_settings.skeys.is_empty());
    assert!(app.gateway_settings.save_error.as_deref().unwrap_or_default().contains("过期日期"));
}
