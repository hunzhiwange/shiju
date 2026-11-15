//! 其他通道配置类型。
//!
//! 本模块包含 Signal、WhatsApp、IRC、Nostr 等不在常见聊天平台模块中的
//! 通道配置结构。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Signal 通道配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SignalConfig {
    /// Signal HTTP bridge 地址。
    pub http_url: String,
    /// Signal 账户标识。
    pub account: String,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub allowed_from: Vec<String>,
    #[serde(default)]
    pub ignore_attachments: bool,
    #[serde(default)]
    pub ignore_stories: bool,
}

/// WhatsApp 通道配置。
///
/// 同时兼容云 API 模式与本地 Web 会话模式。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WhatsAppConfig {
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub phone_number_id: Option<String>,
    #[serde(default)]
    pub verify_token: Option<String>,
    #[serde(default)]
    pub app_secret: Option<String>,
    #[serde(default)]
    pub session_path: Option<String>,
    #[serde(default)]
    pub pair_phone: Option<String>,
    #[serde(default)]
    pub pair_code: Option<String>,
    #[serde(default)]
    pub allowed_numbers: Vec<String>,
}

impl WhatsAppConfig {
    /// 返回当前配置推断出的后端类型。
    pub fn backend_type(&self) -> &'static str {
        if self.phone_number_id.is_some() {
            "cloud"
        } else if self.session_path.is_some() {
            "web"
        } else {
            "cloud"
        }
    }

    /// 判断是否为云 API 配置。
    pub fn is_cloud_config(&self) -> bool {
        self.phone_number_id.is_some() && self.access_token.is_some() && self.verify_token.is_some()
    }

    /// 判断是否为 Web 会话配置。
    pub fn is_web_config(&self) -> bool {
        self.session_path.is_some()
    }

    /// 判断是否同时配置了互斥的云 API 与 Web 会话字段。
    pub fn is_ambiguous_config(&self) -> bool {
        self.phone_number_id.is_some() && self.session_path.is_some()
    }
}

/// Linq 通道配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LinqConfig {
    pub api_token: String,
    pub from_phone: String,
    #[serde(default)]
    pub signing_secret: Option<String>,
    #[serde(default)]
    pub allowed_senders: Vec<String>,
}

fn default_wati_api_url() -> String {
    "https://live-mt-server.wati.io".to_string()
}

/// Wati 通道配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WatiConfig {
    pub api_token: String,
    #[serde(default = "default_wati_api_url")]
    pub api_url: String,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub allowed_numbers: Vec<String>,
}

/// Nextcloud Talk 通道配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NextcloudTalkConfig {
    pub base_url: String,
    pub app_token: String,
    #[serde(default)]
    pub webhook_secret: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

fn default_irc_port() -> u16 {
    6697
}

/// IRC 通道配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IrcConfig {
    pub server: String,
    #[serde(default = "default_irc_port")]
    pub port: u16,
    pub nickname: String,
    pub username: Option<String>,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
    pub server_password: Option<String>,
    pub nickserv_password: Option<String>,
    pub sasl_password: Option<String>,
    pub verify_tls: Option<bool>,
}

/// DingTalk 通道配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DingTalkConfig {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

/// 命名 DingTalk 通道实例配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DingTalkInstanceConfig {
    /// 稳定实例 ID，用于员工绑定和 profile ref。
    pub id: String,
    /// DingTalk 通道原有配置字段。
    #[serde(flatten)]
    pub config: DingTalkConfig,
}

/// QQ 接收模式。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum QQReceiveMode {
    /// 通过 WebSocket 接收消息。
    Websocket,
    /// 通过 Webhook 接收消息。
    #[default]
    Webhook,
}

/// QQ 通道配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QQConfig {
    pub app_id: String,
    pub app_secret: String,
    #[serde(default)]
    pub allowed_users: Vec<String>,
    #[serde(default)]
    pub receive_mode: QQReceiveMode,
}

/// 默认 Nostr relay 列表。
pub fn default_nostr_relays() -> Vec<String> {
    vec![
        "wss://relay.damus.io".to_string(),
        "wss://nos.lol".to_string(),
        "wss://relay.primal.net".to_string(),
        "wss://relay.snort.social".to_string(),
    ]
}

/// Nostr 通道配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NostrConfig {
    pub private_key: String,
    #[serde(default = "default_nostr_relays")]
    pub relays: Vec<String>,
    #[serde(default)]
    pub allowed_pubkeys: Vec<String>,
}
#[cfg(test)]
#[path = "other_tests.rs"]
mod other_tests;
