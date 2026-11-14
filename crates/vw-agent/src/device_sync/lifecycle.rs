//! Gateway-owned workers. Authorized configuration owners register each target;
//! transport closures reload credentials under the business lock on every hint.
use std::{future::Future, sync::Arc};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use vw_api_types::{agent_apps::DataManagementTarget, device_sync_config::DeviceSyncTransport,
    device_sync_task::{SyncErrorCode, SyncRemoteSnapshot}};
use super::{auto_sync, change_events, operation_lock::CurrentSyncConnection, tasks};

struct Worker {
    target: DataManagementTarget,
    transport: DeviceSyncTransport,
    cancellation: CancellationToken,
    handle: tokio::task::JoinHandle<Result<(), SyncErrorCode>>,
}
#[derive(Default)]
struct State { closed: bool, workers: Vec<Worker> }

/// One instance per gateway lifetime, shared with authorized service handlers.
/// Never bind this owner to the first request's tenant in a process-global cell.
#[derive(Clone, Default)]
pub struct SyncLifecycle { state: Arc<Mutex<State>> }
impl SyncLifecycle {
    /// Register before publishing a connection as active. Duplicate registration
    /// preserves the existing subscription. Journal recovery precedes any upload.
    pub async fn start<C, R, RF, U, UF>(&self,
        database: crate::global_database::GlobalDatabase,
        target: DataManagementTarget, transport: DeviceSyncTransport, reload: R, upload: U,
    ) -> Result<(), SyncErrorCode>
    where C: Send + 'static,
        R: FnMut() -> RF + Send + 'static,
        RF: Future<Output = Result<Option<CurrentSyncConnection<C>>, SyncErrorCode>> + Send,
        U: FnMut(C, tasks::SyncTaskExecution) -> UF + Send + 'static,
        UF: Future<Output = Result<SyncRemoteSnapshot, SyncErrorCode>> + Send,
    {
        let mut state = self.state.lock().await;
        if state.closed { return Err(SyncErrorCode::Cancelled); }
        if let Some(worker) = state.workers.iter().find(|w| w.target == target && w.transport == transport) {
            return if worker.handle.is_finished() { Err(SyncErrorCode::SnapshotUnavailable) } else { Ok(()) };
        }
        tasks::initialize(database, &target)?;
        let changes = change_events::subscribe(target.scope.tenant_scope());
        let cancellation = CancellationToken::new();
        let handle = tokio::spawn(auto_sync::run(target.clone(), transport, changes,
            cancellation.clone(), reload, upload));
        state.workers.push(Worker { target, transport, cancellation, handle });
        Ok(())
    }

    /// Configuration disable/clear must await this before returning. Saving the
    /// disabled configuration under the shared lock makes any pending reload skip.
    pub async fn stop(&self, target: &DataManagementTarget,
        transport: DeviceSyncTransport) -> Result<(), SyncErrorCode> {
        let mut state = self.state.lock().await;
        let Some(index) = state.workers.iter().position(|w| &w.target == target && w.transport == transport)
            else { return Ok(()); };
        state.workers[index].cancellation.cancel();
        let cancelled = tasks::cancel_automatic(target, transport);
        let worker = state.workers.remove(index);
        let joined = worker.handle.await.map_err(|_| SyncErrorCode::SnapshotUnavailable)?;
        cancelled?;
        joined
    }

    /// Stop scheduling first, request cancellation before commit, then await all
    /// transfer cleanup/rollback. Keep the registry locked until workers exit so
    /// a concurrent registration cannot overlap a retiring worker.
    pub async fn shutdown(&self) -> Result<(), SyncErrorCode> {
        let mut state = self.state.lock().await;
        state.closed = true;
        for worker in &state.workers { worker.cancellation.cancel(); }
        let mut failure = None;
        for worker in &state.workers {
            if let Err(code) = tasks::cancel_automatic(&worker.target, worker.transport) { failure = Some(code); }
        }
        for worker in state.workers.drain(..) {
            match worker.handle.await {
                Ok(Ok(())) => {},
                Ok(Err(code)) => { failure = Some(code); },
                Err(_) => { failure = Some(SyncErrorCode::SnapshotUnavailable); },
            }
        }
        failure.map_or(Ok(()), Err)
    }
}
