//! Authorized execution-host path metadata; never a file-write command.
use crate::agent_apps::AppId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderPathsRequest {
    pub target_host_id: String,
    /// Directory override, interpreted on the execution host within its authorized HOME.
    #[serde(default)]
    pub directory_override: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigPathState {
    Present,
    Absent,
    Inaccessible,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigPathStatus {
    pub path: String,
    pub state: ConfigPathState,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderPaths {
    pub app_id: AppId,
    pub directory: String,
    pub data_directory: String,
    pub files: Vec<ConfigPathStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderDirectoryQuery {
    pub target_host_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderDirectoryUpdate {
    pub target_host_id: String,
    pub action: DirectoryOverride,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DirectoryOverride {
    Set { directory: String },
    Reset {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderDirectoryStatus {
    pub paths: ProviderPaths,
    pub directory_override: Option<String>,
}
