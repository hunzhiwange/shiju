//! Host-local task journal in the unified gateway database, never synchronized.
//! Initialization must run under the gateway's exclusive execution-host ownership.
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use vw_api_types::{
    agent_apps::DataManagementTarget,
    device_sync_config::DeviceSyncTransport,
    device_sync_task::{SyncErrorCode, SyncRemoteSnapshot, SyncTaskState, SyncTaskStatus},
};
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use super::tasks::Record;

const NAMESPACE: &str = "device_sync.status.local.v1";
type Result<T> = std::result::Result<T, SyncErrorCode>;

/// Each summary is scoped to the complete target and transport. Safe task DTOs
/// retain origin, direction and terminal state without raw transport errors.
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct SyncHistory {
    pub last_success: Option<SyncTaskStatus>,
    pub last_error: Option<SyncTaskStatus>,
    pub manifest_baseline: Option<SyncRemoteSnapshot>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Journal {
    records: Vec<Record>,
    webdav: SyncHistory,
    s3: SyncHistory,
}
impl Journal {
    fn history(&mut self, transport: DeviceSyncTransport) -> &mut SyncHistory {
        match transport {
            DeviceSyncTransport::Webdav => &mut self.webdav,
            DeviceSyncTransport::S3 => &mut self.s3,
        }
    }
}
struct Binding {
    target: DataManagementTarget,
    store: GatewaySettingsStore,
}
fn bindings() -> &'static Mutex<Vec<Binding>> {
    static BINDINGS: OnceLock<Mutex<Vec<Binding>>> = OnceLock::new();
    BINDINGS.get_or_init(Default::default)
}
fn key(target: &DataManagementTarget) -> Result<String> {
    serde_json::to_string(target).map_err(|_| SyncErrorCode::StatePersistenceFailed)
}
fn decode(raw: Option<&str>, target: &DataManagementTarget) -> anyhow::Result<Journal> {
    let journal: Journal = raw.map(serde_json::from_str).transpose()?.unwrap_or_default();
    anyhow::ensure!(journal.records.iter().all(|r| &r.status.target == target
        && &r.request.target == target), "invalid device sync journal scope");
    Ok(journal)
}

// Conditional replacement rejects another process's update rather than losing it.
fn update<T>(store: &GatewaySettingsStore, target: &DataManagementTarget,
    operation: impl FnOnce(Option<&str>) -> anyhow::Result<(T, Option<String>)>) -> anyhow::Result<T> {
    let key = key(target).map_err(|_| anyhow::anyhow!("invalid sync target"))?;
    let tenant = target.scope.tenant_scope();
    let previous = store.get(tenant, NAMESPACE, &key)?;
    let (result, replacement) = operation(previous.as_deref())?;
    if let Some(payload) = replacement {
        let changed = match previous {
            Some(previous) => store.compare_exchange(tenant, NAMESPACE, &key, &previous, &payload)?,
            None => store.insert_if_absent(tenant, NAMESPACE, &key, &payload)?,
        };
        anyhow::ensure!(changed, "device sync journal changed concurrently");
    }
    Ok(result)
}

pub(super) fn initialize(database: GlobalDatabase, target: &DataManagementTarget,
    records: &mut Vec<Record>) -> Result<()> {
    let mut bindings = bindings().lock().map_err(|_| SyncErrorCode::StatePersistenceFailed)?;
    if bindings.iter().any(|b| &b.target == target) { return Ok(()); }
    let result = (|| -> anyhow::Result<_> {
        let store = GatewaySettingsStore::new(database)?;
        let recovered = update(&store, target, |raw| {
                let mut journal = decode(raw, target)?;
                let mut interrupted = Vec::new();
                for record in &mut journal.records {
                    if !record.status.state.is_terminal() {
                        record.status.state = SyncTaskState::Interrupted { committed: record.committed };
                        interrupted.push(record.status.clone());
                    }
                }
                for status in interrupted {
                    journal.history(status.transport).last_error = Some(status);
                }
                let payload = serde_json::to_string(&journal)?;
                Ok((journal.records, Some(payload)))
            })?;
        Ok((store, recovered))
    })().map_err(|_| SyncErrorCode::StatePersistenceFailed)?;
    records.extend(result.1);
    bindings.push(Binding { target: target.clone(), store: result.0 });
    Ok(())
}

pub(super) fn persist(record: &Record) -> Result<()> {
    let bindings = bindings().lock().map_err(|_| SyncErrorCode::StatePersistenceFailed)?;
    let target = &record.status.target;
    let binding = bindings.iter().find(|b| &b.target == target)
        .ok_or(SyncErrorCode::StatePersistenceFailed)?;
    update(&binding.store, target, |raw| {
            let mut journal = decode(raw, target)?;
            if let Some(old) = journal.records.iter_mut()
                .find(|r| r.status.task_id == record.status.task_id) {
                *old = record.clone();
            } else { journal.records.push(record.clone()); }
            let history = journal.history(record.status.transport);
            match &record.status.state {
                SyncTaskState::Succeeded { snapshot } => {
                    history.last_success = Some(record.status.clone());
                    history.manifest_baseline = Some(snapshot.clone());
                }
                SyncTaskState::Failed { .. } | SyncTaskState::Interrupted { .. } => {
                    history.last_error = Some(record.status.clone());
                }
                _ => {}
            }
            Ok(((), Some(serde_json::to_string(&journal)?)))
        }).map_err(|_| SyncErrorCode::StatePersistenceFailed)
}

/// Reauthorize the complete target before reading host-local history.
pub fn history(target: &DataManagementTarget, transport: DeviceSyncTransport) -> Result<SyncHistory> {
    let bindings = bindings().lock().map_err(|_| SyncErrorCode::StatePersistenceFailed)?;
    let binding = bindings.iter().find(|b| &b.target == target)
        .ok_or(SyncErrorCode::StatePersistenceFailed)?;
    let raw = binding.store.get(target.scope.tenant_scope(), NAMESPACE, &key(target)?)
        .map_err(|_| SyncErrorCode::StatePersistenceFailed)?;
    let mut journal = decode(raw.as_deref(), target).map_err(|_| SyncErrorCode::StatePersistenceFailed)?;
    Ok(journal.history(transport).clone())
}

/// Read without initialization: observing settings must not interrupt live workers.
pub fn connection_status(database: GlobalDatabase, target: &DataManagementTarget,
    transport: DeviceSyncTransport) -> Result<(Option<SyncTaskStatus>, Option<vw_api_types::id::TaskId>)> {
    let store = GatewaySettingsStore::new(database).map_err(|_| SyncErrorCode::StatePersistenceFailed)?;
    let raw = store.get(target.scope.tenant_scope(), NAMESPACE, &key(target)?)
        .map_err(|_| SyncErrorCode::StatePersistenceFailed)?;
    let journal = decode(raw.as_deref(), target).map_err(|_| SyncErrorCode::StatePersistenceFailed)?;
    let records: Vec<_> = journal.records.iter().filter(|r| r.status.transport == transport).collect();
    let active = records.iter().rev().find(|r| !r.status.state.is_terminal())
        .map(|r| r.status.task_id.clone());
    Ok((records.last().map(|r| r.status.clone()), active))
}
