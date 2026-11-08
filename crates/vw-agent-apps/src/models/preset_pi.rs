//! Offline Pi provider presets and their reviewed base capabilities.

use super::preset_search::{PresetPlan, PresetRegion};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiPreset {
    pub name: String,
    pub name_key: Option<String>,
    pub provider_key: String,
    pub website_url: String,
    pub api_key_url: Option<String>,
    pub category: String,
    pub family: Option<String>,
    pub plan_key: Option<PresetPlan>,
    pub region_key: Option<PresetRegion>,
    pub icon: Option<String>,
    pub icon_color: Option<String>,
    pub settings_config: PiPresetConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiPresetConfig {
    pub name: String,
    pub base_url: String,
    pub api: PiPresetProtocol,
    pub headers: Option<BTreeMap<String, String>>,
    pub models: Vec<PiPresetModel>,
}

impl PiPresetConfig {
    /// The native model ID is scoped to the preset's exact endpoint and wire protocol.
    pub fn model_at_deployment(
        &self,
        endpoint: &str,
        protocol: PiPresetProtocol,
        model_id: &str,
    ) -> Option<&PiPresetModel> {
        if self.base_url != endpoint || self.api != protocol {
            return None;
        }
        self.models.iter().find(|model| model.id == model_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PiPresetProtocol {
    OpenaiCompletions,
    OpenaiResponses,
    AnthropicMessages,
    GoogleGenerativeAi,
    BedrockConverseStream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiModelInput {
    Text,
    Image,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PiModelCapabilities {
    pub name: String,
    pub reasoning: bool,
    pub input: Vec<PiModelInput>,
    pub context_window: u64,
    pub max_tokens: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PiPresetModel {
    pub id: String,
    #[serde(flatten)]
    pub capabilities: PiModelCapabilities,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compat: Option<BTreeMap<String, PiPresetCompatValue>>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PiPresetCompatValue {
    Flag(bool),
    Dialect(String),
}
