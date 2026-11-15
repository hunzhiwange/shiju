//! 浏览器与网关设置状态及配置转换。
use std::collections::HashMap;
use iced::widget::text_editor;

#[derive(Debug, Clone)]
pub struct BrowserSettingsState {
    pub installing: bool,
    pub enabled: bool,
    pub allowed_domains_input: String,
    pub allowed_domains_editor: text_editor::Content,
    pub browser_open: String,
    pub session_name_input: String,
    pub backend: String,
    pub native_headless: bool,
    pub native_webdriver_url: String,
    pub native_chrome_path_input: String,
    pub computer_use_endpoint: String,
    pub computer_use_api_key_input: String,
    pub computer_use_timeout_ms_input: String,
    pub computer_use_allow_remote_endpoint: bool,
    pub computer_use_window_allowlist_input: String,
    pub computer_use_max_coordinate_x_input: String,
    pub computer_use_max_coordinate_y_input: String,
    pub save_error: Option<String>,
    pub save_error_translation: Option<desktop_i18n::format::Message>,
}

impl Default for BrowserSettingsState {
    fn default() -> Self {
        let config = config_tools::BrowserConfig::default();
        let allowed_domains_input = config.allowed_domains.join("\n");
        Self {
            installing: false,
            enabled: config.enabled,
            allowed_domains_editor: text_editor::Content::with_text(&allowed_domains_input),
            allowed_domains_input,
            browser_open: "new_tab".to_string(),
            session_name_input: String::new(),
            backend: "agent_browser".to_string(),
            native_headless: true,
            native_webdriver_url: "http://127.0.0.1:9515".to_string(),
            native_chrome_path_input: String::new(),
            computer_use_endpoint: "http://127.0.0.1:8787/v1/actions".to_string(),
            computer_use_api_key_input: String::new(),
            computer_use_timeout_ms_input: "15000".to_string(),
            computer_use_allow_remote_endpoint: false,
            computer_use_window_allowlist_input: String::new(),
            computer_use_max_coordinate_x_input: String::new(),
            computer_use_max_coordinate_y_input: String::new(),
            save_error: None,
            save_error_translation: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewaySettingsTab {
    Config,
    Skeys,
}

#[derive(Debug, Clone)]
pub struct GatewaySettingsState {
    pub active_tab: GatewaySettingsTab,
    pub port: u16,
    pub host_input: String,
    pub auth_enabled: bool,
    pub skeys: Vec<config_gateway::GatewaySkey>,
    pub new_skey_name_input: String,
    pub new_skey_expires_at_input: String,
    pub new_skey_calendar_month: String,
    pub new_skey_calendar_open: bool,
    pub last_created_skey: Option<String>,
    pub last_created_skey_copied: bool,
    pub allow_public_bind: bool,
    pub webhook_rate_limit_per_minute: u32,
    pub trust_forwarded_headers: bool,
    pub rate_limit_max_keys: u32,
    pub idempotency_ttl_secs: u32,
    pub idempotency_max_keys: u32,
    pub node_control_enabled: bool,
    pub node_control_auth_token_input: String,
    pub node_control_allowed_node_ids_input: String,
    pub service_action_running: Option<String>,
    pub service_action_output: Option<desktop_i18n::settings_models::messages::Feedback>,
    pub copied_service_command: Option<String>,
    pub pending_service_command_confirmation: Option<String>,
    pub show_help_modal: bool,
    pub save_error: Option<desktop_i18n::settings_models::messages::Feedback>,
}

impl Default for GatewaySettingsState {
    fn default() -> Self {
        Self {
            active_tab: GatewaySettingsTab::Config,
            port: config_gateway::DEFAULT_GATEWAY_PORT,
            host_input: "127.0.0.1".to_string(),
            auth_enabled: false,
            skeys: Vec::new(),
            new_skey_name_input: String::new(),
            new_skey_expires_at_input: String::new(),
            new_skey_calendar_month: String::new(),
            new_skey_calendar_open: false,
            last_created_skey: None,
            last_created_skey_copied: false,
            allow_public_bind: false,
            webhook_rate_limit_per_minute: 60,
            trust_forwarded_headers: false,
            rate_limit_max_keys: 10_000,
            idempotency_ttl_secs: 300,
            idempotency_max_keys: 10_000,
            node_control_enabled: false,
            node_control_auth_token_input: String::new(),
            node_control_allowed_node_ids_input: String::new(),
            service_action_running: None,
            service_action_output: None,
            copied_service_command: None,
            pending_service_command_confirmation: None,
            show_help_modal: false,
            save_error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GatewayClientServerDraft {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub platform_id: String,
    pub company_id: String,
    pub skey: String,
}

impl GatewayClientServerDraft {
    pub fn from_config(server: &config_ui::GatewayClientServerConfig) -> Self {
        Self {
            id: server.id.clone(),
            name: server.name.clone(),
            host: server.host.clone(),
            port: server.port.clamp(1, u16::MAX),
            platform_id: server.platform_id.clone(),
            company_id: server.company_id.clone(),
            skey: if server.skey.trim().is_empty() {
                server.bearer_token.clone()
            } else {
                server.skey.clone()
            },
        }
    }

    pub fn to_config(&self) -> config_ui::GatewayClientServerConfig {
        config_ui::GatewayClientServerConfig {
            id: self.id.clone(),
            name: self.name.clone(),
            host: self.host.clone(),
            port: self.port.clamp(1, u16::MAX),
            platform_id: self.platform_id.clone(),
            company_id: self.company_id.clone(),
            bearer_token: String::new(),
            username: String::new(),
            password: String::new(),
            skey: self.skey.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GatewayClientSettingsState {
    pub servers: Vec<GatewayClientServerDraft>,
    pub selected_server_id: String,
    pub health: HashMap<String, bool>,
    pub name_input: String,
    pub host_input: String,
    pub port: u16,
    pub platform_id_input: String,
    pub company_id_input: String,
    pub skey_input: String,
    pub pending_remove_server_id: Option<String>,
    pub show_help_modal: bool,
    pub save_error: Option<desktop_i18n::settings_models::messages::Feedback>,
}

impl Default for GatewayClientSettingsState {
    fn default() -> Self {
        let server = GatewayClientServerDraft::from_config(
            &config_ui::GatewayClientServerConfig::default(),
        );
        Self {
            selected_server_id: server.id.clone(),
            name_input: server.name.clone(),
            host_input: "127.0.0.1".to_string(),
            port: config_gateway::DEFAULT_GATEWAY_PORT,
            platform_id_input: server.platform_id.clone(),
            company_id_input: server.company_id.clone(),
            servers: vec![server],
            health: HashMap::new(),
            skey_input: String::new(),
            pending_remove_server_id: None,
            show_help_modal: false,
            save_error: None,
        }
    }
}
