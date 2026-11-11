//! Wire contracts for application diagnostics owned by the execution host.

use crate::{
    agent_apps::AppId,
    id::{ModelId, ProjectId, ProjectScope, ProviderId, TaskId},
    mcp_management::{McpCatalogScope, McpConnectionCheckItem, McpRecordId, McpTransport},
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Selects a resource within the tenant authenticated by the gateway.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticScopeQuery {
    pub project_id: ProjectId,
    pub target_host_id: String,
    pub app_id: AppId,
}

/// The gateway must authorize and resolve these IDs before starting a probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DiagnosticStartRequest {
    Reachability { provider_id: ProviderId },
    Speed { provider_id: ProviderId, endpoint_ids: Vec<DiagnosticEndpointId> },
    Model { provider_id: ProviderId, model_id: ModelId },
    Streaming { provider_id: ProviderId, model_id: ModelId },
    Mcp { server_ids: Vec<McpRecordId> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticProbeKind {
    Reachability,
    Speed,
    Model,
    Streaming,
    Mcp,
    OutboundProxy,
}

/// Server-issued identity returned on acceptance. IDs do not grant access to the job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticJobIdentity {
    pub task_id: TaskId,
    pub correlation_id: String,
    pub scope: DiagnosticScope,
    pub kind: DiagnosticProbeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticScope {
    pub project: ProjectScope,
    pub target_host_id: String,
    pub app_id: AppId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticJobSnapshot {
    pub job: DiagnosticJobIdentity,
    pub state: DiagnosticJobState,
    pub progress: DiagnosticProgress,
    /// Partial or final results. Absent when detailed results are unavailable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<DiagnosticResult>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiagnosticJobState {
    // Empty struct variants reject extra payload fields during deserialization.
    Queued {},
    Running {},
    /// Cancellation has been requested and cleanup is still pending.
    Cancelling {},
    Succeeded {},
    Failed {
        error: DiagnosticFailure,
    },
    /// The probe has stopped and released its resources.
    Cancelled {},
    Interrupted {},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticProgress {
    pub completed: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
}

/// Each probe has its own evidence shape, including the identity it applies to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DiagnosticResult {
    Reachability {
        provider_id: ProviderId,
        report: DiagnosticReport<ReachabilityEvidence>,
    },
    Speed {
        endpoint_id: DiagnosticEndpointId,
        report: DiagnosticReport<SpeedEvidence>,
    },
    Model {
        provider_id: ProviderId,
        model_id: ModelId,
        may_incur_cost: bool,
        report: DiagnosticReport<ModelEvidence>,
    },
    Streaming {
        provider_id: ProviderId,
        model_id: ModelId,
        may_incur_cost: bool,
        report: DiagnosticReport<StreamingEvidence>,
    },
    Mcp {
        server_id: McpRecordId,
        report: DiagnosticReport<McpHandshakeEvidence>,
    },
    McpConnectionCheck {
        scope: McpCatalogScope,
        generation: u64,
        item: McpConnectionCheckItem,
    },
    OutboundProxy {
        report: DiagnosticReport<crate::app_routing::RoutingOutboundProbeEvidence>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticReport<E> {
    pub source: DiagnosticEvidenceSource,
    pub timings: Vec<DiagnosticTiming>,
    pub outcome: DiagnosticOutcome<E>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticEvidenceSource {
    RealService,
    ControlledFixture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DiagnosticOutcome<E> {
    Succeeded {
        evidence: E,
    },
    Failed {
        error: DiagnosticFailure,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        http_status: Option<u16>,
    },
    Cancelled {},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReachabilityEvidence {
    /// Any HTTP response establishes reachability, including 401 and 500.
    pub http_status: u16,
    pub slow: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpeedEvidence {
    pub http_status: u16,
    /// Time to GET response headers after warmup.
    pub latency_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelEvidence {
    pub http_status: u16,
    pub actual_model_id: ModelId,
}

/// Successful evidence requires both a valid increment and a valid stream ending.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StreamingEvidence {
    pub http_status: u16,
    pub actual_model_id: ModelId,
    pub first_event_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_event_gap_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpHandshakeEvidence {
    pub transport: McpTransport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticTiming {
    pub phase: DiagnosticPhase,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticPhase {
    Connect,
    Warmup,
    ResponseHeaders,
    ModelResponse,
    FirstEvent,
    Stream,
    Handshake,
    Cleanup,
}

/// Stable public classification. Raw upstream errors are kept out of the DTO.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticFailure {
    pub phase: DiagnosticPhase,
    pub code: DiagnosticFailureCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticFailureCode {
    Timeout,
    Transport,
    Unauthorized,
    Unsupported,
    UnexpectedHttpStatus,
    InvalidResponse,
}

/// Queries candidates for one authorized provider without running a probe or saving a draft.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticEndpointCandidatesQuery {
    pub provider_id: ProviderId,
    /// The host validates these before issuing references to any draft candidates.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub draft_urls: Vec<String>,
}

impl fmt::Debug for DiagnosticEndpointCandidatesQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DiagnosticEndpointCandidatesQuery")
            .field("provider_id", &self.provider_id)
            .finish_non_exhaustive()
    }
}

/// Opaque host-issued reference to a candidate in the authorized provider scope.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DiagnosticEndpointId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticEndpointCandidatesResponse {
    pub candidates: Vec<DiagnosticEndpointCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticEndpointCandidate {
    pub id: DiagnosticEndpointId,
    /// Display-safe URL prepared by the host.
    pub display_url: String,
    pub sources: Vec<DiagnosticEndpointSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticEndpointSource {
    Draft,
    SavedConfig,
    Preset,
    Custom,
}
