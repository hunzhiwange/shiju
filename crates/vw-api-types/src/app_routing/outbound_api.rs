//! Execution-host egress settings and redacted runtime status.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RoutingEgressMode {
    Direct,
    System,
    Custom,
}

/// The proxy URL is write-only and may contain credentials.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoutingEgressUpdate {
    pub mode: RoutingEgressMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
}

impl std::fmt::Debug for RoutingEgressUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoutingEgressUpdate").field("mode", &self.mode).finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoutingEgressStatus {
    pub configured_mode: RoutingEgressMode,
    pub effective_mode: RoutingEgressMode,
    pub scheme: Option<String>,
    pub has_credentials: bool,
}
