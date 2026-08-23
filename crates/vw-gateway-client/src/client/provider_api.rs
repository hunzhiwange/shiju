use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use vw_shared::provider::types::Info;

use super::GatewayClient;
use crate::http::directory_query;

/// Provider 列表接口的聚合响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderListResponse {
    /// 当前可见的全部 provider 信息。
    pub all: Vec<Info>,
    /// 各能力对应的默认 provider 映射。
    pub default: HashMap<String, String>,
    /// 已完成连接校验的 provider 标识列表。
    pub connected: Vec<String>,
}

/// Provider 支持的认证方式。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ProviderAuthMethodDto {
    /// OAuth 浏览器或设备授权。
    Oauth { label: String },
    /// API 密钥认证。
    Api { label: String },
}

impl ProviderAuthMethodDto {
    /// 返回适合用户展示的认证方式名称。
    pub fn label(&self) -> &str {
        match self {
            Self::Oauth { label } | Self::Api { label } => label,
        }
    }
}

/// Provider ID 到认证方式列表的映射。
pub type ProviderAuthMethodsResponse = HashMap<String, Vec<ProviderAuthMethodDto>>;

/// OAuth 授权启动信息。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderOAuthAuthorizationDto {
    /// 用户应打开的授权 URL。
    pub url: String,
    /// 授权方式标识。
    pub method: String,
    /// 授权流程说明。
    pub instructions: String,
}

#[derive(Serialize)]
struct ProviderOAuthAuthorizeRequest {
    method: usize,
}

#[derive(Serialize)]
struct ProviderOAuthCallbackRequest<'a> {
    method: usize,
    code: &'a str,
}

impl GatewayClient {
    /// 获取指定目录上下文下可用的 provider 列表。
    pub async fn provider_list(
        &self,
        directory: Option<&str>,
    ) -> Result<ProviderListResponse, String> {
        self.get_json("/v1/provider", &directory_query(directory)).await
    }

    /// 获取全部 provider 的可用认证方式。
    pub async fn provider_auth_methods(&self) -> Result<ProviderAuthMethodsResponse, String> {
        self.get_json("/v1/provider/auth", &[]).await
    }

    /// 启动指定 provider 的 OAuth 授权流程。
    pub async fn provider_oauth_authorize(
        &self,
        provider_id: &str,
        method: usize,
    ) -> Result<Option<ProviderOAuthAuthorizationDto>, String> {
        let provider_id = required_value("provider id", provider_id)?;
        self.post_json(
            &format!("/v1/provider/{}/oauth/authorize", encode_path_segment(provider_id)),
            &[],
            &ProviderOAuthAuthorizeRequest { method },
        )
        .await
    }

    /// 完成指定 provider 的 OAuth 回调。
    pub async fn provider_oauth_callback(
        &self,
        provider_id: &str,
        method: usize,
        code: &str,
    ) -> Result<bool, String> {
        let provider_id = required_value("provider id", provider_id)?;
        let code = required_value("OAuth callback code", code)?;
        self.post_json(
            &format!("/v1/provider/{}/oauth/callback", encode_path_segment(provider_id)),
            &[],
            &ProviderOAuthCallbackRequest { method, code },
        )
        .await
    }
}

fn required_value<'a>(name: &str, value: &'a str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    Ok(value)
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
#[path = "provider_api_tests.rs"]
mod provider_api_tests;
