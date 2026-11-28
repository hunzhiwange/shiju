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
#[serde(rename_all = "camelCase")]
pub struct ProviderModeStatus {
    pub mode: ProviderMode,
    pub direct_provider: Option<String>,
    /// Logical target of the selected mode, not evidence of a running proxy.
    pub in_use_provider: Option<String>,
    pub proxy_route: Option<String>,
    pub attached: bool,
}
