//! Saved universal records only; application projection is a separate operation.
use super::create::ProviderSecretWrite;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniversalApps {
    #[serde(default)]
    pub claude: bool,
    #[serde(default)]
    pub codex: bool,
    #[serde(default)]
    pub gemini: bool,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeModels {
    pub model: Option<String>,
    pub haiku_model: Option<String>,
    pub sonnet_model: Option<String>,
    pub opus_model: Option<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexModels {
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeminiModels {
    pub model: Option<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniversalModels {
    pub claude: Option<ClaudeModels>,
    pub codex: Option<CodexModels>,
    pub gemini: Option<GeminiModels>,
}
/// Full replacement of editable fields. Opaque stored metadata is retained internally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UniversalConfig {
    pub name: String,
    pub provider_type: String,
    pub apps: UniversalApps,
    pub base_url: String,
    #[serde(default)]
    pub models: UniversalModels,
    pub website_url: Option<String>,
    pub notes: Option<String>,
    pub icon: Option<String>,
    pub icon_color: Option<String>,
    pub sort_index: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UniversalProviderWrite {
    pub config: UniversalConfig,
    pub api_key: ProviderSecretWrite,
}
/// No secret or arbitrary metadata is returned, including from historical records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UniversalProviderSummary {
    pub id: String,
    pub config: UniversalConfig,
    pub configured: bool,
}
