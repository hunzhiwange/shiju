//! Device-local connection DTOs. These must never enter a portable sync snapshot.
use std::fmt;

use serde::{Deserialize, Serialize};

/// Closed transport vocabulary; unknown values fail deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceSyncTransport {
    Webdav,
    S3,
}

/// Write-only intent. Replace carries the exact input, including an empty string;
/// connection validation decides whether that input is valid. No implicit Keep.
/// Serialize only into an authorized write request, never a config response/log.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum SyncSecretUpdate {
    Keep {},
    Replace { value: String },
    Clear {},
}

impl fmt::Debug for SyncSecretUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keep {} => f.write_str("Keep"),
            Self::Replace { .. } => f.write_str("Replace { value: [redacted] }"),
            Self::Clear {} => f.write_str("Clear"),
        }
    }
}

/// Read model: password material cannot be represented here. The host must
/// validate URLs to exclude embedded credentials before returning this model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebDavSyncConfig {
    pub enabled: bool,
    pub auto_sync: bool,
    pub base_url: String,
    pub username: String,
    pub remote_root: String,
    pub profile: String,
    pub secret_present: bool,
}

/// Read model: secret access keys are owned exclusively by the host credential store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct S3SyncConfig {
    pub enabled: bool,
    pub auto_sync: bool,
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    /// None selects the standard AWS endpoint.
    pub endpoint: Option<String>,
    pub remote_root: String,
    pub profile: String,
    pub secret_present: bool,
}

/// Explicit write request, separate from the read model. Keep is valid only for
/// the same authorized scope and connection identity, checked by the host.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateWebDavSyncConfigRequest {
    pub enabled: bool,
    pub auto_sync: bool,
    pub base_url: String,
    pub username: String,
    pub remote_root: String,
    pub profile: String,
    pub secret: SyncSecretUpdate,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateS3SyncConfigRequest {
    pub enabled: bool,
    pub auto_sync: bool,
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub endpoint: Option<String>,
    pub remote_root: String,
    pub profile: String,
    pub secret: SyncSecretUpdate,
}

// Do not log unvalidated draft URLs or other user-controlled connection material.
impl fmt::Debug for UpdateWebDavSyncConfigRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UpdateWebDavSyncConfigRequest { [redacted] }")
    }
}

impl fmt::Debug for UpdateS3SyncConfigRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UpdateS3SyncConfigRequest { [redacted] }")
    }
}

/// Authorized local connection and discoverable background work; no credential material.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebDavSyncConfigResponse {
    pub config: Option<WebDavSyncConfig>,
    pub last_status: Option<crate::device_sync_task::SyncTaskStatus>,
    pub active_task_id: Option<crate::id::TaskId>,
}

/// Authorized host-local S3 connection; secret material is never a response field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct S3SyncConfigResponse {
    pub config: Option<S3SyncConfig>,
    pub last_status: Option<crate::device_sync_task::SyncTaskStatus>,
    pub active_task_id: Option<crate::id::TaskId>,
}

/// Saved-connection preview. Contains no endpoint, path, ETag or credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum DeviceSyncRemoteResponse {
    Empty,
    Snapshot {
        device_name: String,
        created_at: String,
        snapshot_id: String,
        protocol_version: u32,
        data_compat_version: u32,
        compatibility: DeviceSyncRemoteCompatibility,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceSyncRemoteCompatibility {
    Compatible,
    UnsupportedProtocol,
    UnsupportedDataVersion,
}
