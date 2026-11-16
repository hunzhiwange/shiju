pub fn selected_gateway_health_key(
    settings: &crate::state::GatewayClientSettingsState,
) -> Option<String> {
    let server = settings.servers.iter().find(|server| server.id == settings.selected_server_id)?;
    crate::server_health_key(server)
}

pub fn selected_gateway_became_healthy(
    settings: &crate::state::GatewayClientSettingsState,
    results: &[(String, bool)],
) -> bool {
    selected_gateway_health_key(settings).is_some_and(|key| {
        settings.health.get(&key) != Some(&true)
            && results.iter().any(|(candidate, healthy)| *healthy && candidate == &key)
    })
}

pub fn apply_bootstrap_gateway_client(
    state: &mut crate::state::GatewayClientSettingsState,
    config: config_ui::GatewayClientSystemSettingsConfig,
) {
    let active_gateway_client = config.active_server();
    state.servers = config
        .normalized_servers()
        .iter()
        .map(crate::state::GatewayClientServerDraft::from_config)
        .collect();
    let gateway_health_keys = state
        .servers
        .iter()
        .filter_map(crate::server_health_key)
        .collect::<std::collections::HashSet<_>>();
    state.health.retain(|key, _| gateway_health_keys.contains(key));
    state.selected_server_id = active_gateway_client.id;
    state.name_input = active_gateway_client.name;
    state.host_input = {
        let value = active_gateway_client.host.trim().to_string();
        if value.is_empty() { "127.0.0.1".to_string() } else { value }
    };
    state.port = active_gateway_client.port.clamp(1, u16::MAX);
    state.platform_id_input = active_gateway_client.platform_id;
    state.company_id_input = active_gateway_client.company_id;
    state.skey_input = active_gateway_client.skey;
}

pub fn apply_bootstrap_browser_settings(
    state: &mut crate::state::BrowserSettingsState,
    browser: config_tools::BrowserConfig,
) {
    state.enabled = browser.enabled;
    state.allowed_domains_input = browser.allowed_domains.join("\n");
    state.allowed_domains_editor =
        iced::widget::text_editor::Content::with_text(&browser.allowed_domains.join("\n"));
    state.browser_open = match browser.browser_open.trim().to_ascii_lowercase().as_str() {
        "default" | "new_window" | "new_tab" => browser.browser_open.trim().to_ascii_lowercase(),
        _ => "default".to_string(),
    };
    state.session_name_input = browser.session_name.unwrap_or_default();
    state.backend = match browser.backend.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "agent_browser" => "agent_browser".to_string(),
        "rust_native" | "native" => "native".to_string(),
        "computer_use" => "computer_use".to_string(),
        "auto" => "auto".to_string(),
        _ => "agent_browser".to_string(),
    };
    state.native_headless = browser.native_headless;
    state.native_webdriver_url = browser.native_webdriver_url;
    state.native_chrome_path_input = browser.native_chrome_path.unwrap_or_default();
    state.computer_use_endpoint = browser.computer_use.endpoint;
    state.computer_use_api_key_input = browser.computer_use.api_key.unwrap_or_default();
    state.computer_use_timeout_ms_input = browser.computer_use.timeout_ms.to_string();
    state.computer_use_allow_remote_endpoint = browser.computer_use.allow_remote_endpoint;
    state.computer_use_window_allowlist_input = browser.computer_use.window_allowlist.join(", ");
    state.computer_use_max_coordinate_x_input =
        browser.computer_use.max_coordinate_x.map(|value| value.to_string()).unwrap_or_default();
    state.computer_use_max_coordinate_y_input =
        browser.computer_use.max_coordinate_y.map(|value| value.to_string()).unwrap_or_default();
}

pub fn apply_health_results(
    state: &mut crate::state::GatewayClientSettingsState,
    results: Vec<(String, bool)>,
) {
    let known_keys = state
        .servers
        .iter()
        .filter_map(crate::server_health_key)
        .collect::<std::collections::HashSet<_>>();
    state.health.retain(|key, _| known_keys.contains(key));
    for (key, healthy) in results {
        if known_keys.contains(&key) {
            state.health.insert(key, healthy);
        }
    }
}
