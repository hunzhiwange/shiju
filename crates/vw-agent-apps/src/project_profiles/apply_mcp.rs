//! Minimal MCP changes over an executor-authorized observation of one target.
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
};

use vw_api_types::{
    mcp_management::{
        McpAppTarget, McpDistributionItem, McpDistributionOutcome, McpFailureCode, McpRecordRef,
    },
    project_profiles::{
        ProjectProfileCategory, ProjectProfileItemResult, ProjectProfileItemStatus,
        ProjectProfilePayload, ProjectProfileScope,
    },
};

/// Complete catalog references and effective enablement for exactly this target.
/// Readers must reject unreadable, unmanaged or inconsistent native state.
pub struct McpSlotState {
    pub target: McpAppTarget,
    pub records: BTreeMap<String, (McpRecordRef, bool)>,
}

/// Null is a strict no-op, including no observation. The executor must authorize
/// project, host, installation, catalog and credential use before supplying the
/// reader and writer. The writer must call the catalog binding service, which
/// rechecks the revision and acquires its own native locks. No native lock may
/// be held across this call. Failures stay per item; committed items are retained.
pub async fn apply_mcp<R, W, F>(
    payload: &ProjectProfilePayload,
    group: ProjectProfileScope,
    target: &McpAppTarget,
    read: R,
    mut toggle: W,
) -> Vec<ProjectProfileItemResult>
where
    R: FnOnce() -> Result<McpSlotState, McpFailureCode>,
    W: FnMut(McpRecordRef, bool) -> F,
    F: Future<Output = Result<McpDistributionItem, McpFailureCode>>,
{
    let Some(ids) = payload.mcp.get(group) else { return Vec::new() };
    if target.app_id != group.app_id() || target.target_host_id.trim().is_empty() {
        return vec![failure(None, McpFailureCode::PermissionDenied)];
    }
    if group == ProjectProfileScope::ClaudeDesktop {
        return vec![item(None, ProjectProfileItemStatus::Unsupported, Some("mcp_unsupported"))];
    }
    let state = match read() {
        Ok(state) if state.target == *target => state,
        Ok(_) => return vec![failure(None, McpFailureCode::PermissionDenied)],
        Err(code) => return vec![failure(None, code)],
    };
    if state.records.iter().any(|(id, (record, _))| id.trim().is_empty() || *id != record.id.0) {
        return vec![failure(None, McpFailureCode::Conflict)];
    }
    let desired: BTreeSet<&str> = ids.iter().map(String::as_str).collect();
    let all: BTreeSet<&str> =
        state.records.keys().map(String::as_str).chain(desired.iter().copied()).collect();
    let mut results = Vec::new();
    for id in all {
        let Some((record, enabled)) = state.records.get(id) else {
            results.push(failure(Some(id), McpFailureCode::NotFound));
            continue;
        };
        let wanted = desired.contains(id);
        if *enabled == wanted {
            continue;
        }
        let result = match toggle(record.clone(), wanted).await {
            Err(code) => failure(Some(id), code),
            Ok(result)
                if result.target != *target
                    || result.record.id != record.id
                    || result.enabled != wanted =>
            {
                failure(Some(id), McpFailureCode::Conflict)
            }
            Ok(result) => match result.native_outcome {
                McpDistributionOutcome::Applied | McpDistributionOutcome::Unchanged
                    if result.desired_committed =>
                {
                    item(Some(id), ProjectProfileItemStatus::Succeeded, None)
                }
                McpDistributionOutcome::Unsupported => {
                    item(Some(id), ProjectProfileItemStatus::Unsupported, Some("mcp_unsupported"))
                }
                McpDistributionOutcome::Cancelled => {
                    item(Some(id), ProjectProfileItemStatus::Cancelled, Some("mcp_cancelled"))
                }
                McpDistributionOutcome::Failed { code } => failure(Some(id), code),
                _ => item(Some(id), ProjectProfileItemStatus::Failed, Some("mcp_not_verified")),
            },
        };
        results.push(result);
    }
    results
}

fn item(
    id: Option<&str>,
    status: ProjectProfileItemStatus,
    reason: Option<&str>,
) -> ProjectProfileItemResult {
    ProjectProfileItemResult {
        category: ProjectProfileCategory::Mcp,
        reference_id: id.map(str::to_owned),
        status,
        reason_code: reason.map(str::to_owned),
    }
}

fn failure(id: Option<&str>, code: McpFailureCode) -> ProjectProfileItemResult {
    use McpFailureCode::*;
    let (status, reason) = match code {
        NotFound => (ProjectProfileItemStatus::Missing, "mcp_not_found"),
        StaleRevision => (ProjectProfileItemStatus::Conflict, "mcp_stale_revision"),
        Conflict => (ProjectProfileItemStatus::Conflict, "mcp_conflict"),
        PermissionDenied => (ProjectProfileItemStatus::Denied, "mcp_permission_denied"),
        InvalidDefinition => (ProjectProfileItemStatus::Failed, "mcp_invalid_definition"),
        TargetUnavailable => (ProjectProfileItemStatus::Failed, "mcp_target_unavailable"),
        CredentialUnavailable => (ProjectProfileItemStatus::Failed, "mcp_credential_unavailable"),
        StorageUnavailable => (ProjectProfileItemStatus::Failed, "mcp_storage_unavailable"),
        NativeWriteFailed => (ProjectProfileItemStatus::Failed, "mcp_native_write_failed"),
        ConnectionFailed => (ProjectProfileItemStatus::Failed, "mcp_connection_failed"),
        ToolDiscoveryFailed => (ProjectProfileItemStatus::Failed, "mcp_tool_discovery_failed"),
        TimedOut => (ProjectProfileItemStatus::Failed, "mcp_timed_out"),
    };
    item(id, status, Some(reason))
}
