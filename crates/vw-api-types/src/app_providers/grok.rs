//! Grok configuration uses explicit authentication; no client paths or login payloads.
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GrokConfig {
    Official {},
    ThirdParty { toml: String },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GrokProfile {
    pub name: String,
    pub config: GrokConfig,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GrokActivation {
    #[default]
    CatalogOnly,
    First,
    IfCurrent,
    Switch,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GrokWrite {
    pub target_host_id: String,
    pub profile: GrokProfile,
    #[serde(default)]
    pub activation: GrokActivation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GrokTarget {
    pub target_host_id: String,
}
pub use super::claude::{ClaudeLiveResult as GrokLiveResult, ClaudeResult as GrokResult};
macro_rules! concealed {
    ($($t:ty),*) => {$(impl std::fmt::Debug for $t {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("GrokConfig([redacted])")
        }
    })*};
}
concealed!(GrokConfig, GrokProfile, GrokWrite);
