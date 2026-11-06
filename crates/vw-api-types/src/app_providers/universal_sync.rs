//! Explicit catalog projection; native application activation is not implied.
use crate::agent_apps::AppId;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UniversalSyncRequest {
    pub apps: Vec<AppId>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UniversalSyncError {
    UnsupportedApp,
    AppDisabled,
    InvalidTargetIdentity,
    InvalidProfile,
    TargetConflict,
    StorageUnavailable,
    ChangedDuringUpdate,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum UniversalCatalogResult {
    Updated,
    Failed { code: UniversalSyncError },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UniversalLiveResult {
    NotAttempted,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UniversalSyncResult {
    pub app_id: AppId,
    pub provider_id: Option<String>,
    pub catalog: UniversalCatalogResult,
    pub live: UniversalLiveResult,
}
