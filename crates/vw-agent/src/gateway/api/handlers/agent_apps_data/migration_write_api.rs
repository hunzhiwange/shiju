//! Authorized provider migration. Task outcome and catalog commit are atomic.
//! Unsupported native refresh stays an explicit partial result, never success.
use super::super::agent_apps::{
    authenticate_data_migration, authorize_data_backups, authorize_data_migration,
    authorize_provider_config_access,
};
use super::migration_read_api::{State, source_scope};
use crate::{
    app_providers::AppProviderStore, gateway::settings_store::GatewaySettingsStore,
    global_database::GlobalDatabase,
};
use axum::{Extension, Json, Router, extract::Request, http::StatusCode, routing::post};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vw_agent_apps::data_management::{
    operations::ImportOperation, post_import, preview::Decision,
    safety_snapshot::SafetySnapshotReceipt,
};
use vw_api_types::{agent_apps::data_management::*, id::TenantScope, task::TaskStatus};

const NAMESPACE: &str = "agent_apps.data_import.operations";
#[derive(Clone, Serialize, Deserialize)]
struct Record {
    fingerprint: String,
    executor_pid: u32,
    progress: DataOperationProgress,
    receipt: Option<SafetySnapshotReceipt>,
}

pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/data/import/execute", post(execute))
        .route("/agent-apps/data/operations/get", post(get))
        .route("/agent-apps/data/operations/cancel", post(cancel))
        .route("/agent-apps/data/operations/retry-projections", post(retry))
}

macro_rules! endpoint {
    ($name:ident, $request:ty, $action:expr) => {
        async fn $name(
            Extension(state): Extension<State>,
            Extension(tenant): Extension<TenantScope>,
            database: Option<Extension<GlobalDatabase>>,
            request: Request,
        ) -> Result<Json<DataOperationProgress>, StatusCode> {
            authenticate_data_migration(state.policy.as_deref(), request.headers(), &tenant)?;
            let headers = request.headers().clone();
            let bytes = axum::body::to_bytes(request.into_body(), 16 * 1024)
                .await
                .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
            let input: $request =
                serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
            tokio::task::spawn_blocking(move || {
                if input.target.scope.tenant_scope() != &tenant {
                    return Err(StatusCode::FORBIDDEN);
                }
                authorize_data_migration(state.policy.as_deref(), &headers, &input.target, None)?;
                let db = database.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
                // The current domain writer supports SQLite only.
                db.connect_sqlite().map_err(|_| StatusCode::NOT_IMPLEMENTED)?;
                GatewaySettingsStore::new(db.clone())
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                ($action)(state, headers, db, input).map(Json)
            })
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        }
    };
}
endpoint!(execute, ExecuteDataImportRequest, submit);
endpoint!(get, GetDataOperationRequest, |_, _, db, input: GetDataOperationRequest| {
    read(&db, &input.target, input.operation_id.as_ref()).map(|r| r.progress)
});
endpoint!(
    cancel,
    CancelDataOperationRequest,
    |state, headers, db, input: CancelDataOperationRequest| {
        authorize_write(&state, &headers, &input.target)?;
        change(&db, &input.target, input.operation_id.as_ref(), |record| {
            if record.progress.phase == DataOperationPhase::Terminal {
                return Ok(());
            }
            // Atomic provider commit is the cancellation boundary. Never pretend to
            // cancel an in-flight transaction or overwrite its terminal result.
            if record.progress.status != TaskStatus::Queued {
                return Err(StatusCode::CONFLICT);
            }
            let op =
                ImportOperation::new(record.progress.clone()).map_err(|_| StatusCode::CONFLICT)?;
            let mut progress =
                op.cancel(&input.target, |_| Ok(())).map_err(|_| StatusCode::CONFLICT)?;
            progress.phase = DataOperationPhase::Terminal;
            progress.status = TaskStatus::Cancelled;
            progress.result = Some(DataOperationResult::Cancelled);
            record.progress = progress;
            Ok(())
        })
    }
);
endpoint!(retry, GetDataOperationRequest, retry_projection);

