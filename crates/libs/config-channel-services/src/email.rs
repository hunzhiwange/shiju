//! 邮件通道配置模块。
//!
//! 用于定义 IMAP 收件与 SMTP 发件所需的连接参数。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 邮件通道配置。
///
/// 该结构同时覆盖 IMAP 收件与 SMTP 发件配置，适用于需要以邮箱作为代理入口的场景。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EmailConfig {
    /// IMAP 服务器主机名。
    pub imap_host: String,
    /// IMAP 端口，默认值为 `993`。
    #[serde(default = "default_imap_port")]
    pub imap_port: u16,
    /// IMAP 监听文件夹，默认值为 `INBOX`。
    #[serde(default = "default_imap_folder")]
    pub imap_folder: String,
    /// SMTP 服务器主机名。
    pub smtp_host: String,
    /// SMTP 端口，默认值为 `465`。
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    /// SMTP 是否启用 TLS。
    #[serde(default = "default_true")]
    pub smtp_tls: bool,
    /// 登录用户名。
    pub username: String,
    /// 登录密码。
    pub password: String,
    /// 默认发件地址。
    pub from_address: String,
    /// IMAP idle 超时时间，单位为秒。
    #[serde(default = "default_idle_timeout", alias = "poll_interval_secs")]
    pub idle_timeout_secs: u64,
    /// 允许接收的发件人列表。
    #[serde(default)]
    pub allowed_senders: Vec<String>,
}

fn default_imap_port() -> u16 {
    993
}
fn default_smtp_port() -> u16 {
    465
}
fn default_imap_folder() -> String {
    "INBOX".into()
}
fn default_idle_timeout() -> u64 {
    1740
}
fn default_true() -> bool {
    true
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            imap_host: String::new(),
            imap_port: default_imap_port(),
            imap_folder: default_imap_folder(),
            smtp_host: String::new(),
            smtp_port: default_smtp_port(),
            smtp_tls: default_true(),
            username: String::new(),
            password: String::new(),
            from_address: String::new(),
            idle_timeout_secs: default_idle_timeout(),
            allowed_senders: Vec::new(),
        }
    }
}
#[cfg(test)]
#[path = "email_tests.rs"]
mod email_tests;
