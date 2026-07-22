use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

#[test]
fn parse_csv_lines_trims_mixed_separators_and_discards_empty_values() {
    assert_eq!(
        parse_csv_lines(" example.com, ,api.example.com\n\n*.example.org, "),
        vec!["example.com", "api.example.com", "*.example.org"]
    );
    assert!(parse_csv_lines(" , \n  ,").is_empty());
}

#[test]
fn normalize_browser_open_accepts_supported_aliases_and_defaults_unknown_values() {
    for input in ["new_window", " NEW-WINDOW "] {
        assert_eq!(normalize_browser_open(input), "new_window", "input: {input}");
    }
    for input in ["new_tab", " New-Tab "] {
        assert_eq!(normalize_browser_open(input), "new_tab", "input: {input}");
    }
    for input in ["", "same_tab", "???"] {
        assert_eq!(normalize_browser_open(input), "default", "input: {input}");
    }
}

#[test]
fn normalize_backend_accepts_supported_aliases_and_defaults_unknown_values() {
    for input in ["native", " RUST-NATIVE "] {
        assert_eq!(normalize_backend(input), "native", "input: {input}");
    }
    assert_eq!(normalize_backend(" Computer-Use "), "computer_use");
    assert_eq!(normalize_backend(" AUTO "), "auto");
    for input in ["", "agent_browser", "unexpected"] {
        assert_eq!(normalize_backend(input), "agent_browser", "input: {input}");
    }
}

#[test]
fn parse_timeout_ms_applies_default_and_accepts_positive_boundaries() {
    assert_eq!(parse_timeout_ms("").unwrap(), 15_000);
    assert_eq!(parse_timeout_ms("   ").unwrap(), 15_000);
    assert_eq!(parse_timeout_ms(" 1 ").unwrap(), 1);
    assert_eq!(parse_timeout_ms(&u64::MAX.to_string()).unwrap(), u64::MAX);
}

#[test]
fn parse_timeout_ms_rejects_zero_non_numbers_and_overflow() {
    assert_eq!(parse_timeout_ms("0").unwrap_err(), "computer_use.timeout_ms 必须大于 0");
    for input in ["-1", "1.5", "not-a-number", "18446744073709551616"] {
        assert_eq!(
            parse_timeout_ms(input).unwrap_err(),
            "computer_use.timeout_ms 必须是正整数",
            "input: {input}"
        );
    }
}

#[test]
fn parse_optional_i64_handles_blank_signed_boundaries_and_field_errors() {
    assert_eq!(parse_optional_i64("  ", "coordinate").unwrap(), None);
    assert_eq!(parse_optional_i64(" -42 ", "coordinate").unwrap(), Some(-42));
    assert_eq!(parse_optional_i64(&i64::MIN.to_string(), "coordinate").unwrap(), Some(i64::MIN));
    assert_eq!(parse_optional_i64(&i64::MAX.to_string(), "coordinate").unwrap(), Some(i64::MAX));
    assert_eq!(
        parse_optional_i64("nope", "computer_use.max_coordinate_x").unwrap_err(),
        "computer_use.max_coordinate_x 必须是整数"
    );
}

#[test]
fn update_ignores_unrelated_settings_messages() {
    let mut app = app();
    app.browser_settings.enabled = true;
    app.browser_settings.backend = "native".to_string();
    app.browser_settings.save_error = Some("keep".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(app.browser_settings.enabled);
    assert_eq!(app.browser_settings.backend, "native");
    assert_eq!(app.browser_settings.save_error.as_deref(), Some("keep"));
}

#[test]
fn refresh_only_clears_the_browser_save_error() {
    let mut app = app();
    app.browser_settings.allowed_domains_input = "example.com".to_string();
    app.browser_settings.backend = "computer_use".to_string();
    app.browser_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::Browser(BrowserMessage::Refresh));

    assert_eq!(app.browser_settings.allowed_domains_input, "example.com");
    assert_eq!(app.browser_settings.backend, "computer_use");
    assert!(app.browser_settings.save_error.is_none());
}

