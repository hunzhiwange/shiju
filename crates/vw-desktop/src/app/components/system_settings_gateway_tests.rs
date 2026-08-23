//! 验证网关服务操作、配对令牌、密钥日期与日历计算边界。
//! 日期解析同时接受约定格式并拒绝非法输入，闰年和跨年月份单独覆盖。

use super::*;
use crate::app::App;

// 网关测试使用默认应用提供主题和消息上下文，不读取实际服务状态。
fn test_app() -> App {
    App::new().0
}

#[test]
fn system_settings_gateway_tests_are_wired() {
    assert!(module_path!().contains("system_settings_gateway_tests"));
}

#[test]
fn paired_token_list_height_handles_empty_single_and_capped_rows() {
    assert_eq!(paired_token_list_max_height(0), 0.0);
    assert_eq!(paired_token_list_max_height(1), GATEWAY_PAIRED_TOKEN_ROW_HEIGHT);

    let expected = 10.0 * GATEWAY_PAIRED_TOKEN_ROW_HEIGHT + 9.0 * GATEWAY_PAIRED_TOKEN_ROW_SPACING;
    assert_eq!(paired_token_list_max_height(10), expected);
    assert_eq!(paired_token_list_max_height(11), expected);
}

#[test]
fn paired_token_scrollbar_width_is_four_pixels() {
    assert_eq!(GATEWAY_PAIRED_TOKEN_SCROLLBAR_WIDTH, 4);
}

#[test]
fn service_cli_commands_match_copyable_server_commands() {
    assert_eq!(service_cli_command("start"), "vibewindow server start");
    assert_eq!(service_cli_command("restart"), "vibewindow server restart");
    assert_eq!(service_cli_command("status"), "vibewindow server status");
    assert!(is_danger_service_command("stop"));
    assert!(is_danger_service_command("uninstall"));
    assert!(!is_danger_service_command("restart"));
    assert!(!is_danger_service_command("start"));
    assert!(!is_danger_service_command("install"));
    assert!(!is_danger_service_command("status"));
    assert!(!is_danger_service_command("unknown"));
}

#[test]
fn service_command_labels_cover_all_known_commands() {
    assert_eq!(service_command_label("stop"), "停止服务");
    assert_eq!(service_command_label("uninstall"), "卸载服务");
    assert_eq!(service_command_label("install"), "安装服务");
    assert_eq!(service_command_label("start"), "启动服务");
    assert_eq!(service_command_label("restart"), "重启服务");
    assert_eq!(service_command_label("status"), "查询状态");
    assert_eq!(service_command_label("unknown"), "服务命令");
}

#[test]
fn service_command_classification_handles_unknown_and_danger_commands() {
    for command in ["stop", "uninstall"] {
        assert!(is_danger_service_command(command));
    }
    for command in ["", "install", "start", "restart", "status", "STOP", "unknown"] {
        assert!(!is_danger_service_command(command));
    }

    assert_eq!(service_command_label(""), "服务命令");
}

#[test]
fn service_action_elements_cover_idle_running_and_copied_states() {
    let _: iced::Element<'static, Message> = service_action_button("启动服务", "start", None);
    let _: iced::Element<'static, Message> =
        service_action_button("启动服务", "start", Some("start"));
    let _: iced::Element<'static, Message> =
        service_command_copy_button("restart", Some("vibewindow server restart"));
    let _: iced::Element<'static, Message> =
        service_action_cell("卸载服务", "uninstall", Some("status"), None);
}

