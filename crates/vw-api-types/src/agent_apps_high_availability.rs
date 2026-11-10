//! Public contracts for application failover.

use crate::{
    agent_apps::AppId,
    id::{ProjectId, ProjectScope, ProviderId},
};
use serde::{Deserialize, Serialize};

/// Selects a queue within the tenant authenticated by the gateway.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FailoverQueueQuery {
    pub project_id: ProjectId,
    pub target_host_id: String,
    pub app_id: AppId,
}

/// Identifies the resource resolved by the gateway for a response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FailoverQueueScope {
    pub project: ProjectScope,
    pub target_host_id: String,
    pub app_id: AppId,
}

/// Opaque queue revision used to detect concurrent writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FailoverQueueVersion(pub u64);

/// Public provider identity shared by queue members and eligible candidates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FailoverQueueEntry {
    pub provider_id: ProviderId,
    pub name: String,
}

/// Current queue state returned by reads and successful mutations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FailoverQueueResponse {
    pub scope: FailoverQueueScope,
    pub version: FailoverQueueVersion,
    pub enabled: bool,
    /// The array order is the order in which the members are considered.
    pub entries: Vec<FailoverQueueEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FailoverCandidatesResponse {
    pub scope: FailoverQueueScope,
    pub candidates: Vec<FailoverQueueEntry>,
}

/// Body for adding or removing a provider from the selected queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FailoverQueueMemberRequest {
    pub expected_version: FailoverQueueVersion,
    pub provider_id: ProviderId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FailoverQueueReorderRequest {
    pub expected_version: FailoverQueueVersion,
    /// Complete list of current members in the desired order.
    pub provider_ids: Vec<ProviderId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FailoverQueueEnabledRequest {
    pub expected_version: FailoverQueueVersion,
    pub enabled: bool,
}

/// App-wide policy for subsequent requests.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FailoverPolicy {
    pub circuit_breaker: CircuitBreakerPolicy,
    pub retry_timeout: RetryTimeoutPolicy,
}

impl FailoverPolicy {
    /// Validate the policy before saving or applying it.
    pub fn validate(&self) -> Result<(), &'static str> {
        let circuit = self.circuit_breaker;
        if !(1..=20).contains(&circuit.failure_threshold) {
            return Err("failureThreshold must be between 1 and 20");
        }
        if !(1..=10).contains(&circuit.success_threshold) {
            return Err("successThreshold must be between 1 and 10");
        }
        if circuit.cooldown_secs > 300 {
            return Err("cooldownSecs must be between 0 and 300");
        }
        if !(0.0..=1.0).contains(&circuit.error_rate_threshold) {
            return Err("errorRateThreshold must be a finite ratio between 0 and 1");
        }
        if !(5..=100).contains(&circuit.min_requests) {
            return Err("minRequests must be between 5 and 100");
        }

        let retry = self.retry_timeout;
        if retry.max_retries > 10 {
            return Err("maxRetries must be between 0 and 10");
        }
        if !(1..=120).contains(&retry.first_byte_timeout_secs) {
            return Err("firstByteTimeoutSecs must be between 1 and 120");
        }
        if retry.idle_timeout_secs > 600 {
            return Err("idleTimeoutSecs must be between 0 and 600");
        }
        if !(60..=1200).contains(&retry.non_streaming_timeout_secs) {
            return Err("nonStreamingTimeoutSecs must be between 60 and 1200");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CircuitBreakerPolicy {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    /// Time in seconds before allowing a half-open probe.
    pub cooldown_secs: u32,
    /// Fraction of failed requests, from 0.0 to 1.0.
    pub error_rate_threshold: f64,
    pub min_requests: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetryTimeoutPolicy {
    /// Number of sends allowed in addition to the initial send.
    pub max_retries: u8,
    pub first_byte_timeout_secs: u16,
    /// Zero disables only the idle timer.
    pub idle_timeout_secs: u16,
    pub non_streaming_timeout_secs: u16,
}

/// Production circuit state for a provider within the authorized app scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailoverHealthState {
    Closed,
    Open,
    HalfOpen,
    /// No current observation is available.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FailoverHealthResponse {
    pub scope: FailoverQueueScope,
    pub provider_id: ProviderId,
    pub state: FailoverHealthState,
    /// Remaining cooldown in milliseconds, if available.
    pub retry_after_ms: Option<u64>,
}

/// Public failure classification. Raw upstream errors are excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailoverFailureReason {
    Connection,
    RateLimited,
    UpstreamUnavailable,
    FirstByteTimeout,
    IdleTimeout,
    DeadlineExceeded,
    Other,
}

/// Routing observation, independent of the provider's circuit state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FailoverRouteResponse {
    pub scope: FailoverQueueScope,
    pub configured_provider_id: Option<ProviderId>,
    /// The observed target, or None when no actual target is known.
    pub actual_provider_id: Option<ProviderId>,
    pub failure_reason: Option<FailoverFailureReason>,
}
