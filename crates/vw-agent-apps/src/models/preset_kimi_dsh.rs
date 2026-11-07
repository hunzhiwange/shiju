//! Read-only Kimi and DSH presets with their native model keys and fields.

use super::preset_search::{PresetPlan, PresetRegion};
use serde::Deserialize;
use std::collections::BTreeMap;
use vw_api_types::agent_apps::AppId;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KimiDshPreset {
    pub name: String,
    pub name_key: Option<String>,
    pub provider_key: String,
    pub website_url: String,
    pub category: String,
    pub family: Option<String>,
    pub plan_key: Option<PresetPlan>,
    pub region_key: Option<PresetRegion>,
    pub icon: String,
    pub icon_color: Option<String>,
    pub catalog: KimiDshCatalog,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "appId", content = "settingsConfig", rename_all = "snake_case")]
pub enum KimiDshCatalog {
    Kimi(KimiPresetCatalog),
    Dsh(DshPresetCatalog),
}

impl KimiDshPreset {
    pub fn app_id(&self) -> AppId {
        match &self.catalog {
            KimiDshCatalog::Kimi(_) => AppId::Kimi,
            KimiDshCatalog::Dsh(_) => AppId::Dsh,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KimiPresetCatalog {
    pub provider: KimiPresetProvider,
    pub models: BTreeMap<String, KimiPresetModel>,
}

impl KimiPresetCatalog {
    /// The alias includes the source provider key and is scoped to its exact endpoint.
    pub fn model_at_endpoint(&self, endpoint: &str, alias: &str) -> Option<&KimiPresetModel> {
        if self.provider.base_url != endpoint {
            return None;
        }
        self.models.get(alias)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KimiPresetProvider {
    #[serde(rename = "type")]
    pub provider_type: KimiPresetProtocol,
    pub base_url: String,
    pub custom_headers: Option<BTreeMap<String, String>>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KimiPresetProtocol {
    Kimi,
    Openai,
    OpenaiResponses,
    Anthropic,
    #[serde(rename = "google-genai")]
    GoogleGenai,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KimiPresetModel {
    pub model: String,
    pub display_name: String,
    pub max_context_size: u64,
    pub capabilities: Vec<String>,
    pub max_output_size: Option<u64>,
    pub adaptive_thinking: Option<bool>,
    pub reasoning_key: Option<String>,
    pub support_efforts: Option<Vec<String>>,
    pub default_effort: Option<String>,
    pub off_effort: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DshPresetCatalog {
    pub api: DshPresetProtocol,
    pub compat: Option<BTreeMap<String, DshCompatValue>>,
    pub options: DshPresetOptions,
    pub models: BTreeMap<String, DshPresetModel>,
}

impl DshPresetCatalog {
    /// DSH uses the original model ID within the selected preset's exact endpoint.
    pub fn model_at_endpoint(&self, endpoint: &str, model_id: &str) -> Option<&DshPresetModel> {
        if self.options.base_url != endpoint {
            return None;
        }
        self.models.get(model_id)
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DshPresetProtocol {
    OpenaiCompletions,
    OpenaiResponses,
    AnthropicMessages,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DshPresetOptions {
    #[serde(rename = "baseURL")]
    pub base_url: String,
    pub headers: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DshPresetModel {
    pub name: String,
    pub modalities: DshPresetModalities,
    pub limit: DshPresetLimit,
    pub compat: Option<BTreeMap<String, DshCompatValue>>,
    pub reasoning: Option<bool>,
    pub reasoning_efforts: DshReasoningEfforts,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DshPresetModalities {
    pub input: Vec<String>,
    pub output: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DshPresetLimit {
    pub context: u64,
    pub output: u64,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum DshCompatValue {
    Flag(bool),
    Dialect(String),
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum DshReasoningEfforts {
    Flag(bool),
    /// A null wire value means the off level exists without an explicit wire value.
    Levels(BTreeMap<String, Option<String>>),
}
