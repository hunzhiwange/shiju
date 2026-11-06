//! User preferences are separate from execution-host settings.
use super::AppId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppUserPreferences {
    /// Raw saved IDs permit filtering removed/unknown entries during normalization.
    #[serde(default)]
    pub app_order: Vec<String>,
    pub visible_apps: Vec<AppId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppHostPreferences {
    pub preferred_terminal: Option<String>,
    #[serde(default)]
    pub wsl_shells: Vec<AppWslPreference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppWslPreference {
    pub app_id: AppId,
    /// None or "auto" selects the distro default.
    pub shell: Option<String>,
    pub flag: Option<String>,
}

/// Advisory only: applying preferences must not restart any process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppPreferencesResult {
    pub user: AppUserPreferences,
    pub host: AppHostPreferences,
    pub restart_required: bool,
}

/// Replace one complete preferences domain per PATCH. Other domains are preserved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scope", content = "value", rename_all = "snake_case", deny_unknown_fields)]
pub enum AppPreferencesPatch {
    User(AppUserPreferences),
    Host(AppHostPreferences),
}
