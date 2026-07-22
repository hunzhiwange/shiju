use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum PreviewAutoSaveMode {
    Off,
    AfterDelay,
    #[default]
    OnFocusChange,
    OnWindowChange,
}

impl PreviewAutoSaveMode {
    pub const ALL: [Self; 4] =
        [Self::Off, Self::AfterDelay, Self::OnFocusChange, Self::OnWindowChange];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "关闭",
            Self::AfterDelay => "延迟保存",
            Self::OnFocusChange => "编辑器失焦时保存",
            Self::OnWindowChange => "窗口失焦时保存",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Off => "不自动保存，需手动保存当前文件。",
            Self::AfterDelay => "编辑内容变更后，等待短暂延迟再自动保存。",
            Self::OnFocusChange => "当编辑器失去焦点时，自动保存当前文件。",
            Self::OnWindowChange => "当应用窗口失去焦点时，自动保存当前文件。",
        }
    }
}

impl std::fmt::Display for PreviewAutoSaveMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq)]
pub struct ModelRoute {
    #[serde(default)]
    pub pattern: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub priority: u32,
}

/// Desktop app UI configuration (`[app_ui]` section).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq)]
pub struct AppUiConfig {
    /// System settings values managed by the desktop UI.
    #[serde(default)]
    pub system_settings: AppSystemSettingsConfig,
}

/// Desktop system settings persisted in agent config (`[app_ui.system_settings]`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AppSystemSettingsConfig {
    /// Desktop gateway client connection settings.
    #[serde(default)]
    pub gateway_client: GatewayClientSystemSettingsConfig,
    /// Iced theme name (`Theme::to_string()`).
    #[serde(default = "default_app_theme_name")]
    pub app_theme: String,
    /// Terminal shell key (`bash` | `zsh`).
    #[serde(default = "default_terminal_shell")]
    pub terminal_shell: String,
    /// Terminal theme key (`system` | `ui` | `solarized_dark` | `monokai`).
    #[serde(default = "default_terminal_theme")]
    pub terminal_theme: String,
    /// Terminal font family.
    #[serde(default = "default_terminal_font_family")]
    pub terminal_font_family: String,
    /// Terminal font size.
    #[serde(default = "default_terminal_font_size")]
    pub terminal_font_size: f32,
    /// Whether editor theme follows system/app theme.
    #[serde(default = "default_true")]
    pub editor_follow_system_theme: bool,
    /// Iced editor theme name (`Theme::to_string()`).
    #[serde(default = "default_app_theme_name")]
    pub editor_theme: String,
    /// Per-project worktree toggle map.
    #[serde(default)]
    pub project_worktree_enabled: HashMap<String, bool>,
    /// Preview/editor font size.
    #[serde(default = "default_editor_font_size")]
    pub editor_font_size: f32,
    /// Preview/editor line height.
    #[serde(default = "default_editor_line_height")]
    pub editor_line_height: f32,
    /// Whether line height follows font size.
    #[serde(default = "default_true")]
    pub editor_auto_line_height: bool,
    /// Preview editor auto save mode.
    #[serde(default)]
    pub preview_auto_save: PreviewAutoSaveMode,
    #[serde(default = "default_true")]
    pub dialogue_flow_show_reasoning_summary: bool,
    #[serde(default = "default_false")]
    pub dialogue_flow_expand_shell_tool_section: bool,
    #[serde(default = "default_false")]
    pub dialogue_flow_expand_edit_tool_section: bool,
    /// Desktop-managed model routing rules.
    #[serde(default)]
    pub model_routes: Vec<ModelRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GatewayClientServerConfig {
    #[serde(default = "default_gateway_client_server_id")]
    pub id: String,
    #[serde(default = "default_gateway_client_server_name")]
    pub name: String,
    #[serde(default = "default_gateway_client_host")]
    pub host: String,
    #[serde(default = "default_gateway_client_port")]
    pub port: u16,
    #[serde(default = "default_platform_id", deserialize_with = "deserialize_tenant_id")]
    pub platform_id: String,
    #[serde(default = "default_company_id", deserialize_with = "deserialize_tenant_id")]
    pub company_id: String,
    /// 旧版 `bearer_token` 字段，仅用于读取旧配置并迁移到 skey。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bearer_token: String,
    /// 旧版用户名字段，仅用于读取旧配置。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub username: String,
    /// 旧版密码字段，仅用于读取旧配置。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub password: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub skey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GatewayClientSystemSettingsConfig {
    #[serde(default = "default_gateway_client_host")]
    pub host: String,
    #[serde(default = "default_gateway_client_port")]
    pub port: u16,
    #[serde(default = "default_platform_id", deserialize_with = "deserialize_tenant_id")]
    pub platform_id: String,
    #[serde(default = "default_company_id", deserialize_with = "deserialize_tenant_id")]
    pub company_id: String,
    /// 旧版 `bearer_token` 字段，仅用于读取旧配置并迁移到 skey。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bearer_token: String,
    /// 旧版用户名字段，仅用于读取旧配置。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub username: String,
    /// 旧版密码字段，仅用于读取旧配置。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub password: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub skey: String,
    #[serde(default)]
    pub active_server_id: String,
    #[serde(default)]
    pub servers: Vec<GatewayClientServerConfig>,
}

