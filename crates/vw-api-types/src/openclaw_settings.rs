//! Section-only OpenClaw wire contract; no filesystem paths or command names.
//! Reads use Option<T>: missing sections are null, empty objects remain objects,
//! and permission, host, parse and IO failures remain API errors.
//! Env reads require an independent sensitive-read grant in the authenticated
//! tenant/project/executor scope, checked before reading even a missing file.
//! Ordinary configuration read permission does not grant access to env. These
//! DTOs do not authorize access. Payloads and extensions must not enter logs.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenClawDefaultModel {
    pub primary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallbacks: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OpenClawModelCatalogEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

pub type OpenClawModelCatalog = BTreeMap<String, OpenClawModelCatalogEntry>;

/// Full agents.defaults object, matching the existing domain representation.
/// Preserve nested model/catalog, legacy timeout and unknown field shapes;
/// field projection and migration remain the domain's responsibility.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OpenClawAgentsDefaults(pub Map<String, Value>);

/// Full env object including nested vars/shellEnv and unknown values. Deliberately
/// has no Debug implementation. Never substitute masked values on denied reads.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OpenClawEnvConfig(pub Map<String, Value>);

/// Configures OpenClaw only; never grants ShiJu tool capabilities. Keep unknown
/// profile strings intact rather than coercing them to a supported profile.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OpenClawToolsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Diagnostics contain fixed descriptions and configuration member names only,
/// never offending values, env contents or parser excerpts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenClawHealthWarning {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Successful write metadata; conflicts/recovery/IO failures remain errors.
/// Structured warnings and optional backup location match the source contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenClawWriteOutcome {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
    pub warnings: Vec<OpenClawHealthWarning>,
}