#[test]
fn field_rows_build_text_secure_bool_hint_and_number_controls() {
    let _: iced::Element<'_, Message> =
        field_row("标签", "说明", iced::widget::text("control").size(12));
    let _: iced::Element<'_, Message> =
        secure_text_row("密钥", "说明", "placeholder", "secret", |value| {
            Message::Settings(SettingsMessage::Gateway(
                GatewayMessage::NodeControlAuthTokenChanged(value),
            ))
        });
    let _: iced::Element<'_, Message> = bool_row("开关", "说明", true, "开启", |value| {
        Message::Settings(SettingsMessage::Gateway(GatewayMessage::AuthEnabledToggled(value)))
    });
    let _: iced::Element<'_, Message> = text_row_with_hint(
        "文本",
        "说明",
        "占位",
        "值",
        "提示文本",
        |_| Message::None,
    );
    let _: iced::Element<'_, Message> = bool_row_with_hint(
        "开关",
        "说明",
        true,
        "开启",
        "提示文本",
        |_| Message::None,
    );
    let _: iced::Element<'_, Message> = number_row("数字", "说明", 7, 1, 10, "秒", |value| {
        Message::Settings(SettingsMessage::Gateway(GatewayMessage::IdempotencyTtlSecsChanged(
            value,
        )))
    });
}

#[test]
fn gateway_tab_labels_returns_config_and_skeys_labels() {
    let labels = gateway_tab_labels();
    assert_eq!(labels.len(), 2);
    assert_eq!(labels[0], ("配置", GatewaySettingsTab::Config));
    assert_eq!(labels[1], ("skey 管理", GatewaySettingsTab::Skeys));
}

#[test]
fn gateway_save_error_banner_follows_tabs() {
    let mut app = test_app();
    assert_eq!(view(&app).as_widget().children().len(), 3);

    app.gateway_settings.save_error = Some("gateway save failed".to_string());
    assert_eq!(view(&app).as_widget().children().len(), 4);
}

#[test]
fn parse_skey_date_parses_iso_date() {
    assert_eq!(parse_skey_date("2026-12-31"), NaiveDate::from_ymd_opt(2026, 12, 31));
    assert_eq!(parse_skey_date("2026-01-01"), NaiveDate::from_ymd_opt(2026, 1, 1));
}

#[test]
fn parse_skey_date_parses_rfc3339() {
    let result = parse_skey_date("2026-12-31T23:59:59Z");
    assert_eq!(result, NaiveDate::from_ymd_opt(2026, 12, 31));

    let result = parse_skey_date("2026-06-15T10:30:00+08:00");
    assert_eq!(result, NaiveDate::from_ymd_opt(2026, 6, 15));
}

#[test]
fn parse_skey_date_handles_empty_and_invalid() {
    assert_eq!(parse_skey_date(""), None);
    assert_eq!(parse_skey_date("  "), None);
    assert_eq!(parse_skey_date("not-a-date"), None);
    assert_eq!(parse_skey_date("2026/12/31"), None);
}

#[test]
fn parse_skey_date_trims_whitespace() {
    assert_eq!(parse_skey_date("  2026-12-31  "), NaiveDate::from_ymd_opt(2026, 12, 31));
    assert_eq!(parse_skey_date("  2026-12-31T00:00:00Z  "), NaiveDate::from_ymd_opt(2026, 12, 31));
}

#[test]
fn month_start_returns_first_day_of_month() {
    let d = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
    assert_eq!(month_start(d), NaiveDate::from_ymd_opt(2026, 6, 1).unwrap());
}

#[test]
fn month_start_preserves_january_and_december() {
    let jan = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
    assert_eq!(month_start(jan), NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());

    let dec = NaiveDate::from_ymd_opt(2026, 12, 25).unwrap();
    assert_eq!(month_start(dec), NaiveDate::from_ymd_opt(2026, 12, 1).unwrap());
}

#[test]
fn parse_calendar_month_parses_yyyy_mm() {
    assert_eq!(
        parse_calendar_month("2026-12"),
        Some(NaiveDate::from_ymd_opt(2026, 12, 1).unwrap())
    );
    assert_eq!(parse_calendar_month("2026-01"), Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()));
}

#[test]
fn parse_calendar_month_handles_invalid() {
    assert_eq!(parse_calendar_month(""), None);
    assert_eq!(parse_calendar_month("2026-13"), None);
    assert_eq!(parse_calendar_month("not-a-month"), None);
    assert_eq!(
        parse_calendar_month(" 2026-02 "),
        Some(NaiveDate::from_ymd_opt(2026, 2, 1).unwrap())
    );
}

