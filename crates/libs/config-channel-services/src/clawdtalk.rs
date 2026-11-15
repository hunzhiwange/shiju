//! ClawdTalk 通道配置模块。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ClawdTalk 通道配置。
///
/// 用于短信或类似目的地投递场景，包含鉴权、连接标识、发送号码和目的地限制。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClawdTalkConfig {
    /// API Key。
    pub api_key: String,
    /// 连接 ID。
    pub connection_id: String,
    /// 发送方号码。
    pub from_number: String,
    /// 允许投递的目标列表。
    #[serde(default)]
    pub allowed_destinations: Vec<String>,
    /// Webhook 校验密钥。
    #[serde(default)]
    pub webhook_secret: Option<String>,
}
#[cfg(test)]
#[path = "clawdtalk_tests.rs"]
mod clawdtalk_tests;
