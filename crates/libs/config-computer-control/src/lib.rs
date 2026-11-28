//! Persistent authorization metadata only: no host tokens, document contents or images.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ComputerControlConfig {
    /// Monotonic across all users so a change invalidates every in-flight snapshot.
    pub permission_version: u64,
    /// Keys are trusted local OS user identities, never model-provided identities.
    pub users: BTreeMap<String, ComputerControlUserConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(from = "StoredComputerControlUser")]
#[schemars(!from, default, deny_unknown_fields)]
pub struct ComputerControlUserConfig {
    /// Allow otherwise unlisted native applications; explicit denial takes priority.
    pub native_apps: bool,
    /// Additional gate, never a replacement for target authorization or OS probes.
    pub locked_use: bool,
    /// Stable bundle IDs, not window titles. Individual allows persist independently
    /// of the broad native-app switch; removing one falls back to that switch.
    pub applications: BTreeMap<String, GrantDecision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GrantDecision {
    Allow,
    Deny,
}

#[cfg(test)]
#[path = "computer_control_tests.rs"]
mod computer_control_tests;

// Old profiles can still carry grants for removed integrations. Consume those
// keys only while reading, so they never affect authorization or get serialized.
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct StoredComputerControlUser {
    native_apps: bool,
    locked_use: bool,
    applications: BTreeMap<String, GrantDecision>,
    #[serde(rename = "chrome")]
    _chrome: Option<serde::de::IgnoredAny>,
    #[serde(rename = "browser_permissions")]
    _browser_permissions: Option<serde::de::IgnoredAny>,
    #[serde(rename = "excel")]
    _excel: Option<serde::de::IgnoredAny>,
    #[serde(rename = "word")]
    _word: Option<serde::de::IgnoredAny>,
    #[serde(rename = "chrome_origins")]
    _chrome_origins: Option<serde::de::IgnoredAny>,
    #[serde(rename = "excel_documents")]
    _excel_documents: Option<serde::de::IgnoredAny>,
    #[serde(rename = "word_documents")]
    _word_documents: Option<serde::de::IgnoredAny>,
}

impl From<StoredComputerControlUser> for ComputerControlUserConfig {
    fn from(stored: StoredComputerControlUser) -> Self {
        Self {
            native_apps: stored.native_apps,
            locked_use: stored.locked_use,
            applications: stored.applications,
        }
    }
}