fn unavailable() -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
fn digest(value: &impl Serialize) -> Result<String, StatusCode> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(value).map_err(|_| StatusCode::BAD_REQUEST)?)))
}
fn load(
    connection: &rusqlite::Connection,
    target: &DataManagementTarget,
    id: &str,
) -> Result<Option<Record>, StatusCode> {
    let tenant = target.scope.tenant_scope();
    let raw: Option<String> = connection.query_row(
        "SELECT payload_json FROM gateway_settings WHERE platform_id=?1 AND company_id=?2 AND namespace=?3 AND config_key=?4",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), NAMESPACE, id], |r| r.get(0))
        .optional().map_err(|_| unavailable())?;
    raw.map(|raw| {
        let record: Record = serde_json::from_str(&raw).map_err(|_| unavailable())?;
        if record.progress.target != *target {
            return Err(StatusCode::NOT_FOUND);
        }
        Ok(record)
    })
    .transpose()
}
fn save(connection: &rusqlite::Connection, record: &Record) -> Result<(), StatusCode> {
    let p = &record.progress;
    let tenant = p.target.scope.tenant_scope();
    let raw = serde_json::to_string(record).map_err(|_| unavailable())?;
    connection.execute("INSERT INTO gateway_settings(platform_id,company_id,namespace,config_key,payload_json,updated_at_ms) VALUES(?1,?2,?3,?4,?5,CAST(strftime('%s','now') AS INTEGER)*1000) ON CONFLICT(platform_id,company_id,namespace,config_key) DO UPDATE SET payload_json=excluded.payload_json,updated_at_ms=excluded.updated_at_ms",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), NAMESPACE, p.operation_id.as_ref(), raw])
        .map_err(|_| unavailable())?;
    Ok(())
}
fn read(
    db: &GlobalDatabase,
    target: &DataManagementTarget,
    id: &str,
) -> Result<Record, StatusCode> {
    let mut record = load(&db.connect_sqlite().map_err(|_| unavailable())?, target, id)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if record.progress.phase != DataOperationPhase::Terminal && executor_exited(record.executor_pid)
    {
        // Terminal metadata shares the catalog transaction. After the executor
        // dies, a remaining nonterminal record proves that transaction did not
        // commit. Never resume source writes from a persisted task reference.
        record.progress = change(db, target, id, |r| {
            if r.progress.phase != DataOperationPhase::Terminal && executor_exited(r.executor_pid) {
                r.progress.status = TaskStatus::Failed;
                r.progress.phase = DataOperationPhase::Terminal;
                r.progress.result = Some(DataOperationResult::Failed {
                    error: DataManagementError {
                        code: DataManagementErrorCode::HostUnavailable,
                        category: None,
                        entry_id: None,
                    },
                });
            }
            Ok(())
        })?;
    }
    Ok(record)
}
fn executor_exited(pid: u32) -> bool {
    let Ok(pid) = i32::try_from(pid) else {
        return false;
    };
    if pid <= 0 {
        return false;
    }
    // SAFETY: signal zero only checks process existence; it sends no signal.
    let missing = unsafe { libc::kill(pid, 0) != 0 };
    missing && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}
fn change(
    db: &GlobalDatabase,
    target: &DataManagementTarget,
    id: &str,
    action: impl FnOnce(&mut Record) -> Result<(), StatusCode>,
) -> Result<DataOperationProgress, StatusCode> {
    let mut connection = db.connect_sqlite().map_err(|_| unavailable())?;
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| unavailable())?;
    let mut record = load(&tx, target, id)?.ok_or(StatusCode::NOT_FOUND)?;
    action(&mut record)?;
    save(&tx, &record)?;
    tx.commit().map_err(|_| unavailable())?;
    Ok(record.progress)
}

