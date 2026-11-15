//! Read diagnostics from an explicitly authorized execution host.
use super::AppInstallation;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppReadResponse {
    pub installations: Vec<AppInstallation>,
    /// Desktop presence is not a CLI version or proof that the app can launch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desktop: Option<AppDesktopObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDesktopObservation {
    pub supported: bool,
    pub app_bundles: Option<Vec<String>>,
    pub config_directories: Vec<String>,
}
