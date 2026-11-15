//! OMP's native projection of the offline Pi provider presets.

use super::{
    preset_pi::{
        PiModelCapabilities, PiPreset, PiPresetCompatValue, PiPresetModel, PiPresetProtocol,
    },
    preset_search::{PresetPlan, PresetRegion},
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct OmpPreset {
    pub name: &'static str,
    pub name_key: Option<&'static str>,
    pub provider_key: &'static str,
    pub website_url: &'static str,
    pub api_key_url: Option<&'static str>,
    pub category: &'static str,
    pub family: Option<&'static str>,
    pub plan_key: Option<PresetPlan>,
    pub region_key: Option<PresetRegion>,
    pub icon: Option<&'static str>,
    pub icon_color: Option<&'static str>,
    pub settings_config: OmpPresetConfig,
}

impl OmpPreset {
    pub(super) fn from_pi(preset: &'static PiPreset) -> Self {
        let config = &preset.settings_config;
        Self {
            name: &preset.name,
            name_key: preset.name_key.as_deref(),
            provider_key: &preset.provider_key,
            website_url: &preset.website_url,
            api_key_url: preset.api_key_url.as_deref(),
            category: &preset.category,
            family: preset.family.as_deref(),
            plan_key: preset.plan_key,
            region_key: preset.region_key,
            icon: preset.icon.as_deref(),
            icon_color: preset.icon_color.as_deref(),
            settings_config: OmpPresetConfig {
                base_url: &config.base_url,
                api: config.api,
                headers: config.headers.as_ref(),
                compat: config.compat.as_ref().map(project_compat),
                models: config.models.iter().map(OmpPresetModel::from_pi).collect(),
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OmpPresetConfig {
    pub base_url: &'static str,
    pub api: PiPresetProtocol,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<&'static BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compat: Option<BTreeMap<String, PiPresetCompatValue>>,
    pub models: Vec<OmpPresetModel>,
}

impl OmpPresetConfig {
    /// The native model ID is scoped to the preset's exact endpoint and wire protocol.
    pub fn model_at_deployment(
        &self,
        endpoint: &str,
        protocol: PiPresetProtocol,
        model_id: &str,
    ) -> Option<&OmpPresetModel> {
        if self.base_url != endpoint || self.api != protocol {
            return None;
        }
        self.models.iter().find(|model| model.id == model_id)
    }
}

#[derive(Debug, Serialize)]
pub struct OmpPresetModel {
    pub id: &'static str,
    #[serde(flatten)]
    pub capabilities: &'static PiModelCapabilities,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compat: Option<BTreeMap<String, PiPresetCompatValue>>,
}

impl OmpPresetModel {
    fn from_pi(model: &'static PiPresetModel) -> Self {
        Self {
            id: &model.id,
            capabilities: &model.capabilities,
            compat: model.compat.as_ref().map(project_compat).filter(|compat| !compat.is_empty()),
        }
    }
}

fn project_compat(
    compat: &BTreeMap<String, PiPresetCompatValue>,
) -> BTreeMap<String, PiPresetCompatValue> {
    use PiPresetCompatValue::{Dialect, Flag};

    let mut projected = compat.clone();
    projected.remove("forceAdaptiveThinking");
    projected.remove("deferredToolsMode");
    if let Some(Dialect(format)) = projected.get_mut("thinkingFormat")
        && format == "deepseek"
    {
        *format = "zai".into();
    }
    if projected.remove("requiresReasoningContentOnAssistantMessages") == Some(Flag(true)) {
        projected.insert("reasoningContentField".into(), Dialect("reasoning_content".into()));
        projected.insert("requiresReasoningContentForToolCalls".into(), Flag(true));
        projected.insert("requiresReasoningContentForAllAssistantTurns".into(), Flag(true));
        projected.insert("allowsSyntheticReasoningContentForToolCalls".into(), Flag(false));
    }
    projected
}