#[test]
fn allowed_domains_change_keeps_editor_and_input_in_sync() {
    let mut app = app();
    let value = " example.com,\napi.example.com ";

    let _ = update(
        &mut app,
        SettingsMessage::Browser(BrowserMessage::AllowedDomainsChanged(value.to_string())),
    );

    assert_eq!(app.browser_settings.allowed_domains_input, value);
    assert_eq!(app.browser_settings.allowed_domains_editor.text(), value);
    assert!(app.browser_settings.save_error.is_none());
}

#[test]
fn allowed_domains_editor_persists_edits_but_not_selection_actions() {
    let mut app = app();
    app.browser_settings.allowed_domains_editor = text_editor::Content::with_text("old.example");
    app.browser_settings.allowed_domains_input = "old.example".to_string();
    app.browser_settings.save_error = Some("keep".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Browser(BrowserMessage::AllowedDomainsEditorAction(
            text_editor::Action::SelectAll,
        )),
    );

    assert_eq!(app.browser_settings.allowed_domains_input, "old.example");
    assert_eq!(app.browser_settings.save_error.as_deref(), Some("keep"));

    let _ = update(
        &mut app,
        SettingsMessage::Browser(BrowserMessage::AllowedDomainsEditorAction(
            text_editor::Action::Edit(text_editor::Edit::Insert('x')),
        )),
    );

    assert_eq!(app.browser_settings.allowed_domains_editor.text(), "x");
    assert_eq!(app.browser_settings.allowed_domains_input, "x");
    assert!(app.browser_settings.save_error.is_none());
}

