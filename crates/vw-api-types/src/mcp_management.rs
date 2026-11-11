//! Wire contracts for the managed MCP catalog and its independent consumers.

use crate::{
    ProjectScope, SessionId, TaskId, TenantScope,
    agent_apps::{AppId, AppInstallationId},
    agent_apps_auth::AuthService,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    Stdio,
    StreamableHttp,
    Sse,
}

/// Reference to an existing authorization account. Resolution and scope checks
/// belong to the execution host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpCredentialRef {
    pub service: AuthService,
    pub account_id: String,
}

/// Only non-secret configuration belongs in a literal. Credentials use references.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpConfigValue {
    Literal { value: String },
    Credential { reference: McpCredentialRef },
}

/// Configuration input for validation on the server. Public records and results
/// omit this definition because commands, arguments and URLs can carry secrets.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "transport", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpConnectionDefinition {
    Stdio {
        command: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        env: BTreeMap<String, McpConfigValue>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
    },
    StreamableHttp {
        url: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        headers: BTreeMap<String, McpConfigValue>,
    },
    Sse {
        url: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        headers: BTreeMap<String, McpConfigValue>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpCatalogScope {
    Tenant { scope: TenantScope },
    Project { scope: ProjectScope },
}

/// Stable within the catalog scope.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct McpRecordId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpRecordRef {
    pub id: McpRecordId,
    pub revision: u64,
}

/// Public projection. The server sanitizes display metadata before constructing
/// a record. Usage and native file state are returned separately.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpCatalogRecord {
    pub record: McpRecordRef,
    pub scope: McpCatalogScope,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub transport: McpTransport,
    /// Remains stable across metadata-only edits.
    pub connection_revision: u64,
}

/// The server resolves these identifiers against the authorized host and installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpAppTarget {
    pub app_id: AppId,
    pub installation_id: AppInstallationId,
    pub target_host_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpCliBinding {
    pub record: McpRecordRef,
    pub target: McpAppTarget,
    pub enabled: bool,
    pub native_status: McpNativeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpNativeStatus {
    Present,
    Missing,
    Unreadable,
    NotVerified,
    Unsupported,
}

/// Selection for the ShiJu runtime in a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpRuntimeSelection {
    pub record: McpRecordRef,
    pub project: ProjectScope,
}

/// Selection for one ACP session. The session determines its execution host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpAcpMount {
    pub record: McpRecordRef,
    pub session_id: SessionId,
}

/// Availability of a specific MCP capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpSupport {
    Supported,
    Unsupported,
    NotVerified,
}

/// Native distribution support by transport. Installation and connection
/// observations are separate from adapter capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpAppCapability {
    pub app_id: AppId,
    pub stdio: McpSupport,
    pub streamable_http: McpSupport,
    pub sse: McpSupport,
}

/// Stable diagnostics for clients to render. Raw errors and execution payloads
/// stay on the execution side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpFailureCode {
    InvalidDefinition,
    StaleRevision,
    Conflict,
    NotFound,
    PermissionDenied,
    TargetUnavailable,
    CredentialUnavailable,
    StorageUnavailable,
    NativeWriteFailed,
    ConnectionFailed,
    ToolDiscoveryFailed,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpBatchOutcome {
    Succeeded,
    PartiallySucceeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpDistributionResult {
    pub outcome: McpBatchOutcome,
    pub items: Vec<McpDistributionItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpDistributionItem {
    /// Revision after the attempt if the requested binding was saved.
    pub record: McpRecordRef,
    pub target: McpAppTarget,
    /// Requested binding state, even if the operation failed.
    pub enabled: bool,
    /// Whether the requested binding is already committed in the catalog.
    pub desired_committed: bool,
    pub native_outcome: McpDistributionOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpDistributionOutcome {
    Applied,
    Unchanged,
    NotVerified,
    Unsupported,
    Failed { code: McpFailureCode },
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpConnectionCheckResult {
    pub operation_id: TaskId,
    pub generation: u64,
    pub outcome: McpBatchOutcome,
    pub items: Vec<McpConnectionCheckItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpConnectionCheckItem {
    pub id: McpRecordId,
    pub connection_revision: u64,
    pub outcome: McpConnectionOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum McpConnectionOutcome {
    Connected { tool_count: Option<u32> },
    NotVerified,
    Unsupported,
    Failed { code: McpFailureCode },
    Cancelled,
}

/// A stable MCP management error without host or backend diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpFailureResponse {
    pub code: McpFailureCode,
}

/// The catalog and its desired native bindings on the requested host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpCatalogResponse {
    pub records: Vec<McpCatalogRecord>,
    pub bindings: Vec<McpCliBinding>,
    pub capabilities: Vec<McpAppCapability>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpPresetsResponse {
    pub presets: Vec<McpPresetDraftView>,
}

/// A built-in, disabled draft. The connection contains only static preset values.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpPresetDraftView {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub homepage: String,
    pub documentation: String,
    pub connection: McpConnectionDefinition,
    pub enabled: bool,
}

/// Observation for the requested installation, with no native paths or definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpNativePreview {
    pub target: McpAppTarget,
    pub status: McpNativeStatus,
    pub entries: Vec<McpNativePreviewEntry>,
}

/// Compare a catalog ID against the native file on the authorized host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpNativePreviewEntry {
    pub id: McpRecordId,
    pub status: McpNativeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<McpTransport>,
}
