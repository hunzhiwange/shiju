//! Offline Pi provider presets and their reviewed base capabilities.

use super::{
    pi_thinking::{self, FORCE_ADAPTIVE_THINKING, PiThinkingLevelMap, ResolvedPiThinkingProfile},
    preset_search::{PresetPlan, PresetRegion},
};
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
    pub compat: Option<BTreeMap<String, PiPresetCompatValue>>,
    pub models: Vec<PiPresetModel>,
}

impl PiPresetConfig {
    pub(super) fn apply_thinking_profiles(&mut self) {
        let adaptive_compat =
            self.compat.as_ref().and_then(|compat| compat.get(FORCE_ADAPTIVE_THINKING));
        for model in &mut self.models {
            model.apply_thinking_profile(self.api, adaptive_compat);
        }
    }

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
#[serde(rename_all = "camelCase")]
pub struct PiPresetModel {
    pub id: String,
    #[serde(flatten)]
    pub capabilities: PiModelCapabilities,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compat: Option<BTreeMap<String, PiPresetCompatValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_level_map: Option<PiThinkingLevelMap>,
    #[serde(skip_serializing)]
    thinking_reference: Option<PiThinkingReference>,
}

impl PiPresetModel {
    /// Copy the reviewed mapping so callers can edit their own draft.
    pub fn thinking_level_map(&self) -> Option<PiThinkingLevelMap> {
        self.thinking_level_map.clone()
    }

    fn apply_thinking_profile(
        &mut self,
        api: PiPresetProtocol,
        provider_adaptive_compat: Option<&PiPresetCompatValue>,
    ) {
        let Some(reference) = self.thinking_reference.take() else {
            return;
        };
        if !self.capabilities.reasoning {
            return;
        }
        // Local profiles belong to this preset's deployment and take precedence.
        let resolved = match reference.preset_thinking_profile_id {
            Some(profile_id) => Some(ResolvedPiThinkingProfile {
                map: pi_thinking::profile_map(&profile_id),
                force_adaptive_thinking: false,
            }),
            None => {
                let adaptive_compat = self
                    .compat
                    .as_ref()
                    .and_then(|compat| compat.get(FORCE_ADAPTIVE_THINKING))
                    .or(provider_adaptive_compat);
                pi_thinking::resolve(&reference.catalog_key, api, adaptive_compat)
            }
        };
        if let Some(profile) = resolved {
            self.thinking_level_map = Some(profile.map);
            if profile.force_adaptive_thinking {
                self.compat
                    .get_or_insert_with(BTreeMap::new)
                    .insert(FORCE_ADAPTIVE_THINKING.into(), PiPresetCompatValue::Flag(true));
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PiThinkingReference {
    catalog_key: String,
    preset_thinking_profile_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PiPresetCompatValue {
    Flag(bool),
    Dialect(String),
}
