//! Native application providers. These identities are separate from model providers.
use vw_api_app_identity::{AppId, LaunchRequest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderProtocol {
    Anthropic,
    OpenaiResponses,
    Gemini,
}

/// Write-only secret input. Never return this type in an API response.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppProviderWrite {
    pub name: String,
    pub protocol: ProviderProtocol,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    #[serde(
        default,
        deserialize_with = "crate::validation::present",
        skip_serializing_if = "Option::is_none"
    )]
    pub request_options: Option<crate::validation::ProviderRequestOptions>,
}
impl std::fmt::Debug for AppProviderWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AppProviderWrite([redacted])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppProviderSummary {
    pub id: String,
    pub app_id: AppId,
    pub name: String,
    pub configured: bool,
}

/// The authenticated tenant supplies ownership; no client-supplied tenant or secrets.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderLaunchRequest {
    pub provider_id: String,
    pub launch: LaunchRequest,
}

pub mod basic;

pub mod claude;
#[cfg(test)]
mod claude_tests;

pub mod claude_login;
#[cfg(test)]
mod claude_login_tests;

pub mod create;
#[cfg(test)]
mod create_tests;

pub mod update;
#[cfg(test)]
mod update_tests;

pub mod validation;
#[cfg(test)]
mod validation_tests;

#[cfg(test)]
mod tests;
