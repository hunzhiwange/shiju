//! Read-only model defaults and deployment hints for the two Claude applications.

use super::preset_search::{PresetPlan, PresetRegion};
use serde::Deserialize;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        desktop_mapping::{DesktopMappingMode, DesktopModelRoute},
        universal::ClaudeModels,
    },
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudePresetProvider {
    pub name: String,
    pub name_key: Option<String>,
    pub website_url: String,
    pub category: ClaudePresetCategory,
    pub family: Option<String>,
    pub plan_key: Option<PresetPlan>,
    pub region_key: Option<PresetRegion>,
    pub icon: Option<String>,
    pub icon_color: Option<String>,
    #[serde(default)]
    pub hidden: bool,
    pub oauth_provider: Option<String>,
    /// Source address or template. A consumer must resolve and validate it before use.
    pub base_url: Option<String>,
    #[serde(default)]
    pub endpoint_candidates: Vec<String>,
    #[serde(default)]
    pub api_format: ClaudePresetApiFormat,
    #[serde(default)]
    pub api_key_header: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudePresetCategory {
    Official,
    CnOfficial,
    ThirdParty,
    Aggregator,
    CloudProvider,
    Local,
}

/// Upstream API format from the preset. It does not imply native runtime support.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudePresetApiFormat {
    #[default]
    Anthropic,
    OpenaiChat,
    OpenaiResponses,
    GeminiNative,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeDesktopPresetRoute {
    pub route_id: String,
    pub target: DesktopModelRoute,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "appId", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum ClaudePreset {
    #[serde(rename = "claude")]
    Code {
        provider: ClaudePresetProvider,
        #[serde(default)]
        models: ClaudeModels,
        small_fast_model: Option<String>,
        subagent_model: Option<String>,
        models_url: Option<String>,
        max_context_tokens: Option<u32>,
        auto_compact_window: Option<u32>,
        max_output_tokens: Option<u32>,
    },
    #[serde(rename = "claude-desktop")]
    Desktop {
        provider: ClaudePresetProvider,
        mode: DesktopMappingMode,
        #[serde(default)]
        routes: Vec<ClaudeDesktopPresetRoute>,
    },
}

impl ClaudePreset {
    pub fn app_id(&self) -> AppId {
        match self {
            Self::Code { .. } => AppId::Claude,
            Self::Desktop { .. } => AppId::ClaudeDesktop,
        }
    }

    pub fn provider(&self) -> &ClaudePresetProvider {
        match self {
            Self::Code { provider, .. } | Self::Desktop { provider, .. } => provider,
        }
    }
}
