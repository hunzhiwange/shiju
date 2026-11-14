//! Bounded post-commit scheduling. The gateway lifecycle owns and awaits this loop.
use std::{future::Future, time::Duration};

use tokio::{sync::watch, time::Instant};
use tokio_util::sync::CancellationToken;
use vw_api_types::{
    agent_apps::DataManagementTarget,
    device_sync_config::DeviceSyncTransport,
    device_sync_task::{SyncDirection, SyncErrorCode, SyncRemoteSnapshot, SyncStartRequest, SyncTaskOrigin},
};

use super::{operation_lock::{acquire_sync_lock, CurrentSyncConnection}, tasks};

const DEBOUNCE: Duration = Duration::from_secs(1);
const MAX_WAIT: Duration = Duration::from_secs(10);

/// Run one transport for an already authorized target. Subscribe using
/// change_events::subscribe before spawning this future. Lifecycle must enforce
/// one worker per target/transport and initialize the durable task journal first.
///
/// Reload resolves current configuration and credentials under the shared lock.
/// Upload must use only that material, await transfer and baseline persistence
/// inline, and map errors to safe codes. Neither closure may detach work. Upload
/// owns cancellation at its commit boundary; shutdown waits for it to return.
/// No initial upload or error retry is synthesized: only unseen commit hints run.
pub async fn run<C, Reload, ReloadFuture, Upload, UploadFuture>(
    target: DataManagementTarget,
    transport: DeviceSyncTransport,
    mut changes: watch::Receiver<u64>,
    cancellation: CancellationToken,
    mut reload: Reload,
    mut upload: Upload,
) -> Result<(), SyncErrorCode>
where
    Reload: FnMut() -> ReloadFuture,
    ReloadFuture: Future<Output = Result<Option<CurrentSyncConnection<C>>, SyncErrorCode>>,
    Upload: FnMut(C, tasks::SyncTaskExecution) -> UploadFuture,
    UploadFuture: Future<Output = Result<SyncRemoteSnapshot, SyncErrorCode>>,
{
    let tenant = target.scope.tenant_scope();
    let lock_tenant = serde_json::to_string(&(
        tenant.platform_id().as_ref(), tenant.company_id().as_ref(),
    )).map_err(|_| SyncErrorCode::InvalidRequest)?;
    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            changed = changes.changed() => {
                if changed.is_err() { return Ok(()); }
            }
        }
        let first = Instant::now();
        let mut last = first;
        loop {
            let deadline = (last + DEBOUNCE).min(first + MAX_WAIT);
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                () = tokio::time::sleep_until(deadline) => break,
                changed = changes.changed() => {
                    if changed.is_err() { return Ok(()); }
                    last = Instant::now();
                }
            }
        }
        // Consume only hints included in this attempt. Changes while waiting for
        // the lock, reloading or uploading remain dirty for the next iteration.
        changes.borrow_and_update();
        let guard = match acquire_sync_lock(&lock_tenant, &target.target_host_id, &cancellation).await {
            Ok(guard) => guard,
            Err(_) if cancellation.is_cancelled() => return Ok(()),
            Err(_) => return Err(SyncErrorCode::SnapshotUnavailable),
        };
        if cancellation.is_cancelled() { return Ok(()); }
        let connection = reload().await;
        if matches!(&connection, Ok(None)) { continue; }
        if matches!(&connection, Ok(Some(c)) if !c.enabled || !c.auto_sync) { continue; }
        if cancellation.is_cancelled() { return Ok(()); }
        let status = tasks::accept(SyncStartRequest {
            target: target.clone(), transport, direction: SyncDirection::Upload,
            request_id: uuid::Uuid::new_v4().to_string(), expected_remote_snapshot: None,
        }, SyncTaskOrigin::Auto)?;
        // Error results are durable Auto task failures, not immediate retries.
        // In-flight hints are untouched even on failure. Persistence errors stop
        // the worker so lifecycle can report a failure instead of losing it.
        let upload = &mut upload;
        tasks::execute(&target, &status.task_id, |execution| async move {
            match connection {
                Ok(Some(connection)) => upload(connection.material, execution).await,
                Err(code) => Err(code),
                Ok(None) => Err(SyncErrorCode::NotConfigured),
            }
        }).await?;
        drop(guard);
    }
}
