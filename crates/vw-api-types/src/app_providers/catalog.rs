//! Read-only observations. Unavailable never means unconfigured or inactive.
use super::AppProviderSummary;
use crate::agent_apps::AppId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProviderSelection {
    Selected { id: String },
    Unconfigured,
    NotApplicable,
    Unavailable { reason: ProviderReadUnavailable },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderReadUnavailable {
    NotImplemented,
    UnsupportedPlatform,
    Unreadable,
    InvalidConfiguration,
    UnsupportedFormat,
    CustomPathNotSupported,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderMembership {
    Present,
    Absent,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderCatalogRow {
    pub provider: AppProviderSummary,
    pub native_member: ProviderMembership,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderCatalog {
    pub app_id: AppId,
    pub rows: Vec<ProviderCatalogRow>,
    /// Managed direct pointer. Native selection does not establish this pointer.
    pub direct: ProviderSelection,
    /// Native configured selection; does not attest to a running process or proxy.
    pub in_use: ProviderSelection,
    pub native_read: Option<ProviderReadUnavailable>,
}