#[test]
fn days_in_month_returns_correct_day_count() {
    assert_eq!(days_in_month(2026, 1), 31);
    assert_eq!(days_in_month(2026, 2), 28);
    assert_eq!(days_in_month(2026, 3), 31);
    assert_eq!(days_in_month(2026, 4), 30);
    assert_eq!(days_in_month(2026, 12), 31);
}

#[test]
fn days_in_month_handles_leap_year() {
    assert_eq!(days_in_month(2024, 2), 29);
    assert_eq!(days_in_month(2025, 2), 28);
    assert_eq!(days_in_month(2000, 2), 29);
    assert_eq!(days_in_month(2100, 2), 28);
    assert_eq!(days_in_month(2026, 13), 31);
}

#[test]
fn skey_calendar_month_falls_back_to_today_when_no_input() {
    let state = GatewaySettingsState::default();
    assert_eq!(state.new_skey_calendar_month, "");
    assert_eq!(state.new_skey_expires_at_input, "");
    let app = test_app();
    let result = skey_calendar_month(&app.gateway_settings);
    let today = Utc::now().date_naive();
    assert_eq!(result, month_start(today));
}

#[test]
fn skey_calendar_month_uses_calendar_month_when_set() {
    let mut app = test_app();
    app.gateway_settings.new_skey_calendar_month = "2026-12".to_string();
    assert_eq!(
        skey_calendar_month(&app.gateway_settings),
        NaiveDate::from_ymd_opt(2026, 12, 1).unwrap()
    );
}

#[test]
fn skey_calendar_month_falls_back_to_expires_at_month() {
    let mut app = test_app();
    app.gateway_settings.new_skey_calendar_month = "".to_string();
    app.gateway_settings.new_skey_expires_at_input = "2026-06-15".to_string();
    assert_eq!(
        skey_calendar_month(&app.gateway_settings),
        NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()
    );
}

#[test]
fn view_builds_empty_and_populated_gateway_states() {
    let app = test_app();
    assert_eq!(app.gateway_settings.active_tab, GatewaySettingsTab::Config);
    let _ = view(&app);

    let mut populated = test_app();
    populated.gateway_settings.active_tab = GatewaySettingsTab::Skeys;
    populated.gateway_settings.host_input = "0.0.0.0".to_string();
    populated.gateway_settings.auth_enabled = true;
    populated.gateway_settings.allow_public_bind = true;
    populated.gateway_settings.trust_forwarded_headers = true;
    populated.gateway_settings.skeys = vec![vw_config_types::gateway::GatewaySkey {
        enabled: true,
        skey: None,
        skey_hash: "a".repeat(64),
        masked_skey: "sk-aaaaaaaaaaaaa***************aaaaaaaaa".to_string(),
        name: "abcd***wxyz".to_string(),
        expires_at: None,
    }];
    populated.gateway_settings.new_skey_name_input = "pending-skey".to_string();
    populated.gateway_settings.new_skey_expires_at_input = "2026-12-31".to_string();
    populated.gateway_settings.new_skey_calendar_month = "2026-12".to_string();
    populated.gateway_settings.new_skey_calendar_open = true;
    populated.gateway_settings.last_created_skey = Some("sk-test".to_string());
    populated.gateway_settings.last_created_skey_copied = true;
    populated.gateway_settings.node_control_enabled = true;
    populated.gateway_settings.node_control_auth_token_input = "node-secret".to_string();
    populated.gateway_settings.node_control_allowed_node_ids_input = "node-a\nnode-b".to_string();
    populated.gateway_settings.save_error = Some("gateway save failed".to_string());
    populated.gateway_settings.copied_service_command =
        Some("vibewindow server restart".to_string());
    let _ = view(&populated);
}

#[test]
fn view_handles_skeys_tab_with_empty_skeys() {
    let mut app = test_app();
    app.gateway_settings.active_tab = GatewaySettingsTab::Skeys;
    let _ = view(&app);
}

