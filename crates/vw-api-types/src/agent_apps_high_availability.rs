//! Public contracts for application failover queues.

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