#[test]
fn update_normalizes_browser_open_and_backend_values() {
    let mut app = app();

    let _ = update(
        &mut app,
        SettingsMessage::Browser(BrowserMessage::BrowserOpenChanged(" New-Window ".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Browser(BrowserMessage::BackendChanged(" RUST-NATIVE ".to_string())),
    );

    assert_eq!(app.browser_settings.browser_open, "new_window");
    assert_eq!(app.browser_settings.backend, "native");
    assert!(app.browser_settings.save_error.is_none());
}

#[test]
fn update_applies_all_non_numeric_browser_fields() {
    let mut app = app();

    let messages = [
        BrowserMessage::EnabledToggled(true),
        BrowserMessage::SessionNameChanged(" session ".to_string()),
        BrowserMessage::NativeHeadlessToggled(false),
        BrowserMessage::NativeWebdriverUrlChanged(" http://webdriver.example ".to_string()),
        BrowserMessage::NativeChromePathChanged(" /opt/chrome ".to_string()),
        BrowserMessage::ComputerUseEndpointChanged(" http://computer-use.example ".to_string()),
        BrowserMessage::ComputerUseApiKeyChanged(" secret-placeholder ".to_string()),
        BrowserMessage::ComputerUseAllowRemoteEndpointToggled(true),
        BrowserMessage::ComputerUseWindowAllowlistChanged(" Terminal, Browser ".to_string()),
    ];
    for message in messages {
        let _ = update(&mut app, SettingsMessage::Browser(message));
    }

    assert!(app.browser_settings.enabled);
    assert_eq!(app.browser_settings.session_name_input, " session ");
    assert!(!app.browser_settings.native_headless);
    assert_eq!(app.browser_settings.native_webdriver_url, " http://webdriver.example ");
    assert_eq!(app.browser_settings.native_chrome_path_input, " /opt/chrome ");
    assert_eq!(app.browser_settings.computer_use_endpoint, " http://computer-use.example ");
    assert_eq!(app.browser_settings.computer_use_api_key_input, " secret-placeholder ");
    assert!(app.browser_settings.computer_use_allow_remote_endpoint);
    assert_eq!(app.browser_settings.computer_use_window_allowlist_input, " Terminal, Browser ");
    assert!(app.browser_settings.save_error.is_none());
}

#[test]
fn update_accepts_valid_timeout_and_signed_coordinate_inputs() {
    let mut app = app();

    let _ = update(
        &mut app,
        SettingsMessage::Browser(BrowserMessage::ComputerUseTimeoutMsChanged(" 5000 ".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Browser(BrowserMessage::ComputerUseMaxCoordinateXChanged(
            "-1".to_string(),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Browser(BrowserMessage::ComputerUseMaxCoordinateYChanged(
            "4096".to_string(),
        )),
    );

    assert_eq!(app.browser_settings.computer_use_timeout_ms_input, " 5000 ");
    assert_eq!(app.browser_settings.computer_use_max_coordinate_x_input, "-1");
    assert_eq!(app.browser_settings.computer_use_max_coordinate_y_input, "4096");
    assert!(app.browser_settings.save_error.is_none());
}

#[test]
fn update_reports_timeout_and_coordinate_validation_errors() {
    let mut timeout_app = app();
    let _ = update(
        &mut timeout_app,
        SettingsMessage::Browser(BrowserMessage::ComputerUseTimeoutMsChanged("0".to_string())),
    );
    assert_eq!(
        timeout_app.browser_settings.save_error.as_deref(),
        Some("computer_use.timeout_ms 必须大于 0")
    );

    let mut x_app = app();
    let _ = update(
        &mut x_app,
        SettingsMessage::Browser(BrowserMessage::ComputerUseMaxCoordinateXChanged("x".to_string())),
    );
    assert_eq!(
        x_app.browser_settings.save_error.as_deref(),
        Some("computer_use.max_coordinate_x 必须是整数")
    );

    let mut y_app = app();
    let _ = update(
        &mut y_app,
        SettingsMessage::Browser(BrowserMessage::ComputerUseMaxCoordinateYChanged("y".to_string())),
    );
    assert_eq!(
        y_app.browser_settings.save_error.as_deref(),
        Some("computer_use.max_coordinate_y 必须是整数")
    );
}

#[test]
fn persist_browser_settings_accepts_blank_defaultable_and_optional_fields() {
    let mut app = app();
    app.browser_settings.allowed_domains_input = " ,\n ".to_string();
    app.browser_settings.browser_open = "unknown".to_string();
    app.browser_settings.session_name_input = "  ".to_string();
    app.browser_settings.backend = "unknown".to_string();
    app.browser_settings.native_webdriver_url = "  ".to_string();
    app.browser_settings.native_chrome_path_input = "  ".to_string();
    app.browser_settings.computer_use_endpoint = "  ".to_string();
    app.browser_settings.computer_use_api_key_input = "  ".to_string();
    app.browser_settings.computer_use_timeout_ms_input = "  ".to_string();
    app.browser_settings.computer_use_window_allowlist_input = " ,\n ".to_string();
    app.browser_settings.computer_use_max_coordinate_x_input = "  ".to_string();
    app.browser_settings.computer_use_max_coordinate_y_input = "  ".to_string();

    assert!(persist_browser_settings(&mut app).is_ok());
}

#[test]
fn persist_browser_settings_accepts_explicit_optional_fields() {
    let mut app = app();
    app.browser_settings.allowed_domains_input = "example.com, api.example.com".to_string();
    app.browser_settings.browser_open = "new-tab".to_string();
    app.browser_settings.session_name_input = " session ".to_string();
    app.browser_settings.backend = "computer-use".to_string();
    app.browser_settings.native_webdriver_url = " http://webdriver.example ".to_string();
    app.browser_settings.native_chrome_path_input = " /opt/chrome ".to_string();
    app.browser_settings.computer_use_endpoint = " http://computer-use.example ".to_string();
    app.browser_settings.computer_use_api_key_input = " key-placeholder ".to_string();
    app.browser_settings.computer_use_timeout_ms_input = "30000".to_string();
    app.browser_settings.computer_use_window_allowlist_input = "Terminal, Browser".to_string();
    app.browser_settings.computer_use_max_coordinate_x_input = "1920".to_string();
    app.browser_settings.computer_use_max_coordinate_y_input = "1080".to_string();

    assert!(persist_browser_settings(&mut app).is_ok());
}
