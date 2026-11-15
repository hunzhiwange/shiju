//! 从已加载配置构造本领域设置；不执行网络或宿主操作。

pub fn build_browser_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> crate::state::BrowserSettingsState {
    let browser_cfg = &full_agent_cfg.browser;
    let allowed_domains = browser_cfg.allowed_domains.join("\n");

    crate::state::BrowserSettingsState {
        installing: false,
        enabled: browser_cfg.enabled,
        allowed_domains_input: allowed_domains.clone(),
        allowed_domains_editor: iced::widget::text_editor::Content::with_text(&allowed_domains),
        browser_open: match browser_cfg.browser_open.trim().to_ascii_lowercase().as_str() {
            "default" | "new_window" | "new_tab" => {
                browser_cfg.browser_open.trim().to_ascii_lowercase()
            }
            _ => "default".to_string(),
        },
        session_name_input: browser_cfg.session_name.clone().unwrap_or_default(),
        backend: match browser_cfg.backend.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "agent_browser" => "agent_browser".to_string(),
            "rust_native" | "native" => "native".to_string(),
            "computer_use" => "computer_use".to_string(),
            "auto" => "auto".to_string(),
            _ => "agent_browser".to_string(),
        },
        native_headless: browser_cfg.native_headless,
        native_webdriver_url: browser_cfg.native_webdriver_url.clone(),
        native_chrome_path_input: browser_cfg.native_chrome_path.clone().unwrap_or_default(),
        computer_use_endpoint: browser_cfg.computer_use.endpoint.clone(),
        computer_use_api_key_input: browser_cfg.computer_use.api_key.clone().unwrap_or_default(),
        computer_use_timeout_ms_input: browser_cfg.computer_use.timeout_ms.to_string(),
        computer_use_allow_remote_endpoint: browser_cfg.computer_use.allow_remote_endpoint,
        computer_use_window_allowlist_input: browser_cfg.computer_use.window_allowlist.join(", "),
        computer_use_max_coordinate_x_input: browser_cfg
            .computer_use
            .max_coordinate_x
            .map(|value| value.to_string())
            .unwrap_or_default(),
        computer_use_max_coordinate_y_input: browser_cfg
            .computer_use
            .max_coordinate_y
            .map(|value| value.to_string())
            .unwrap_or_default(),
        save_error: None,
        save_error_translation: None,
    }
}

pub fn build_http_request_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> super::state::HttpRequestSettingsState {
    let http_request_cfg = &full_agent_cfg.http_request;

    super::state::HttpRequestSettingsState {
        enabled: http_request_cfg.enabled,
        allowed_domains: http_request_cfg.allowed_domains.clone(),
        new_allowed_domain_input: String::new(),
        max_response_size: http_request_cfg.max_response_size.min(u32::MAX as usize) as u32,
        timeout_secs: http_request_cfg.timeout_secs.min(u32::MAX as u64) as u32,
        user_agent: http_request_cfg.user_agent.clone(),
        save_error: None,
    }
}

pub fn build_gateway_settings(
    gateway_cfg_result: &Result<config_gateway::GatewayConfig, String>,
) -> crate::state::GatewaySettingsState {
    let gateway_cfg = gateway_cfg_result.clone().unwrap_or_default();

    crate::state::GatewaySettingsState {
        active_tab: crate::state::GatewaySettingsTab::Config,
        port: gateway_cfg.port.clamp(1, u16::MAX),
        host_input: {
            let value = gateway_cfg.host.trim().to_string();
            if value.is_empty() { "127.0.0.1".to_string() } else { value }
        },
        auth_enabled: gateway_cfg.auth_enabled,
        skeys: gateway_cfg.skeys,
        new_skey_name_input: String::new(),
        new_skey_expires_at_input: String::new(),
        new_skey_calendar_month: String::new(),
        new_skey_calendar_open: false,
        last_created_skey: None,
        last_created_skey_copied: false,
        allow_public_bind: gateway_cfg.allow_public_bind,
        webhook_rate_limit_per_minute: gateway_cfg.webhook_rate_limit_per_minute.clamp(1, 100_000),
        trust_forwarded_headers: gateway_cfg.trust_forwarded_headers,
        rate_limit_max_keys: gateway_cfg.rate_limit_max_keys.min(u32::MAX as usize) as u32,
        idempotency_ttl_secs: gateway_cfg.idempotency_ttl_secs.min(u32::MAX as u64) as u32,
        idempotency_max_keys: gateway_cfg.idempotency_max_keys.min(u32::MAX as usize) as u32,
        node_control_enabled: gateway_cfg.node_control.enabled,
        node_control_auth_token_input: gateway_cfg.node_control.auth_token.unwrap_or_default(),
        node_control_allowed_node_ids_input: gateway_cfg.node_control.allowed_node_ids.join("\n"),
        service_action_running: None,
        service_action_output: None,
        copied_service_command: None,
        pending_service_command_confirmation: None,
        show_help_modal: false,
        save_error: None,
    }
}

