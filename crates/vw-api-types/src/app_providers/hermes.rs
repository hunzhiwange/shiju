//! Hermes member operations; execution paths and process commands are never accepted.
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HermesProfile {
    pub name: String,
    pub declaration: Value,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HermesActivation {
    #[default]
    CatalogOnly,
    Add,
    IfMember,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HermesWrite {
    pub target_host_id: String,
    pub profile: HermesProfile,
    #[serde(default)]
    pub activation: HermesActivation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HermesTarget {
    pub target_host_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HermesState {
    pub members: Vec<String>,
    pub read_only_members: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HermesImportResult {
    pub imported: usize,
}
pub use super::claude::{ClaudeLiveResult as HermesLiveResult, ClaudeResult as HermesResult};
macro_rules! concealed { ($($t:ty),*) => {$(impl std::fmt::Debug for $t { fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("Hermes([redacted])") } })*}; }
concealed!(HermesProfile, HermesWrite);
