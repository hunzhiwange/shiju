//! Read-only model presets scoped to a selected OpenCode or OpenClaw deployment.

use super::preset_search::{PresetPlan, PresetRegion};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use vw_api_types::{agent_apps::AppId, model_catalog::ModelTokenCost};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenCodeOpenClawPreset {
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
    pub is_custom_template: bool,
    /// OpenCode's native adapter package, when declared.
    pub npm: Option<String>,
    /// OpenClaw's native API protocol, when declared.
    pub api: Option<String>,
    pub base_url: Option<String>,
    pub models: Vec<OpenCodeOpenClawPresetModel>,
    pub suggested_defaults: Option<OpenClawPresetDefaults>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenCodeOpenClawPresetModel {
    pub id: String,
    pub name: String,
    /// Missing values are left unspecified by the selected preset.
    pub context_window: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub input_modalities: Option<Vec<String>>,
    pub output_modalities: Option<Vec<String>>,
    pub reasoning: Option<bool>,
    /// Reviewed per-deployment prices, when declared by the OpenClaw preset.
    pub cost: Option<ModelTokenCost>,
    /// Native OpenCode model options preserved as source data.
    pub options: Option<Value>,
    /// Native OpenCode variant settings keyed by their source names.
    pub variants: Option<Value>,
    pub compat: Option<OpenClawPresetModelCompat>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenClawPresetModelCompat {
    pub max_tokens_field: Option<String>,
    pub supports_tools: Option<bool>,
    pub tool_schema_profile: Option<String>,
    pub supports_reasoning_effort: Option<bool>,
    pub supported_reasoning_efforts: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenClawPresetDefaults {
    pub model: Option<OpenClawPresetModelRefs>,
    #[serde(default)]
    pub model_catalog: BTreeMap<String, OpenClawPresetAlias>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenClawPresetModelRefs {
    pub primary: String,
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenClawPresetAlias {
    pub alias: Option<String>,
}

impl OpenCodeOpenClawPreset {
    /// Look up an exact model ID at this selected preset's declared base URL.
    pub fn model_at_endpoint(
        &self,
        endpoint: &str,
        model_id: &str,
    ) -> Option<&OpenCodeOpenClawPresetModel> {
        if self.base_url.as_deref() != Some(endpoint) {
            return None;
        }
        self.models.iter().find(|model| model.id == model_id)
    }
}
