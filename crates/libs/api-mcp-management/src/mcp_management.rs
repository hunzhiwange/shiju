//! Wire contracts for the managed MCP catalog and its independent consumers.

use vw_api_transport::{ProjectId, ProjectScope, SessionId, TenantScope};
use vw_api_app_identity::{AppId, AppInstallationId};
use vw_api_app_auth::AuthService;
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

/// Create a record under the authenticated tenant, optionally in a project.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpCreateRequest {
    #[serde(default = "vw_api_transport::local_host_id")]
    pub target_host_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub connection: McpConnectionDefinition,
}

/// Host-admin operation on the exact existing project catalog revision.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpLinuxStdioSelection {
    Select {
        sha256: String,
        #[serde(rename = "allowedTools")]
        allowed_tools: Vec<String>,
    },
    Disable,
}

/// Revision-checked partial metadata edits and optional private connection replacement.
/// A successful save does not imply native distribution or a connection check.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpUpdateRequest {
    #[serde(default = "vw_api_transport::local_host_id")]
    pub target_host_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub expected_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection: Option<McpConnectionDefinition>,
    #[serde(default)]
    pub metadata: McpMetadataPatch,
    #[serde(default)]
    pub public_summary: McpPublicSummaryEdit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linux_stdio_selection: Option<McpLinuxStdioSelection>,
}

/// Retire an expected catalog revision. Native cleanup is reported separately per target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpDeleteRequest {
    #[serde(default = "vw_api_transport::local_host_id")]
    pub target_host_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub expected_revision: u64,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_summary: Option<String>,
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

/// Explicit, revision-bound discovery. The diagnostic query supplies the authorized host/project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpToolDetailsRequest {
    pub scope: McpCatalogScope,
    pub record: McpRecordRef,
    pub connection_revision: u64,
    pub generation: u64,
}

/// Persisted project selection; discovery never changes it or grants invocation permission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum McpRuntimeUsage {
    NotSelected,
    Selected { record: McpRecordRef, disabled: bool, generation: u64, current: bool },
}

/// Public discovered specifications and independent, stable diagnostics. No execution configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpToolDetails {
    pub request: McpToolDetailsRequest,
    pub tools: Vec<vw_api_tools::ToolSpecDto>,
    pub connection_error: Option<McpFailureCode>,
    pub tools_error: Option<McpFailureCode>,
    pub runtime: Option<McpRuntimeUsage>,
    pub runtime_error: Option<McpFailureCode>,
}

/// Explicit actions distinguish omission, replacement, and clearing without nullable ambiguity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", content = "value", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpTextEdit {
    #[default]
    Keep,
    Set(String),
    Clear,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpMetadataPatch {
    #[serde(default)]
    pub name: McpTextEdit,
    #[serde(default)]
    pub description: McpTextEdit,
    #[serde(default)]
    pub homepage: McpTextEdit,
    #[serde(default)]
    pub documentation: McpTextEdit,
    /// Omission preserves tags; an empty array clears them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}
/// The caller explicitly declares this display value public; it is never an export definition.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", content = "value", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpPublicSummaryEdit {
    #[default]
    Keep,
    Set(String),
    Clear,
}
