//! Address-book entries only; full URLs are retained verbatim, never activated or probed.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppProviderEndpointKind {
    BaseUrl,
    FullUrl,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppProviderEndpoint {
    pub url: String,
    pub kind: AppProviderEndpointKind,
    pub added_at: u64,
    pub last_used: Option<u64>,
}
/// The server supplies timestamps. MarkUsed records explicit use, not successful traffic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub enum AppProviderEndpointChange {
    Add { url: String, kind: AppProviderEndpointKind },
    Remove { url: String, kind: AppProviderEndpointKind },
    MarkUsed { url: String, kind: AppProviderEndpointKind },
}