pub fn build_web_search_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> super::state::WebSearchSettingsState {
    let web_search_cfg = &full_agent_cfg.web_search;

    super::state::WebSearchSettingsState {
        enabled: web_search_cfg.enabled,
        provider: match web_search_cfg.provider.trim().to_ascii_lowercase().as_str() {
            "ddg" | "duckduckgo" => "duckduckgo".to_string(),
            "brave" => "brave".to_string(),
            "serper" => "serper".to_string(),
            "google" => "google".to_string(),
            "bing" => "bing".to_string(),
            _ => "duckduckgo".to_string(),
        },
        api_key_input: web_search_cfg.api_key.clone().unwrap_or_default(),
        api_url_input: web_search_cfg.api_url.clone().unwrap_or_default(),
        brave_api_key_input: web_search_cfg.brave_api_key.clone().unwrap_or_default(),
        max_results_input: web_search_cfg.max_results.clamp(1, 10).to_string(),
        timeout_secs_input: web_search_cfg.timeout_secs.max(1).to_string(),
        user_agent: if web_search_cfg.user_agent.trim().is_empty() {
            config_tools::DEFAULT_USER_AGENT.to_string()
        } else {
            web_search_cfg.user_agent.clone()
        },
        show_help_modal: false,
        save_error: None,
    }
}

pub fn build_proxy_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> super::state::ProxySettingsState {
    let proxy_cfg = &full_agent_cfg.proxy;

    super::state::ProxySettingsState {
        enabled: proxy_cfg.enabled,
        http_proxy: proxy_cfg.http_proxy.clone().unwrap_or_default(),
        https_proxy: proxy_cfg.https_proxy.clone().unwrap_or_default(),
        all_proxy: proxy_cfg.all_proxy.clone().unwrap_or_default(),
        no_proxy_input: proxy_cfg.no_proxy.join(", "),
        scope: proxy_cfg.scope,
        services_input: proxy_cfg.services.join(", "),
        show_help_modal: false,
        save_error: None,
    }
}

pub fn build_tunnel_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> super::state::TunnelSettingsState {
    let tunnel_cfg = &full_agent_cfg.tunnel;

    super::state::TunnelSettingsState {
        provider: match tunnel_cfg.provider.trim().to_ascii_lowercase().as_str() {
            "cloudflare" => "cloudflare".to_string(),
            "tailscale" => "tailscale".to_string(),
            "ngrok" => "ngrok".to_string(),
            "custom" => "custom".to_string(),
            _ => "none".to_string(),
        },
        cloudflare_token: tunnel_cfg
            .cloudflare
            .as_ref()
            .map(|config| config.token.clone())
            .unwrap_or_default(),
        tailscale_funnel: tunnel_cfg
            .tailscale
            .as_ref()
            .map(|config| config.funnel)
            .unwrap_or(false),
        tailscale_hostname: tunnel_cfg
            .tailscale
            .as_ref()
            .and_then(|config| config.hostname.clone())
            .unwrap_or_default(),
        ngrok_auth_token: tunnel_cfg
            .ngrok
            .as_ref()
            .map(|config| config.auth_token.clone())
            .unwrap_or_default(),
        ngrok_domain: tunnel_cfg
            .ngrok
            .as_ref()
            .and_then(|config| config.domain.clone())
            .unwrap_or_default(),
        custom_start_command: tunnel_cfg
            .custom
            .as_ref()
            .map(|config| config.start_command.clone())
            .unwrap_or_default(),
        custom_health_url: tunnel_cfg
            .custom
            .as_ref()
            .and_then(|config| config.health_url.clone())
            .unwrap_or_default(),
        custom_url_pattern: tunnel_cfg
            .custom
            .as_ref()
            .and_then(|config| config.url_pattern.clone())
            .unwrap_or_default(),
        save_error: None,
    }
}

/// 构造本机网关连接编辑状态，保留旧凭据回退规则。
pub fn build_gateway_client_settings(
    gateway_client_cfg: &config_ui::GatewayClientSystemSettingsConfig,
) -> crate::state::GatewayClientSettingsState {
    crate::state::GatewayClientSettingsState {
        selected_server_id: gateway_client_cfg.active_server().id.clone(),
        name_input: gateway_client_cfg.active_server().name.clone(),
        servers: gateway_client_cfg
            .normalized_servers()
            .iter()
            .map(crate::state::GatewayClientServerDraft::from_config)
            .collect(),
        health: std::collections::HashMap::new(),
        host_input: {
            let value = gateway_client_cfg.active_server().host.trim().to_string();
            if value.is_empty() { "127.0.0.1".to_string() } else { value }
        },
        port: gateway_client_cfg.active_server().port.clamp(1, u16::MAX),
        platform_id_input: gateway_client_cfg.active_server().platform_id.clone(),
        company_id_input: gateway_client_cfg.active_server().company_id.clone(),
        skey_input: {
            let active = gateway_client_cfg.active_server();
            if active.skey.trim().is_empty() { active.bearer_token } else { active.skey }
        },
        pending_remove_server_id: None,
        show_help_modal: false,
        save_error: None,
    }
}
