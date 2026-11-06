//! OpenCode member operations; execution paths and process commands are never accepted.
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenCodeFormat {
    #[default]
    V1,
    V2,
}
impl OpenCodeFormat {
    pub fn key(self) -> &'static str {
        match self {
            Self::V1 => "provider",
            Self::V2 => "providers",
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenCodeProfile {
    pub name: String,
    #[serde(default)]
    pub format: OpenCodeFormat,
    pub declaration: Value,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenCodeActivation {
    #[default]
    CatalogOnly,
    Add,
    IfMember,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenCodeWrite {
    pub target_host_id: String,
    pub profile: OpenCodeProfile,
    #[serde(default)]
    pub activation: OpenCodeActivation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenCodeTarget {
    pub target_host_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenCodeDefaultModel {
    pub target_host_id: String,
    pub model: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenCodeState {
    pub members: Vec<String>,
    pub model: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenCodeImportResult {
    pub imported: usize,
}
pub use super::claude::{ClaudeLiveResult as OpenCodeLiveResult, ClaudeResult as OpenCodeResult};
macro_rules! concealed { ($($t:ty),*) => {$(impl std::fmt::Debug for $t { fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("OpenCode([redacted])") } })*}; }
concealed!(OpenCodeProfile, OpenCodeWrite);
