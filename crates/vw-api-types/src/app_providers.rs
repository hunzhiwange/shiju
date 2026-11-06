//! Native application providers. These identities are separate from model providers.
use crate::agent_apps::{AppId, LaunchRequest};
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

pub mod catalog;
#[cfg(test)]
mod catalog_tests;

pub mod create;
#[cfg(test)]
mod create_tests;

pub mod update;
#[cfg(test)]
mod update_tests;

pub mod delete;
#[cfg(test)]
mod delete_tests;

pub mod duplicate;
#[cfg(test)]
mod duplicate_tests;

pub mod presentation;
#[cfg(test)]
mod presentation_tests;
