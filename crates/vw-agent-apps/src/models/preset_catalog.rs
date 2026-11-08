//! Built-in provider presets for the native model catalog.

use super::preset_pi::{PiModelCapabilities, PiPreset};
use super::{
    preset_claude::ClaudePreset, preset_codex::CodexPreset, preset_gemini_grok::GeminiGrokPreset,
    preset_hermes_mcode::HermesMcodePreset, preset_kimi_dsh::KimiDshPreset,
    preset_opencode_openclaw::OpenCodeOpenClawPreset,
};
use crate::providers::{universal_sync::projected_endpoint, validation::endpoint};
use serde::Deserialize;
use std::{collections::BTreeMap, sync::LazyLock};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        ProviderProtocol,
        universal::{UniversalApps, UniversalModels},
        universal_sync::UniversalSyncError,
    },
};

static CLAUDE_PRESETS: LazyLock<Vec<ClaudePreset>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("presets/claude.json"))
        .expect("valid built-in Claude presets")
});

static CODEX_PRESETS: LazyLock<Vec<CodexPreset>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("presets/codex.json")).expect("valid built-in Codex presets")
});

static GEMINI_GROK_PRESETS: LazyLock<Vec<GeminiGrokPreset>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("presets/gemini_grok.json"))
        .expect("valid built-in Gemini and Grok Build presets")
});

static HERMES_MCODE_PRESETS: LazyLock<Vec<HermesMcodePreset>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("presets/hermes_mcode.json"))
        .expect("valid built-in Hermes and Mcode presets")
});

static KIMI_DSH_PRESETS: LazyLock<Vec<KimiDshPreset>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("presets/kimi_dsh.json"))
        .expect("valid built-in Kimi and DSH presets")
});

static OPENCODE_OPENCLAW_PRESETS: LazyLock<Vec<OpenCodeOpenClawPreset>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("presets/opencode_openclaw.json"))
        .expect("valid built-in OpenCode and OpenClaw presets")
});

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PiPresets {
    base_models: BTreeMap<String, PiModelCapabilities>,
    presets: Vec<PiPreset>,
}

static PI_PRESETS: LazyLock<PiPresets> = LazyLock::new(|| {
    serde_json::from_str(include_str!("presets/pi.json")).expect("valid built-in Pi presets")
});

static UNIVERSAL_PRESETS: LazyLock<Vec<UniversalPreset>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("presets/universal.json"))
        .expect("valid built-in universal presets")
});

/// Defaults shared by the enabled applications. The reasoning effort is a
/// configuration default and capabilities of the deployed models remain unknown.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UniversalPreset {
    pub provider_type: String,
    pub name: String,
    pub default_apps: UniversalApps,
    pub default_models: UniversalModels,
    pub website_url: Option<String>,
    pub icon: Option<String>,
    pub icon_color: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub is_custom_template: bool,
}

impl UniversalPreset {
    /// The deployment supplies its API base URL separately from the display website.
    pub fn endpoint(
        &self,
        app: AppId,
        base_url: &str,
    ) -> Result<(ProviderProtocol, String), UniversalSyncError> {
        use UniversalSyncError::*;
        let enabled = match app {
            AppId::Claude => self.default_apps.claude,
            AppId::Codex => self.default_apps.codex,
            AppId::Gemini => self.default_apps.gemini,
            _ => return Err(UnsupportedApp),
        };
        if !enabled {
            return Err(AppDisabled);
        }
        endpoint(base_url).map_err(|_| InvalidProfile)?;
        let (protocol, projected_url) = projected_endpoint(app, base_url)?;
        endpoint(&projected_url).map_err(|_| InvalidProfile)?;
        Ok((protocol, projected_url))
    }
}

pub fn universal_presets() -> &'static [UniversalPreset] {
    &UNIVERSAL_PRESETS
}

pub fn universal_preset(provider_type: &str) -> Option<&'static UniversalPreset> {
    universal_presets().iter().find(|preset| preset.provider_type == provider_type)
}

