//! Minimal saved native catalog contract. Model discovery and preset knowledge extend this owner.
use crate::agent_apps::AppId;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelCatalogIdentity {
    pub target_host_id: String,
    pub app_id: AppId,
    pub provider_id: String,
    pub endpoint: String,
}
/// Already prepared native rows; no template guessing or network enrichment on publication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexNativeModel {
    pub slug: String,
    pub display_name: String,
    pub description: String,
    pub base_instructions: String,
    pub supports_reasoning_summaries: bool,
    pub supports_parallel_tool_calls: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedCodexCatalog {
    pub models: Vec<CodexNativeModel>,
}
