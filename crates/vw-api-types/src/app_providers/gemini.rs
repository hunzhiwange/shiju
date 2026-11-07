//! Gemini configuration uses explicit authentication; no client paths or login payloads.
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeminiConfig {
    Official {},
    ThirdParty { base_url: String, api_key: String },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeminiProfile {
    pub name: String,
    pub config: GeminiConfig,
    #[serde(default)]
    pub model: Option<String>,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeminiActivation {
    #[default]
    CatalogOnly,
    First,
    IfCurrent,
    Switch,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeminiWrite {
    pub target_host_id: String,
    pub profile: GeminiProfile,
    #[serde(default)]
    pub activation: GeminiActivation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeminiTarget {
    pub target_host_id: String,
}
pub use super::claude::{ClaudeLiveResult as GeminiLiveResult, ClaudeResult as GeminiResult};
macro_rules! concealed {
    ($($t:ty),*) => {$(impl std::fmt::Debug for $t {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("GeminiConfig([redacted])")
        }
    })*};
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum GeminiFormKind {
    Official {},
    ThirdParty { base_url: String },
    Unsupported {},
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeminiFormView {
    pub fields: super::basic::ProviderBasic,
    pub config: GeminiFormKind,
    pub model: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum GeminiFormConfig {
    Official {},
    ThirdParty { base_url: String, api_key: super::create::ProviderSecretWrite },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeminiFormWrite {
    pub target_host_id: String,
    pub fields: super::basic::ProviderBasic,
    pub config: GeminiFormConfig,
    pub model: Option<String>,
    pub apply: bool,
}
concealed!(GeminiConfig, GeminiProfile, GeminiWrite, GeminiFormConfig, GeminiFormWrite);
