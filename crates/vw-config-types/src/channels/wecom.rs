//! 企业微信 AI 机器人通道配置。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 企业微信 AI 机器人长连接凭证。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WeComConfig {
    pub bot_id: String,
    pub secret: String,
}

impl WeComConfig {
    pub fn has_required_credentials(&self) -> bool {
        !self.bot_id.trim().is_empty() && !self.secret.trim().is_empty()
    }
}

#[cfg(test)]
#[path = "wecom_tests.rs"]
mod tests;