fn authorize_write(
    state: &State,
    headers: &axum::http::HeaderMap,
    target: &DataManagementTarget,
) -> Result<std::path::PathBuf, StatusCode> {
    authorize_provider_config_access(
        state.policy.as_deref(),
        headers,
        target.scope.tenant_scope(),
        &target.target_host_id,
        true,
    )?;
    authorize_data_backups(state.policy.as_deref(), headers, target)
}
fn submit(
    state: State,
    headers: axum::http::HeaderMap,
    db: GlobalDatabase,
    input: ExecuteDataImportRequest,
) -> Result<DataOperationProgress, StatusCode> {
    if !input.confirmed
        || input.idempotency_key.trim().is_empty()
        || input.idempotency_key.len() > 256
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let root = authorize_write(&state, &headers, &input.target)?;
    let id = digest(&(&input.target, &input.idempotency_key))?;
    let fingerprint = digest(&input)?;
    // Durable replay works even after source/preview expiry and process restart.
    if let Some(record) =
        load(&db.connect_sqlite().map_err(|_| unavailable())?, &input.target, &id)?
    {
        if record.fingerprint != fingerprint {
            return Err(StatusCode::CONFLICT);
        }
        return Ok(record.progress);
    }
    let mut cache = state.cache.lock().map_err(|_| unavailable())?;
    cache.expire();
    let preview = cache.previews.get(&input.preview_id.0).ok_or(StatusCode::NOT_FOUND)?;
    if preview._target != input.target {
        return Err(StatusCode::NOT_FOUND);
    }
    if preview._response.revision != input.expected_revision {
        return Err(StatusCode::CONFLICT);
    }
    let source = cache.sources.get(&preview._transfer).ok_or(StatusCode::NOT_FOUND)?;
    authorize_data_migration(
        state.policy.as_deref(),
        &headers,
        &input.target,
        Some(&source.source_id),
    )?;
    let mut decisions = Vec::new();
    for resolution in &input.conflicts {
        let conflict = preview
            ._response
            .conflicts
            .iter()
            .find(|c| c.conflict_id == resolution.conflict_id)
            .ok_or(StatusCode::BAD_REQUEST)?;
        if !conflict.allowed_decisions.contains(&resolution.decision) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let decision = match resolution.decision {
            DataConflictDecision::Skip => Decision::Skip,
            DataConflictDecision::Replace => Decision::Replace,
            DataConflictDecision::KeepBoth => return Err(StatusCode::BAD_REQUEST),
        };
        decisions.push((
            uuid::Uuid::parse_str(&resolution.conflict_id).map_err(|_| StatusCode::BAD_REQUEST)?,
            decision,
        ));
    }
    // Same executor-owned directory lock as backup maintenance; held through
    // snapshot, commit and terminal publication. No native files are activated.
    let _guard = vw_agent_apps::data_management::backup_api::lock_directory(&root)
        .map_err(|_| StatusCode::CONFLICT)?;
    AppProviderStore::new(db.clone()).map_err(|_| unavailable())?;
    let queued = DataOperationProgress {
        target: input.target.clone(),
        operation_id: id.clone().into(),
        operation: DataOperation::Import,
        status: TaskStatus::Queued,
        phase: DataOperationPhase::Queued,
        completed_items: 0,
        total_items: Some(preview._preview.entries().len() as u64),
        cancel_requested: false,
        committed: false,
        rollback: DataRollbackState::NotRequired,
        rollback_backup_id: None,
        items: vec![],
        result: None,
        created_backup: None,
        exported_transfer_id: None,
    };
    let record =
        Record { fingerprint, executor_pid: std::process::id(), progress: queued, receipt: None };
    // Admission is cross-process atomic; one target cannot have overlapping imports.
    {
        let mut connection = db.connect_sqlite().map_err(|_| unavailable())?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        if let Some(previous) = load(&tx, &input.target, &id)? {
            return if previous.fingerprint == record.fingerprint {
                Ok(previous.progress)
            } else {
                Err(StatusCode::CONFLICT)
            };
        }
        let tenant = input.target.scope.tenant_scope();
        let mut stmt = tx.prepare("SELECT payload_json FROM gateway_settings WHERE platform_id=?1 AND company_id=?2 AND namespace=?3").map_err(|_| unavailable())?;
        let rows = stmt
            .query_map(
                params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), NAMESPACE],
                |r| r.get::<_, String>(0),
            )
            .map_err(|_| unavailable())?;
        for row in rows {
            let previous: Record = serde_json::from_str(&row.map_err(|_| unavailable())?)
                .map_err(|_| unavailable())?;
            // Provider catalogs are tenant-owned, so exclude even different projects.
            if previous.progress.phase != DataOperationPhase::Terminal {
                return Err(StatusCode::CONFLICT);
            }
        }
        drop(stmt);
        save(&tx, &record)?;
        tx.commit().map_err(|_| unavailable())?;
    }
    let queued = record.progress.clone();
    let preview = preview._preview.clone();
    let source = source.snapshot.clone();
    drop(cache);
    drop(_guard);
    let task_db = db.clone();
    let executor = vw_agent_apps::apps::lifecycle::operations::Operation::new();
    let failure_db = db.clone();
    let failure_target = input.target.clone();
    let failure_id = id.clone();
    executor
        .start_task(async move {
            let result = tokio::task::spawn_blocking(move || {
                authorize_data_migration(state.policy.as_deref(), &headers, &input.target, None)?;
                let current_root = authorize_write(&state, &headers, &input.target)?;
                if current_root != root {
                    return Err(StatusCode::FORBIDDEN);
                }
                run_provider(task_db, input, root, preview, source, decisions, record)
            })
            .await;
            if matches!(result, Ok(Ok(_))) {
                Ok(())
            } else {
                let _ = change(&failure_db, &failure_target, &failure_id, |r| {
                    if r.progress.phase != DataOperationPhase::Terminal {
                        r.progress.status = TaskStatus::Failed;
                        r.progress.phase = DataOperationPhase::Terminal;
                        r.progress.result = Some(DataOperationResult::Failed {
                            error: DataManagementError {
                                code: DataManagementErrorCode::StorageUnavailable,
                                category: Some(DataCategory::Providers),
                                entry_id: None,
                            },
                        });
                    }
                    Ok(())
                });
                Err(vw_agent_apps::apps::lifecycle::operations::OperationFailure::Failed)
            }
        })
        .map_err(|_| unavailable())?;
    Ok(queued)
}

