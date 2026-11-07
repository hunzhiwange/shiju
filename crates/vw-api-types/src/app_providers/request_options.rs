//! Write-only options for native Claude/Codex records. Views contain no user-supplied strings.
use super::validation::ProviderRequestOptions;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestOptionsTarget {
    pub target_host_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestOptionsWrite {
    pub target_host_id: String,
    pub options: ProviderRequestOptions,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestOptionsApplication {
    NotApplied,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestOptionsView {
    pub headers_configured: bool,
    pub overrides_configured: bool,
    pub application: RequestOptionsApplication,
}
