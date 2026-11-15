//! Read-only execution-host mode observations. No credentials, contract hashes or file paths.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderMode {
    #[default]
    Direct,
    Proxy,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModeStateQuery {
    pub target_host_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub mode: ProviderMode,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RewriteDecision {
    SkipRewrite,
    RewriteRequired,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StackStatus {
    pub enabled: bool,
    pub active: bool,
    pub members: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RouteRuntimeStatus {
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderModeStatus {
    pub mode: ProviderMode,
    pub direct_provider: Option<String>,
    /// Logical target of the selected mode, not evidence of a running proxy.
    pub in_use_provider: Option<String>,
    pub proxy_route: Option<String>,
    pub attached: bool,
    pub has_contract: bool,
    pub stack: StackStatus,
    pub decision: Option<RewriteDecision>,
    pub route_runtime: RouteRuntimeStatus,
}
