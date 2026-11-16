// Tests migrated from desktop host state; production types retain their original owners.
mod behavior_0 {
use crate::app_init::state::HttpRequestSettingsState;
use config_tools::HttpRequestConfig;
#[test]
fn http_request_defaults_match_config_ui() {
    let http = HttpRequestSettingsState::default();
    assert!(http.enabled);
    assert_eq!(http.allowed_domains, ["*"]);
    assert!(http.new_allowed_domain_input.is_empty());
    assert_eq!(http.max_response_size, 1_000_000);
    assert_eq!(http.timeout_secs, 30);
    assert_eq!(http.user_agent, HttpRequestConfig::default().user_agent);
    assert!(http.save_error.is_none());
}
}

mod behavior_1 {
use crate::app_init::state::WebSearchSettingsState;
use crate::state::*;
#[test]
fn web_search_browser_and_gateway_defaults_are_ready_for_empty_config() {
    let web = WebSearchSettingsState::default();
    assert!(web.enabled);
    assert_eq!(web.provider, "duckduckgo");
    assert!(web.api_key_input.is_empty());
    assert!(web.api_url_input.is_empty());
    assert!(web.brave_api_key_input.is_empty());
    assert_eq!(web.max_results_input, "5");
    assert_eq!(web.timeout_secs_input, "15");
    assert_eq!(web.user_agent, config_tools::DEFAULT_USER_AGENT);
    assert!(!web.show_help_modal);
    assert!(web.save_error.is_none());

    let browser = BrowserSettingsState::default();
    assert!(browser.enabled);
    assert_eq!(browser.allowed_domains_input, "*");
    assert_eq!(browser.allowed_domains_editor.text(), "*");
    assert_eq!(browser.browser_open, "new_tab");
    assert!(browser.session_name_input.is_empty());
    assert_eq!(browser.backend, "agent_browser");
    assert!(browser.native_headless);
    assert_eq!(browser.native_webdriver_url, "http://127.0.0.1:9515");
    assert!(browser.native_chrome_path_input.is_empty());
    assert_eq!(browser.computer_use_endpoint, "http://127.0.0.1:8787/v1/actions");
    assert!(browser.computer_use_api_key_input.is_empty());
    assert_eq!(browser.computer_use_timeout_ms_input, "15000");
    assert!(!browser.computer_use_allow_remote_endpoint);
    assert!(browser.computer_use_window_allowlist_input.is_empty());
    assert!(browser.computer_use_max_coordinate_x_input.is_empty());
    assert!(browser.computer_use_max_coordinate_y_input.is_empty());
    assert!(browser.save_error.is_none());

    let gateway = GatewaySettingsState::default();
    assert_eq!(gateway.active_tab, GatewaySettingsTab::Config);
    assert_eq!(gateway.port, config_gateway::DEFAULT_GATEWAY_PORT);
    assert_eq!(gateway.host_input, "127.0.0.1");
    assert!(!gateway.auth_enabled);
    assert!(!gateway.allow_public_bind);
    assert!(gateway.skeys.is_empty());
    assert!(gateway.new_skey_name_input.is_empty());
    assert!(gateway.new_skey_expires_at_input.is_empty());
    assert!(gateway.new_skey_calendar_month.is_empty());
    assert!(!gateway.new_skey_calendar_open);
    assert!(gateway.last_created_skey.is_none());
    assert!(!gateway.last_created_skey_copied);
    assert_eq!(gateway.webhook_rate_limit_per_minute, 60);
    assert!(!gateway.trust_forwarded_headers);
    assert_eq!(gateway.rate_limit_max_keys, 10_000);
    assert_eq!(gateway.idempotency_ttl_secs, 300);
    assert_eq!(gateway.idempotency_max_keys, 10_000);
    assert!(!gateway.node_control_enabled);
    assert!(gateway.node_control_auth_token_input.is_empty());
    assert!(gateway.node_control_allowed_node_ids_input.is_empty());
    assert!(gateway.service_action_running.is_none());
    assert!(gateway.service_action_output.is_none());
    assert!(gateway.copied_service_command.is_none());
    assert!(gateway.pending_service_command_confirmation.is_none());
    assert!(!gateway.show_help_modal);
    assert!(gateway.save_error.is_none());
}
}

