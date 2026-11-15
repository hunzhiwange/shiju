//! 企业微信 AI 机器人通道配置。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 企业微信 AI 机器人长连接凭证。
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct WeComConfig {
    pub bot_id: String,
    pub secret: String,
}

impl WeComConfig {
    pub fn has_required_credentials(&self) -> bool {
        !self.bot_id.trim().is_empty() && !self.secret.trim().is_empty()
    }
}

impl std::fmt::Debug for WeComConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeComConfig")
            .field("bot_id", &self.bot_id)
            .field("secret", &"[REDACTED]")
            .finish()
    }
}

/// 仅用于出站投递的独立默认企业微信群机器人凭据。
/// 完整 URL 都是秘密，持久化前必须加密。
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WeComWebhookConfig {
    pub webhook_url: String,
}

impl std::fmt::Debug for WeComWebhookConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeComWebhookConfig").field("webhook_url", &"[REDACTED]").finish()
    }
}

#[cfg(test)]
#[path = "wecom_tests.rs"]
mod tests;
