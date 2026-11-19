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
#[serde(default, deny_unknown_fields)]
pub struct ComputerControlUserConfig {
    /// Allow otherwise unlisted native applications; explicit denial takes priority.
    pub native_apps: bool,
    pub chrome: bool,
    pub browser_permissions: BrowserPermissions,
    pub excel: bool,
    pub word: bool,
    /// Additional gate, never a replacement for target authorization or OS probes.
    pub locked_use: bool,
    /// Stable bundle IDs, not window titles. Individual allows persist independently
    /// of the broad native-app switch; removing one falls back to that switch.
    pub applications: BTreeMap<String, GrantDecision>,
    /// Exact canonical origins, without paths, credentials or wildcards.
    pub chrome_origins: BTreeMap<String, GrantDecision>,
    /// Host-assigned stable document identities; not titles, body text or pairing tokens.
    pub excel_documents: BTreeMap<String, GrantDecision>,
    pub word_documents: BTreeMap<String, GrantDecision>,
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

/// Missing fields inherit the default policy; an explicit denial always wins.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BrowserPermission { #[default] Ask, Allow, Deny }
impl std::fmt::Display for BrowserPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self { Self::Ask => "需要批准", Self::Allow => "始终允许", Self::Deny => "阻止" })
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct BrowserOriginPermissions {
    pub access: Option<BrowserPermission>,
    pub downloads: Option<BrowserPermission>,
    pub uploads: Option<BrowserPermission>,
    pub full_cdp_access: Option<BrowserPermission>,
    pub website_tools: Option<BrowserPermission>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct BrowserPermissions {
    pub history: BrowserPermission,
    pub website_tools: bool,
    pub full_cdp_access: bool,
    pub defaults: BrowserOriginPermissions,
    pub origins: BTreeMap<String, BrowserOriginPermissions>,
}
impl BrowserPermissions {
    pub fn resolve(&self, origin: &str, field: fn(&BrowserOriginPermissions) -> Option<BrowserPermission>) -> BrowserPermission {
        self.origins.get(origin).and_then(field).or_else(|| field(&self.defaults)).unwrap_or_default()
    }
}
