use vw_api_transport::id::ProjectId;
use serde::{Deserialize, Serialize};
use crate::*;

/// Publish only a server-issued preview; no native configuration crosses the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AggregationPublishRequest {
    pub target_host_id: String,
    pub project_id: ProjectId,
    pub expected_revision: String,
    pub request_id: String,
    pub preview_token: String,
    pub confirmed: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationErrorCode {
    InvalidRequest,
    Unauthenticated,
    Forbidden,
    NotFound,
    RevisionConflict,
    RequestConflict,
    DefaultMemberRequired,
    FailoverConflict,
    RecoveryPending,
    AlreadyExists,
    VariantConflict,
    UnsupportedApp,
    UnsupportedTarget,
    InvalidMember,
    InvalidModel,
    InvalidConfiguration,
    ConfirmationRequired,
    TargetFailed,
    RecoveryFailed,
    TargetUnavailable,
    TargetTimeout,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AggregationOutcome {
    Restarted { result: CodexRestartResult },
    Published { state: AggregationState },
    Exited { state: AggregationState },
    Failed { status: u16, code: AggregationErrorCode, recovery: RecoveryState },
}


/// Only opaque revisions and bounded hints are public; no process or credential payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexClientsState {
    pub revision: String,
    pub daemon: bool,
    pub others: bool,
    pub auth: bool,
    pub unverified: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexClientsAckRequest {
    pub target_host_id: String,
    pub project_id: ProjectId,
    pub expected_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexRestartResult {
    Restarted,
    NotRunning,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexRestartRequest {
    pub target_host_id: String,
    pub project_id: ProjectId,
    pub expected_revision: String,
    pub request_id: String,
    pub confirmed: bool,
}

/// Restore direct client settings using only the server-owned saved configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AggregationExitRequest {
    pub target_host_id: String,
    pub project_id: ProjectId,
    pub expected_revision: String,
    pub request_id: String,
    pub confirmed: bool,
}