mod behavior_2 {
use crate::state::GatewayClientServerDraft;
#[test]
fn gateway_client_server_draft_round_trips_config_and_clamps_port() {
    let config = config_ui::GatewayClientServerConfig {
        id: "remote".to_string(),
        name: "Remote".to_string(),
        host: "gw.example".to_string(),
        port: 0,
        platform_id: "20000".to_string(),
        company_id: "200000001".to_string(),
        bearer_token: "bearer".to_string(),
        username: "user".to_string(),
        password: "pass".to_string(),
        skey: "skey".to_string(),
    };

    let draft = GatewayClientServerDraft::from_config(&config);
    assert_eq!(draft.id, "remote");
    assert_eq!(draft.name, "Remote");
    assert_eq!(draft.host, "gw.example");
    assert_eq!(draft.port, 1);
    assert_eq!(draft.platform_id, "20000");
    assert_eq!(draft.company_id, "200000001");
    assert_eq!(draft.skey, "skey");

    let round_trip = draft.to_config();
    assert_eq!(round_trip.id, "remote");
    assert_eq!(round_trip.name, "Remote");
    assert_eq!(round_trip.host, "gw.example");
    assert_eq!(round_trip.port, 1);
    assert_eq!(round_trip.platform_id, "20000");
    assert_eq!(round_trip.company_id, "200000001");
    assert!(round_trip.bearer_token.is_empty());
    assert!(round_trip.username.is_empty());
    assert!(round_trip.password.is_empty());
    assert_eq!(round_trip.skey, "skey");
}
}

mod behavior_3 {
use crate::state::GatewayClientServerDraft;
#[test]
fn gateway_client_server_draft_uses_bearer_token_when_skey_is_blank() {
    let config = config_ui::GatewayClientServerConfig {
        id: "legacy".to_string(),
        name: "Legacy".to_string(),
        host: "legacy.example".to_string(),
        port: u16::MAX,
        platform_id: "platform".to_string(),
        company_id: "company".to_string(),
        bearer_token: "legacy-token".to_string(),
        username: String::new(),
        password: String::new(),
        skey: "  \n".to_string(),
    };

    let draft = GatewayClientServerDraft::from_config(&config);

    assert_eq!(draft.port, u16::MAX);
    assert_eq!(draft.skey, "legacy-token");
}
}

mod behavior_4 {
use crate::state::GatewayClientServerDraft;
#[test]
fn gateway_client_server_draft_to_config_clamps_zero_and_clears_legacy_credentials() {
    let draft = GatewayClientServerDraft {
        id: "manual".to_string(),
        name: "Manual".to_string(),
        host: "127.0.0.1".to_string(),
        port: 0,
        platform_id: "10000".to_string(),
        company_id: "100000000".to_string(),
        skey: "current-skey".to_string(),
    };

    let config = draft.to_config();

    assert_eq!(config.port, 1);
    assert!(config.bearer_token.is_empty());
    assert!(config.username.is_empty());
    assert!(config.password.is_empty());
    assert_eq!(config.skey, "current-skey");
}
}

mod behavior_5 {
use crate::state::GatewaySettingsTab;
#[test]
fn gateway_tab_variants_are_distinct() {
    assert_ne!(GatewaySettingsTab::Config, GatewaySettingsTab::Skeys);
}
}

mod behavior_6 {
use crate::app_init::state::{ProxySettingsState, TunnelSettingsState};
#[test]
fn storage_proxy_and_tunnel_defaults_are_stable_1() {
    let proxy = ProxySettingsState::default();
    assert!(!proxy.enabled);
    assert!(proxy.http_proxy.is_empty());
    assert!(proxy.https_proxy.is_empty());
    assert!(proxy.all_proxy.is_empty());
    assert!(proxy.no_proxy_input.is_empty());
    assert_eq!(proxy.scope, config_proxy::ProxyScope::Vibewindow);
    assert!(proxy.services_input.is_empty());
    assert!(!proxy.show_help_modal);
    assert!(proxy.save_error.is_none());

    let tunnel = TunnelSettingsState::default();
    assert_eq!(tunnel.provider, "none");
    assert!(!tunnel.tailscale_funnel);
    assert!(tunnel.cloudflare_token.is_empty());
    assert!(tunnel.tailscale_hostname.is_empty());
    assert!(tunnel.ngrok_auth_token.is_empty());
    assert!(tunnel.ngrok_domain.is_empty());
    assert!(tunnel.custom_start_command.is_empty());
    assert!(tunnel.custom_health_url.is_empty());
    assert!(tunnel.custom_url_pattern.is_empty());
    assert!(tunnel.save_error.is_none());
}
}

mod behavior_7 {
use crate::state::GatewayClientSettingsState;
#[test]
fn gateway_client_agents_coordination_and_cost_defaults_are_stable_0() {
    let client = GatewayClientSettingsState::default();
    assert_eq!(client.servers.len(), 1);
    assert_eq!(client.selected_server_id, client.servers[0].id);
    assert_eq!(client.name_input, client.servers[0].name);
    assert!(client.health.is_empty());
    assert_eq!(client.host_input, "127.0.0.1");
    assert_eq!(client.port, config_gateway::DEFAULT_GATEWAY_PORT);
    assert_eq!(client.platform_id_input, "10000");
    assert_eq!(client.company_id_input, "100000000");
    assert!(client.skey_input.is_empty());
    assert!(client.pending_remove_server_id.is_none());
    assert!(!client.show_help_modal);
    assert!(client.save_error.is_none());


}
}
