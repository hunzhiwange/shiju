//! Read-only model presets scoped to a selected Hermes or Mcode deployment.

use super::preset_search::{PresetPlan, PresetRegion};
use serde::Deserialize;
use std::collections::BTreeMap;
use vw_api_types::agent_apps::AppId;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HermesMcodePreset {
    pub app_id: AppId,
    pub name: String,
    pub name_key: Option<String>,
    pub website_url: String,
    pub category: String,
    pub family: Option<String>,
    pub plan_key: Option<PresetPlan>,
    pub region_key: Option<PresetRegion>,
    pub icon: String,
    pub icon_color: Option<String>,
    #[serde(default)]
    pub is_official: bool,
    /// The source's explicit provider key, when declared.
    pub provider_key: Option<String>,
    /// Hermes' provider name or Mcode's native config name.
    pub config_name: String,
    pub base_url: String,
    /// Native Hermes API mode or Mcode API format.
    pub api: String,
    pub models: Vec<HermesMcodePresetModel>,
    pub suggested_model: Option<HermesPresetDefault>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HermesMcodePresetModel {
    pub id: String,
    pub name: String,
    /// Hermes context_length or Mcode limit.context.
    pub context_window: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub input_modalities: Option<Vec<String>>,
    pub output_modalities: Option<Vec<String>>,
    /// Hermes' per-model declaration. Absence leaves vision unspecified.
    pub supports_vision: Option<bool>,
    pub reasoning: Option<bool>,
    pub compat: Option<BTreeMap<String, McodeCompatValue>>,
    pub effort_options: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum McodeCompatValue {
    Flag(bool),
    Dialect(String),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HermesPresetDefault {
    pub model_id: String,
    pub provider: Option<String>,
}

impl HermesMcodePreset {
    /// Look up an exact model ID at this selected preset's declared base URL.
    pub fn model_at_endpoint(
        &self,
        endpoint: &str,
        model_id: &str,
    ) -> Option<&HermesMcodePresetModel> {
        if self.base_url != endpoint {
            return None;
        }
        self.models.iter().find(|model| model.id == model_id)
    }
}
