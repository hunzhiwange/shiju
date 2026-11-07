//! Read-only Gemini and Grok Build model defaults and deployment hints.

use super::preset_search::{PresetPlan, PresetRegion};
use serde::Deserialize;
use vw_api_types::{agent_apps::AppId, app_providers::ProviderProtocol};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeminiGrokPresetCategory {
    Official,
    ThirdParty,
    Aggregator,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeminiGrokPreset {
    pub app_id: AppId,
    pub name: String,
    pub name_key: Option<String>,
    pub website_url: String,
    pub description: Option<String>,
    pub category: Option<GeminiGrokPresetCategory>,
    pub family: Option<String>,
    pub plan_key: Option<PresetPlan>,
    pub region_key: Option<PresetRegion>,
    pub icon: Option<String>,
    pub icon_color: Option<String>,
    pub official_provider_id: Option<String>,
    /// Source identifier used to recognize Google's OAuth preset.
    pub official_auth_key: Option<String>,
    /// Official accounts have no custom deployment protocol, URL or model.
    pub protocol: Option<ProviderProtocol>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub endpoint_candidates: Vec<String>,
    /// Explicit Gemini API key auth override from the source preset.
    #[serde(default)]
    pub bearer_api_key: bool,
}
