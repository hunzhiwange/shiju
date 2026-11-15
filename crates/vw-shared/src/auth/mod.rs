pub mod store;

pub use store::*;

use serde::{Deserialize, Serialize};

/// OAuth 鉴权信息占位键，用于统一处理需要假值的场景。
pub const OAUTH_DUMMY_KEY: &str = "vibewindow-oauth-dummy-key";

/// OAuth 鉴权令牌信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OauthInfo {
    pub refresh: String,
    pub access: String,
    pub expires: i64,
    #[serde(rename = "accountId")]
    pub account_id: Option<String>,
    #[serde(rename = "enterpriseUrl")]
    pub enterprise_url: Option<String>,
}

/// API Key 鉴权信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    pub key: String,
}

/// 已知格式的组合鉴权信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WellKnownInfo {
    pub key: String,
    pub token: String,
}

/// 提供商鉴权信息的统一枚举。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Info {
    Oauth(OauthInfo),
    Api(ApiInfo),
    Wellknown(WellKnownInfo),
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
