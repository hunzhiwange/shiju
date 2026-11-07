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
    #[serde(
        default,
        deserialize_with = "crate::app_providers::validation::present",
        skip_serializing_if = "Option::is_none"
    )]
    pub request_options: Option<crate::app_providers::validation::ProviderRequestOptions>,
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

pub mod endpoints;
#[cfg(test)]
mod endpoints_tests;

pub mod icons;
#[cfg(test)]
mod icons_tests;

pub mod universal;
#[cfg(test)]
mod universal_tests;

pub mod universal_sync;
#[cfg(test)]
mod universal_sync_tests;

pub mod paths;
#[cfg(test)]
mod paths_tests;

pub mod preview;
#[cfg(test)]
mod preview_tests;

pub mod validation;
#[cfg(test)]
mod validation_tests;

pub mod editor_merge;
#[cfg(test)]
mod editor_merge_tests;

pub mod snippets;
#[cfg(test)]
mod snippets_tests;

pub mod claude;
pub mod claude_login;
#[cfg(test)]
mod claude_login_tests;
#[cfg(test)]
mod claude_tests;

#[cfg(test)]
mod codex_tests;

pub mod codex_catalog;
#[cfg(test)]
mod codex_catalog_tests;

pub mod gemini;
#[cfg(test)]
mod gemini_tests;

pub mod claude_desktop;
pub mod grok;
#[cfg(test)]
mod grok_tests;

#[cfg(test)]
mod claude_desktop_tests;

pub mod desktop_mapping;
#[cfg(test)]
mod desktop_mapping_tests;

pub mod hermes;
#[cfg(test)]
mod hermes_tests;
pub mod openclaw;
#[cfg(test)]
mod openclaw_tests;
pub mod opencode;
#[cfg(test)]
mod opencode_tests;

pub mod pi;
#[cfg(test)]
mod pi_tests;

pub mod omp;
#[cfg(test)]
mod omp_tests;

pub mod mcode;
#[cfg(test)]
mod mcode_tests;

pub mod kimi;
#[cfg(test)]
mod kimi_tests;

pub mod dsh;
#[cfg(test)]
mod dsh_tests;

pub mod agy;
#[cfg(test)]
mod agy_tests;

pub mod import;
#[cfg(test)]
mod import_tests;

pub mod mode_state;
#[cfg(test)]
mod mode_state_tests;

pub mod mode_apply;
#[cfg(test)]
mod mode_apply_tests;

pub mod env_diagnose;
#[cfg(test)]
mod env_diagnose_tests;

pub mod env_clean;
#[cfg(test)]
mod env_clean_tests;

pub mod env_restore;
#[cfg(test)]
mod env_restore_tests;

pub mod operations;
#[cfg(test)]
mod operations_tests;

pub mod basic;

pub mod request_options;
