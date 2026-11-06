//! Terminal launch inputs contain identities and argv, never command strings or credentials.
use super::{AppId, AppInstallationId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LaunchRequest {
    pub app_id: AppId,
    pub installation_id: AppInstallationId,
    pub cwd: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub skip_permissions: bool,
}

/// Returned only after the terminal boundary has accepted the launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LaunchResult {
    pub installation_id: AppInstallationId,
}
