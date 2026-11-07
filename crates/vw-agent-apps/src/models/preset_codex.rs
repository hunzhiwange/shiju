//! Read-only Codex model defaults and deployment metadata.

use super::preset_search::{PresetPlan, PresetRegion};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexPresetCategory {
    Official,
    CnOfficial,
    ThirdParty,
    Aggregator,
    CloudProvider,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexPresetApiFormat {
    OpenaiResponses,
    OpenaiChat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexPresetAuthProvider {
    CodexOauth,
    GithubCopilot,
    XaiOauth,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexPreset {
    pub name: String,
    pub name_key: Option<String>,
    pub website_url: String,
    pub category: Option<CodexPresetCategory>,
    pub family: Option<String>,
    pub plan_key: Option<PresetPlan>,
    pub region_key: Option<PresetRegion>,
    pub icon: Option<String>,
    pub icon_color: Option<String>,
    #[serde(default)]
    pub is_official: bool,
    #[serde(default)]
    pub hidden: bool,
    pub auth_provider: Option<CodexPresetAuthProvider>,
    #[serde(default, rename = "requiresOAuth")]
    pub requires_oauth: bool,
    /// The explicit Codex TOML fallback flag, independent of the account source.
    pub requires_openai_auth: Option<bool>,
    /// Upstream format override or the format declared by the Codex config.
    pub api_format: Option<CodexPresetApiFormat>,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
    pub default_reasoning_effort: Option<String>,
    #[serde(default)]
    pub endpoint_candidates: Vec<String>,
    #[serde(default)]
    pub model_catalog: Vec<CodexPresetModel>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexPresetModel {
    pub model: String,
    pub display_name: Option<String>,
    pub context_window: Option<u64>,
    pub reasoning_levels: Option<Vec<String>>,
    pub default_reasoning_level: Option<String>,
    /// An omitted list leaves the input capabilities unspecified.
    pub input_modalities: Option<Vec<String>>,
    pub supports_parallel_tool_calls: Option<bool>,
    pub base_instructions: Option<String>,
}

impl CodexPreset {
    /// Look up an exact model ID on a declared endpoint of this selected preset.
    pub fn model_at_endpoint(&self, endpoint: &str, model: &str) -> Option<&CodexPresetModel> {
        if self.base_url.as_deref() != Some(endpoint)
            && !self.endpoint_candidates.iter().any(|candidate| candidate == endpoint)
        {
            return None;
        }
        self.model_catalog.iter().find(|row| row.model == model)
    }
}