fn default_app_theme_name() -> String {
    "Light".to_string()
}

fn default_gateway_client_host() -> String {
    "127.0.0.1".to_string()
}

fn default_gateway_client_port() -> u16 {
    42617
}

fn default_platform_id() -> String {
    "10000".to_string()
}

fn default_company_id() -> String {
    "100000000".to_string()
}

fn deserialize_tenant_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.trim().is_empty() {
        return Err(serde::de::Error::custom("tenant id must not be empty"));
    }
    Ok(value)
}

fn default_gateway_client_server_id() -> String {
    "local".to_string()
}

fn default_gateway_client_server_name() -> String {
    "本地网关".to_string()
}

fn default_terminal_shell() -> String {
    "zsh".to_string()
}

fn default_terminal_theme() -> String {
    "system".to_string()
}

fn default_terminal_font_family() -> String {
    "JetBrains Mono".to_string()
}

fn default_terminal_font_size() -> f32 {
    13.0
}

fn default_editor_font_size() -> f32 {
    14.0
}

fn default_editor_line_height() -> f32 {
    20.0
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

impl Default for AppSystemSettingsConfig {
    fn default() -> Self {
        Self {
            gateway_client: GatewayClientSystemSettingsConfig::default(),
            app_theme: default_app_theme_name(),
            terminal_shell: default_terminal_shell(),
            terminal_theme: default_terminal_theme(),
            terminal_font_family: default_terminal_font_family(),
            terminal_font_size: default_terminal_font_size(),
            editor_follow_system_theme: default_true(),
            editor_theme: default_app_theme_name(),
            project_worktree_enabled: HashMap::new(),
            editor_font_size: default_editor_font_size(),
            editor_line_height: default_editor_line_height(),
            editor_auto_line_height: default_true(),
            preview_auto_save: PreviewAutoSaveMode::default(),
            dialogue_flow_show_reasoning_summary: default_true(),
            dialogue_flow_expand_shell_tool_section: default_false(),
            dialogue_flow_expand_edit_tool_section: default_false(),
            model_routes: Vec::new(),
        }
    }
}

impl Default for GatewayClientSystemSettingsConfig {
    fn default() -> Self {
        Self {
            host: default_gateway_client_host(),
            port: default_gateway_client_port(),
            platform_id: default_platform_id(),
            company_id: default_company_id(),
            bearer_token: String::new(),
            username: String::new(),
            password: String::new(),
            skey: String::new(),
            active_server_id: default_gateway_client_server_id(),
            servers: vec![GatewayClientServerConfig::default()],
        }
    }
}

impl Default for GatewayClientServerConfig {
    fn default() -> Self {
        Self {
            id: default_gateway_client_server_id(),
            name: default_gateway_client_server_name(),
            host: default_gateway_client_host(),
            port: default_gateway_client_port(),
            platform_id: default_platform_id(),
            company_id: default_company_id(),
            bearer_token: String::new(),
            username: String::new(),
            password: String::new(),
            skey: String::new(),
        }
    }
}

impl GatewayClientSystemSettingsConfig {
    pub fn normalized_servers(&self) -> Vec<GatewayClientServerConfig> {
        if self.servers.is_empty() {
            return vec![GatewayClientServerConfig {
                id: default_gateway_client_server_id(),
                name: default_gateway_client_server_name(),
                host: self.host.clone(),
                port: self.port,
                platform_id: self.platform_id.clone(),
                company_id: self.company_id.clone(),
                bearer_token: self.bearer_token.clone(),
                username: self.username.clone(),
                password: self.password.clone(),
                skey: self.skey.clone(),
            }];
        }

        self.servers
            .iter()
            .enumerate()
            .map(|(index, server)| {
                let mut server = server.clone();
                if server.id.trim().is_empty() {
                    server.id = if index == 0 {
                        default_gateway_client_server_id()
                    } else {
                        format!("gateway-{}", index + 1)
                    };
                }
                if server.name.trim().is_empty() {
                    server.name = if index == 0 {
                        default_gateway_client_server_name()
                    } else {
                        format!("网关 {}", index + 1)
                    };
                }
                server
            })
            .collect()
    }

    pub fn active_server(&self) -> GatewayClientServerConfig {
        let servers = self.normalized_servers();
        servers
            .iter()
            .find(|server| server.id == self.active_server_id)
            .cloned()
            .or_else(|| servers.first().cloned())
            .unwrap_or_default()
    }

    pub fn set_servers(
        &mut self,
        servers: Vec<GatewayClientServerConfig>,
        active_server_id: String,
    ) {
        let servers =
            if servers.is_empty() { vec![GatewayClientServerConfig::default()] } else { servers };
        let active = servers
            .iter()
            .find(|server| server.id == active_server_id)
            .or_else(|| servers.first())
            .cloned()
            .unwrap_or_default();

        self.active_server_id = active.id.clone();
        self.host = active.host.clone();
        self.port = active.port;
        self.platform_id = active.platform_id.clone();
        self.company_id = active.company_id.clone();
        self.bearer_token = active.bearer_token.clone();
        self.username = active.username.clone();
        self.password = active.password.clone();
        self.skey = active.skey.clone();
        self.servers = servers;
    }
}
#[cfg(test)]
#[path = "ui_tests.rs"]
mod ui_tests;