#[test]
fn view_handles_skey_with_empty_name_and_hash_fallback_display() {
    let mut app = test_app();
    app.gateway_settings.active_tab = GatewaySettingsTab::Skeys;
    app.gateway_settings.skeys = vec![vw_config_types::gateway::GatewaySkey {
        enabled: true,
        skey: None,
        skey_hash: "abcdef1234567890abcdef1234567890".to_string(),
        masked_skey: String::new(),
        name: String::new(),
        expires_at: None,
    }];
    let _ = view(&app);
}

#[test]
fn view_handles_skey_with_expires_at() {
    let mut app = test_app();
    app.gateway_settings.active_tab = GatewaySettingsTab::Skeys;
    app.gateway_settings.skeys = vec![vw_config_types::gateway::GatewaySkey {
        enabled: false,
        skey: None,
        skey_hash: "b".repeat(64),
        masked_skey: "sk-bbbbbbbbbbbbb***************bbbbbbbbb".to_string(),
        name: "test-key".to_string(),
        expires_at: Some("2026-12-31T23:59:59Z".to_string()),
    }];
    let _ = view(&app);
}

#[test]
fn view_handles_multiple_skeys_with_scrollable_list() {
    let mut app = test_app();
    app.gateway_settings.active_tab = GatewaySettingsTab::Skeys;
    app.gateway_settings.skeys = (0..15)
        .map(|i| vw_config_types::gateway::GatewaySkey {
            enabled: i % 2 == 0,
            skey: None,
            skey_hash: format!("skey-hash-{i}"),
            masked_skey: format!("sk-masked-{i}"),
            name: format!("key-{i}"),
            expires_at: None,
        })
        .collect();
    let _ = view(&app);
}

#[test]
fn calendar_popover_and_date_picker_build_for_selected_and_open_states() {
    let mut app = test_app();
    app.gateway_settings.new_skey_calendar_month = "2026-02".to_string();
    app.gateway_settings.new_skey_expires_at_input = "2026-02-14".to_string();
    let _: iced::Element<'_, Message> = skey_calendar_popover(&app);
    let _: iced::Element<'_, Message> = skey_date_picker(&app);

    app.gateway_settings.new_skey_calendar_open = true;
    app.gateway_settings.new_skey_expires_at_input.clear();
    let _: iced::Element<'_, Message> = skey_date_picker(&app);
}

#[test]
fn overlays_return_base_or_help_modal_for_gateway() {
    let app = test_app();
    let base = iced::widget::container(iced::widget::text("base")).into();
    let _ = view_overlays(&app, base);

    let mut with_help = test_app();
    with_help.gateway_settings.show_help_modal = true;
    let base = iced::widget::container(iced::widget::text("base")).into();
    let _ = view_overlays(&with_help, base);

    let mut with_confirm = test_app();
    with_confirm.gateway_settings.pending_service_command_confirmation =
        Some("uninstall".to_string());
    let base = iced::widget::container(iced::widget::text("base")).into();
    let _ = view_overlays(&with_confirm, base);
}

#[test]
fn overlays_handles_stop_confirmation() {
    let mut app = test_app();
    app.gateway_settings.pending_service_command_confirmation = Some("stop".to_string());
    let base = iced::widget::container(iced::widget::text("base")).into();
    let _ = view_overlays(&app, base);

    app.gateway_settings.pending_service_command_confirmation = Some("unknown".to_string());
    let base = iced::widget::container(iced::widget::text("base")).into();
    let _ = view_overlays(&app, base);
}

#[test]
fn view_with_non_empty_service_action_output() {
    let mut app = test_app();
    app.gateway_settings.service_action_output = Some("  service is running  ".to_string());
    let _ = view(&app);
}

#[test]
fn view_with_blank_service_action_output_is_skipped() {
    let mut app = test_app();
    app.gateway_settings.service_action_output = Some("   ".to_string());
    let _ = view(&app);
}
