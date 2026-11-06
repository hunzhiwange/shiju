//! Claude Desktop configuration uses explicit authentication; no client paths or login payloads.
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ClaudeDesktopConfig {
    Official {},
    ThirdParty { base_url: String, auth_token: String },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeDesktopProfile {
    pub name: String,
    pub config: ClaudeDesktopConfig,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeDesktopActivation {
    #[default]
    CatalogOnly,
    First,
    IfCurrent,
    Switch,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeDesktopWrite {
    pub target_host_id: String,
    pub profile: ClaudeDesktopProfile,
    #[serde(default)]
    pub activation: ClaudeDesktopActivation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeDesktopTarget {
    pub target_host_id: String,
}
pub use super::claude::{
    ClaudeLiveResult as ClaudeDesktopLiveResult, ClaudeResult as ClaudeDesktopResult,
};
macro_rules! concealed {
    ($($t:ty),*) => {$(impl std::fmt::Debug for $t {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("ClaudeDesktopConfig([redacted])")
        }
    })*};
}
concealed!(ClaudeDesktopConfig, ClaudeDesktopProfile, ClaudeDesktopWrite);
