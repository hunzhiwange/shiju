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
