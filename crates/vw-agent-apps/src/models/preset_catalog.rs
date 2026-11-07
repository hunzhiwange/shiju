//! Built-in provider presets for the native model catalog.

use super::{preset_claude::ClaudePreset, preset_gemini_grok::GeminiGrokPreset};
use crate::providers::{universal_sync::projected_endpoint, validation::endpoint};
use serde::Deserialize;
use std::sync::LazyLock;
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

static GEMINI_GROK_PRESETS: LazyLock<Vec<GeminiGrokPreset>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("presets/gemini_grok.json"))
        .expect("valid built-in Gemini and Grok Build presets")
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
