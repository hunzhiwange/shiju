//! Pi member operations; execution paths and process commands are never accepted.
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiProfile {
    pub name: String,
    pub declaration: Value,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiActivation {
    #[default]
    CatalogOnly,
    Add,
    IfMember,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiWrite {
    pub target_host_id: String,
    pub profile: PiProfile,
    #[serde(default)]
    pub activation: PiActivation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiTarget {
    pub target_host_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiState {
    pub members: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiImportResult {
    pub imported: usize,
}
pub use super::claude::{ClaudeLiveResult as PiLiveResult, ClaudeResult as PiResult};
macro_rules! concealed { ($($t:ty),*) => {$(impl std::fmt::Debug for $t { fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("Pi([redacted])") } })*}; }
concealed!(PiProfile, PiWrite);

/// Catalog metadata only. Saving this declaration never executes it or rewrites models.json.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiUsageWrite {
    pub target_host_id: String,
    pub usage_script: PiUsageScript,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiUsageScript {
    pub enabled: bool,
    pub language: String,
    pub code: String,
    pub timeout: Option<u64>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub access_token: Option<String>,
    pub user_id: Option<String>,
    pub template_type: Option<String>,
    pub auto_query_interval: Option<u64>,
    pub coding_plan_provider: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub team_organization_id: Option<String>,
    pub team_project_id: Option<String>,
}
concealed!(PiUsageWrite, PiUsageScript);
