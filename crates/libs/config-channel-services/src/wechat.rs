//! 微信 iLink 机器人通道配置。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const DEFAULT_WECHAT_BASE_URL: &str = "https://ilinkai.weixin.qq.com";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WeChatConfig {
    pub bot_token: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

/// 命名微信通道实例。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WeChatInstanceConfig {
    pub id: String,
    #[serde(flatten)]
    pub config: WeChatConfig,
}

impl WeChatConfig {
    pub fn has_required_credentials(&self) -> bool {
        !self.bot_token.trim().is_empty() && self.has_trusted_base_url()
    }

    pub fn has_trusted_base_url(&self) -> bool {
        self.base_url.trim_end_matches('/') == DEFAULT_WECHAT_BASE_URL
    }
}

fn default_base_url() -> String {
    DEFAULT_WECHAT_BASE_URL.to_string()
}

#[cfg(test)]
#[path = "wechat_tests.rs"]
mod tests;
