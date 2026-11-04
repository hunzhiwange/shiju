//! Redacted evidence: only a proven absence of effects permits another submission.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CloudFailureOutcome {
    NotSent,
    Rejected,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CloudFailurePhase {
    Preflight,
    Spawn,
    NativeLaunch,
    Request,
    Response,
    Body,
    UnmatchedResponse,
    Receipt,
    Cancelled,
    Process,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CloudOperationFailure {
    pub outcome: CloudFailureOutcome,
    pub phase: CloudFailurePhase,
    pub exit_code: Option<i32>,
    pub http_status: Option<u16>,
    pub rpc_code: Option<i64>,
    pub request_id: Option<String>,
}
impl CloudOperationFailure {
    pub(super) fn not_sent(phase: CloudFailurePhase) -> Self {
        Self { outcome: CloudFailureOutcome::NotSent, ..Self::unknown(phase) }
    }
    pub(super) fn unknown(phase: CloudFailurePhase) -> Self {
        Self {
            outcome: CloudFailureOutcome::Unknown,
            phase,
            exit_code: None,
            http_status: None,
            rpc_code: None,
            request_id: None,
        }
    }
    pub(super) fn unknown_response(value: &serde_json::Value) -> Self {
        let mut failure = Self::unknown(CloudFailurePhase::Receipt);
        failure.request_id = value["requestId"]
            .as_str()
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
            .map(|id| id.to_string());
        failure
    }
    pub(super) fn from_receipt(value: &serde_json::Value) -> Self {
        let fallback = Self::unknown(CloudFailurePhase::Receipt);
        let Ok(mut failure) = serde_json::from_value::<Self>(value.clone()) else {
            return fallback;
        };
        if failure.http_status.is_some_and(|s| !(100..=599).contains(&s)) {
            return fallback;
        }
        if let Some(id) = &failure.request_id {
            let Ok(id) = uuid::Uuid::parse_str(id) else { return fallback };
            failure.request_id = Some(id.to_string());
        }
        failure
    }
    pub(super) fn permits_resubmission(&self) -> bool {
        matches!(self.outcome, CloudFailureOutcome::NotSent | CloudFailureOutcome::Rejected)
    }
}
impl std::fmt::Display for CloudOperationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cloud operation failed: {}",
            serde_json::to_string(self).map_err(|_| std::fmt::Error)?
        )
    }
}
impl std::error::Error for CloudOperationFailure {}
