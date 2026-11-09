//! Follow gateway refresh operations and read their authoritative snapshots.

use std::time::Duration;

use vw_gateway_client::{
    AppsUsageOperation, AppsUsageOperationItem, AppsUsageReadError, AppsUsageWriteError,
    GatewayClient,
    vw_api_types::{
        agent_apps_auth::AuthErrorCode,
        apps_usage::{UsageSnapshotQuery, UsageSnapshotResponse},
        id::{ProjectId, TaskId},
        task::TaskStatus,
    },
};

use crate::app::components::apps_usage_cards::{CardSnapshot, Failure};

pub(crate) async fn read_snapshot(
    client: &GatewayClient,
    selection: &UsageSnapshotQuery,
    token: &str,
) -> Result<UsageSnapshotResponse, Failure> {
    client.apps_usage_snapshot(selection, token).await.map_err(|error| match error {
        AppsUsageReadError::Http { status, error } => {
            Failure::gateway(status, error.map(|error| error.code))
        }
        AppsUsageReadError::InvalidPrincipalToken => Failure::Denied,
        AppsUsageReadError::Decode => Failure::InvalidResponse,
        AppsUsageReadError::Transport => Failure::Unavailable,
    })
}

fn write_failure(error: AppsUsageWriteError) -> Failure {
    match error {
        AppsUsageWriteError::Http { status, error } => {
            Failure::gateway(status, error.map(|error| error.code))
        }
        AppsUsageWriteError::InvalidPrincipalToken => Failure::Denied,
        AppsUsageWriteError::Decode | AppsUsageWriteError::InvalidOperationId => {
            Failure::InvalidResponse
        }
        _ => Failure::Unavailable,
    }
}

fn item_failure(item: &AppsUsageOperationItem) -> Option<Failure> {
    if item.status == TaskStatus::Completed {
        return None;
    }
    Some(match item.error_code {
        Some(AuthErrorCode::ReauthRequired) => Failure::Reauth,
        Some(AuthErrorCode::Forbidden | AuthErrorCode::Unauthenticated) => Failure::Denied,
        Some(AuthErrorCode::NotFound | AuthErrorCode::Conflict) => Failure::Changed,
        Some(AuthErrorCode::Unsupported) => Failure::Unsupported,
        _ => Failure::RefreshFailed,
    })
}

pub(crate) async fn refresh_one(
    client: &GatewayClient,
    selection: &UsageSnapshotQuery,
    token: &str,
) -> Result<(), Failure> {
    let operation = client.apps_usage_refresh(selection, token).await.map_err(write_failure)?;
    let status = wait_operation(
        client,
        &selection.project_id,
        &selection.target_host_id,
        &operation.operation_id,
        token,
        |items| {
            items.is_empty()
                || matches!(items, [item]
                    if item.app_id == selection.app_id
                        && item.provider_id == selection.provider_id
                        && item.account_id == selection.account_id
                        && selection.kind.is_none_or(|kind| item.kind == kind))
        },
    )
    .await?;
    match status.items.as_slice() {
        [item] => item_failure(item).map_or(Ok(()), Err),
        [] if status.operation.status != TaskStatus::Completed => Err(Failure::RefreshFailed),
        _ => Err(Failure::InvalidResponse),
    }
}

pub(crate) async fn refresh_batch(
    client: &GatewayClient,
    project: &ProjectId,
    host: &str,
    token: &str,
) -> Result<Vec<CardSnapshot>, Failure> {
    let operation =
        client.apps_usage_refresh_all(project, host, token).await.map_err(write_failure)?;
    let status =
        wait_operation(client, project, host, &operation.operation_id, token, |_| true).await?;
    if status.items.is_empty() {
        return Err(Failure::RefreshFailed);
    }
    let mut snapshots = Vec::with_capacity(status.items.len());
    for item in status.items {
        let refresh_failure = item_failure(&item);
        let selection = UsageSnapshotQuery {
            project_id: project.clone(),
            target_host_id: host.into(),
            app_id: item.app_id,
            provider_id: item.provider_id,
            account_id: item.account_id,
            kind: Some(item.kind),
        };
        let snapshot = read_snapshot(client, &selection, token).await;
        snapshots.push(CardSnapshot::new(selection, snapshot, refresh_failure));
    }
    Ok(snapshots)
}

async fn wait_operation(
    client: &GatewayClient,
    project: &ProjectId,
    host: &str,
    id: &TaskId,
    token: &str,
    valid_items: impl Fn(&[AppsUsageOperationItem]) -> bool,
) -> Result<AppsUsageOperation, Failure> {
    tokio::time::timeout(Duration::from_secs(90), async {
        let mut status = read_status(client, host, id, token).await?;
        loop {
            if status.project_id != *project || !valid_items(&status.items) {
                return Err(Failure::InvalidResponse);
            }
            match status.operation.status {
                TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {
                    if status
                        .items
                        .iter()
                        .any(|item| matches!(item.status, TaskStatus::Queued | TaskStatus::Running))
                    {
                        return Err(Failure::InvalidResponse);
                    }
                    return Ok(status);
                }
                TaskStatus::Queued | TaskStatus::Running => {}
            }
            status = match client.apps_usage_operation_events(host, id, token).await {
                Ok(status) => status,
                Err(AppsUsageWriteError::Transport) => {
                    // Recover the same operation after an event connection drops.
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    read_status(client, host, id, token).await?
                }
                Err(error) => return Err(write_failure(error)),
            };
        }
    })
    .await
    .map_err(|_| Failure::RefreshFailed)?
}

// Only transport failures are retried, within the operation wait's deadline.
async fn read_status(
    client: &GatewayClient,
    host: &str,
    id: &TaskId,
    token: &str,
) -> Result<AppsUsageOperation, Failure> {
    loop {
        match client.apps_usage_operation(host, id, token).await {
            Err(AppsUsageWriteError::Transport) => tokio::time::sleep(Duration::from_secs(1)).await,
            result => return result.map_err(write_failure),
        }
    }
}
