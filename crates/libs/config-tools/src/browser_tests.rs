// 浏览器与计算机使用的嵌套配置需同时保护禁用默认和显式覆盖。
#[test]
fn browser_computer_use_config_defaults_and_overrides_deserialize() {
    let config = super::BrowserComputerUseConfig::default();
    assert_eq!(config.endpoint, "http://127.0.0.1:8787/v1/actions");
    assert_eq!(config.api_key, None);
    assert_eq!(config.timeout_ms, 15_000);
    assert!(!config.allow_remote_endpoint);
    assert!(config.window_allowlist.is_empty());
    assert_eq!(config.max_coordinate_x, None);
    assert_eq!(config.max_coordinate_y, None);

    let parsed: super::BrowserComputerUseConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(parsed.endpoint, "http://127.0.0.1:8787/v1/actions");
    assert_eq!(parsed.timeout_ms, 15_000);
    assert!(!parsed.allow_remote_endpoint);

    let custom: super::BrowserComputerUseConfig = serde_json::from_value(serde_json::json!({
        "endpoint": "http://localhost:9000/actions",
        "api_key": "token",
        "timeout_ms": 2500,
        "allow_remote_endpoint": true,
        "window_allowlist": ["Chrome", "Cursor"],
        "max_coordinate_x": 1600,
        "max_coordinate_y": 900
    }))
    .unwrap();
    assert_eq!(custom.endpoint, "http://localhost:9000/actions");
    assert_eq!(custom.api_key.as_deref(), Some("token"));
    assert_eq!(custom.timeout_ms, 2500);
    assert!(custom.allow_remote_endpoint);
    assert_eq!(custom.window_allowlist, vec!["Chrome", "Cursor"]);
    assert_eq!(custom.max_coordinate_x, Some(1600));
    assert_eq!(custom.max_coordinate_y, Some(900));
}

#[test]
fn browser_config_defaults_and_nested_computer_use_deserialize() {
    let config = super::BrowserConfig::default();
    assert!(config.enabled);
    assert_eq!(config.allowed_domains, ["*"]);
    assert_eq!(config.browser_open, "default");
    assert_eq!(config.session_name, None);
    assert_eq!(config.backend, "agent_browser");
    assert!(config.native_headless);
    assert_eq!(config.native_webdriver_url, "http://127.0.0.1:9515");
    assert_eq!(config.native_chrome_path, None);
    assert_eq!(config.computer_use.endpoint, "http://127.0.0.1:8787/v1/actions");

    let parsed: super::BrowserConfig = serde_json::from_str("{}").unwrap();
    assert!(parsed.enabled);
    assert_eq!(parsed.allowed_domains, ["*"]);
    assert_eq!(parsed.browser_open, "default");
    assert_eq!(parsed.backend, "agent_browser");
    assert!(parsed.native_headless);
    assert_eq!(parsed.computer_use.timeout_ms, 15_000);

    let custom: super::BrowserConfig = serde_json::from_value(serde_json::json!({
        "enabled": false,
        "allowed_domains": ["example.com", "docs.example.com"],
        "browser_open": "new_tab",
        "session_name": "qa",
        "backend": "computer_use",
        "native_headless": false,
        "native_webdriver_url": "http://localhost:4444",
        "native_chrome_path": "/Applications/Chromium.app",
        "computer_use": {
            "allow_remote_endpoint": true,
            "window_allowlist": ["Browser"]
        }
    }))
    .unwrap();
    assert!(!custom.enabled);
    assert_eq!(custom.allowed_domains, vec!["example.com", "docs.example.com"]);
    assert_eq!(custom.browser_open, "new_tab");
    assert_eq!(custom.session_name.as_deref(), Some("qa"));
    assert_eq!(custom.backend, "computer_use");
    assert!(!custom.native_headless);
    assert_eq!(custom.native_webdriver_url, "http://localhost:4444");
    assert_eq!(custom.native_chrome_path.as_deref(), Some("/Applications/Chromium.app"));
    assert!(custom.computer_use.allow_remote_endpoint);
    assert_eq!(custom.computer_use.window_allowlist, vec!["Browser"]);
    assert_eq!(custom.computer_use.endpoint, "http://127.0.0.1:8787/v1/actions");
}