/// List one application's presets in source order, including hidden rows.
pub fn claude_presets(app: AppId) -> impl Iterator<Item = &'static ClaudePreset> {
    CLAUDE_PRESETS.iter().filter(move |preset| preset.app_id() == app)
}

/// Find a preset by its exact source name within one application.
pub fn claude_preset(app: AppId, name: &str) -> Option<&'static ClaudePreset> {
    claude_presets(app).find(|preset| preset.provider().name == name)
}

/// List one application's presets in source order, with the official preset first.
pub fn gemini_grok_presets(app: AppId) -> impl Iterator<Item = &'static GeminiGrokPreset> {
    GEMINI_GROK_PRESETS.iter().filter(move |preset| preset.app_id == app)
}

/// Find a preset by its exact source name within one application.
pub fn gemini_grok_preset(app: AppId, name: &str) -> Option<&'static GeminiGrokPreset> {
    gemini_grok_presets(app).find(|preset| preset.name == name)
}

/// List Codex presets in source order, including hidden rows.
pub fn codex_presets(app: AppId) -> impl Iterator<Item = &'static CodexPreset> {
    let presets: &[CodexPreset] = match app {
        AppId::Codex => &CODEX_PRESETS,
        _ => &[],
    };
    presets.iter()
}

/// Find a preset by its exact source name within Codex.
pub fn codex_preset(app: AppId, name: &str) -> Option<&'static CodexPreset> {
    codex_presets(app).find(|preset| preset.name == name)
}

/// List OpenCode or OpenClaw presets in source order.
pub fn opencode_openclaw_presets(
    app: AppId,
) -> impl Iterator<Item = &'static OpenCodeOpenClawPreset> {
    OPENCODE_OPENCLAW_PRESETS.iter().filter(move |preset| preset.app_id == app)
}

/// Find a preset by its exact source name within one application.
pub fn opencode_openclaw_preset(app: AppId, name: &str) -> Option<&'static OpenCodeOpenClawPreset> {
    opencode_openclaw_presets(app).find(|preset| preset.name == name)
}

/// List Hermes or Mcode presets in source order.
pub fn hermes_mcode_presets(app: AppId) -> impl Iterator<Item = &'static HermesMcodePreset> {
    HERMES_MCODE_PRESETS.iter().filter(move |preset| preset.app_id == app)
}

/// Find a preset by its exact source name within one application.
pub fn hermes_mcode_preset(app: AppId, name: &str) -> Option<&'static HermesMcodePreset> {
    hermes_mcode_presets(app).find(|preset| preset.name == name)
}

/// List one application's presets in source order, with the official account first.
pub fn kimi_dsh_presets(app: AppId) -> impl Iterator<Item = &'static KimiDshPreset> {
    KIMI_DSH_PRESETS.iter().filter(move |preset| preset.app_id() == app)
}

/// Find a preset by its exact source key within one application.
pub fn kimi_dsh_preset(app: AppId, provider_key: &str) -> Option<&'static KimiDshPreset> {
    kimi_dsh_presets(app).find(|preset| preset.provider_key == provider_key)
}

/// List Pi presets in source order.
pub fn pi_presets(app: AppId) -> impl Iterator<Item = &'static PiPreset> {
    let presets: &[PiPreset] = match app {
        AppId::Pi => &PI_PRESETS.presets,
        _ => &[],
    };
    presets.iter()
}

/// Find a preset by its exact managed provider key within Pi.
pub fn pi_preset(app: AppId, provider_key: &str) -> Option<&'static PiPreset> {
    pi_presets(app).find(|preset| preset.provider_key == provider_key)
}

/// Look up reviewed base capabilities by source catalog key.
pub fn pi_model_capabilities(catalog_key: &str) -> Option<&'static PiModelCapabilities> {
    PI_PRESETS.base_models.get(catalog_key)
}