fn run_provider(
    db: GlobalDatabase,
    input: ExecuteDataImportRequest,
    root: std::path::PathBuf,
    preview: std::sync::Arc<vw_agent_apps::data_management::providers::ProviderPreview>,
    source: std::sync::Arc<vw_agent_apps::data_management::source::SourceSnapshot>,
    decisions: Vec<(uuid::Uuid, Decision)>,
    mut record: Record,
) -> Result<DataOperationProgress, StatusCode> {
    let store = AppProviderStore::new(db.clone()).map_err(|_| unavailable())?;
    let _guard = vw_agent_apps::data_management::backup_api::lock_directory(&root)
        .map_err(|_| StatusCode::CONFLICT)?;
    let scope = source_scope(&input.target)?;
    let id = record.progress.operation_id.as_ref().to_owned();
    let claimed = change(&db, &input.target, &id, |r| {
        if r.progress.phase == DataOperationPhase::Terminal {
            return Ok(());
        }
        r.progress.status = TaskStatus::Running;
        r.progress.phase = DataOperationPhase::Committing;
        Ok(())
    })?;
    if claimed.phase == DataOperationPhase::Terminal {
        return Ok(claimed);
    }
    record.progress = claimed;
    let result = store.import_cc_providers_recorded(
        &scope,
        &source,
        &preview,
        &decisions,
        &root,
        |tx, count, receipt| {
            record.progress.completed_items = record.progress.total_items.unwrap_or(0);
            record.progress.committed = count > 0;
            record.progress.status = TaskStatus::Completed;
            record.progress.phase = DataOperationPhase::Terminal;
            record.progress.rollback_backup_id = receipt.map(|r| DataBackupId(r.id().into()));
            record.receipt = receipt.cloned();
            record.progress.result = Some(if count == 0 {
                DataOperationResult::Succeeded
            } else {
                // Import changes only the saved provider catalog. No native
                // publication grant is implied; retain that missing projection.
                post_import::refresh_after_commit(&input.target, |_, projection| match projection {
                    post_import::Projection::ProviderLive => Err(()),
                    // These categories were not selected or written.
                    _ => Ok(()),
                })
            });
            save(tx, &record).map_err(|_| "migration outcome persistence failed".into())
        },
    );
    if result.is_err() {
        // The owner transaction rolls back catalog AND terminal metadata together.
        return change(&db, &input.target, &id, |r| {
            r.progress.status = TaskStatus::Failed;
            r.progress.phase = DataOperationPhase::Terminal;
            r.progress.result = Some(DataOperationResult::Failed {
                error: DataManagementError {
                    code: DataManagementErrorCode::StorageUnavailable,
                    category: Some(DataCategory::Providers),
                    entry_id: None,
                },
            });
            Ok(())
        });
    }
    Ok(record.progress)
}
fn retry_projection(
    state: State,
    headers: axum::http::HeaderMap,
    db: GlobalDatabase,
    input: GetDataOperationRequest,
) -> Result<DataOperationProgress, StatusCode> {
    let root = authorize_write(&state, &headers, &input.target)?;
    let _guard = vw_agent_apps::data_management::backup_api::lock_directory(&root)
        .map_err(|_| StatusCode::CONFLICT)?;
    change(&db, &input.target, input.operation_id.as_ref(), |record| {
        let op = ImportOperation::new(record.progress.clone()).map_err(|_| StatusCode::CONFLICT)?;
        record.progress = op
            .retry_projections(&input.target, |_, _| Err(()), |_| Ok(()))
            .map_err(|_| StatusCode::CONFLICT)?;
        Ok(())
    })
}
