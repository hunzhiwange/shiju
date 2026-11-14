//! Authorized local preference restoration. Every owner write and terminal result
//! share one SQLite transaction; unsupported owner bundles fail before capture.
use crate::{
    gateway::settings_store::GatewaySettingsStore,
    global_database::{GlobalDatabase, config::DatabaseProvider},
};
use axum::{
    Extension, Json, Router,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use vw_agent_apps::data_management::{
    backup_api,
    backup_create::{LocalBackupReceipt, open_local_backup},
    recovery,
    retention::RetentionPolicy,
    safety_snapshot::*,
};
use vw_api_types::{agent_apps::data_management::*, id::TenantScope};
const NAMESPACE: &str = "data_restore_operations";
#[derive(Clone)]
struct Policy(Option<PathBuf>);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Preview {
    target: DataManagementTarget,
    backup_id: DataBackupId,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    target: DataManagementTarget,
    operation_id: String,
    fingerprint: String,
    status: String,
    executor_pid: u32,
    rollback_backup_id: Option<String>,
    result: Option<Value>,
    receipt: Option<SafetySnapshotReceipt>,
}
pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/data/backups/restore/preview", post(preview))
        .route("/agent-apps/data/backups/restore", post(execute))
        .route("/agent-apps/data/backups/restore/get", post(get))
        .route("/agent-apps/data/backups/diagnose", post(diagnose))
        .layer(Extension(Policy(policy)))
        .layer(axum::extract::DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::map_response(async |mut r: axum::response::Response| {
            r.headers_mut().insert(
                axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"),
            );
            r
        }))
}
fn unavailable() -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
fn error(e: SafetySnapshotError) -> StatusCode {
    match e {
        SafetySnapshotError::ScopeMismatch => StatusCode::NOT_FOUND,
        SafetySnapshotError::Conflict => StatusCode::CONFLICT,
        SafetySnapshotError::VersionUnsupported => StatusCode::UNPROCESSABLE_ENTITY,
        SafetySnapshotError::Corrupt
        | SafetySnapshotError::InvalidSelection
        | SafetySnapshotError::UnsafeStorage => StatusCode::BAD_REQUEST,
        _ => unavailable(),
    }
}
fn authorize(
    policy: &Policy,
    tenant: &TenantScope,
    headers: &HeaderMap,
    target: &DataManagementTarget,
) -> Result<PathBuf, StatusCode> {
    if target.scope.tenant_scope() != tenant {
        return Err(StatusCode::FORBIDDEN);
    }
    super::super::agent_apps::authorize_data_backups(policy.0.as_deref(), headers, target)
}
fn scope(t: &DataManagementTarget) -> SafetySnapshotScope {
    SafetySnapshotScope {
        project: t.scope.clone(),
        host_id: t.target_host_id.clone(),
        principal_id: t.principal_id.clone(),
    }
}
fn keys(t: &DataManagementTarget) -> [(&str, &str); 2] {
    [("agent_apps_user", t.principal_id.as_str()), ("agent_apps_host", t.target_host_id.as_str())]
}
fn validate(images: &[OwnerPreimage], t: &DataManagementTarget) -> Result<(), SafetySnapshotError> {
    if images.len() != 2 {
        return Err(SafetySnapshotError::InvalidSelection);
    }
    for (namespace, key) in keys(t) {
        let id = serde_json::to_string(&(namespace, key))
            .map_err(|_| SafetySnapshotError::InvalidSelection)?;
        let image = images
            .iter()
            .find(|i| i.target.category == SnapshotCategory::Preference && i.target.object_id == id)
            .ok_or(SafetySnapshotError::InvalidSelection)?;
        if let Some(p) = &image.state {
            if hex::encode(Sha256::digest(&p.bytes)) != p.revision {
                return Err(SafetySnapshotError::Corrupt);
            }
            if namespace == "agent_apps_user" {
                serde_json::from_slice::<vw_api_types::agent_apps::AppUserPreferences>(&p.bytes)
                    .map_err(|_| SafetySnapshotError::Corrupt)?;
            } else {
                serde_json::from_slice::<vw_api_types::agent_apps::AppHostPreferences>(&p.bytes)
                    .map_err(|_| SafetySnapshotError::Corrupt)?;
            }
        }
    }
    Ok(())
}
fn capture(
    c: &rusqlite::Connection,
    t: &DataManagementTarget,
) -> Result<Vec<OwnerPreimage>, SafetySnapshotError> {
    keys(t).iter().map(|(namespace,key)| {
        let value:Option<String>=c.query_row("SELECT payload_json FROM gateway_settings WHERE platform_id=?1 AND company_id=?2 AND namespace=?3 AND config_key=?4",params![t.scope.platform_id().as_ref(),t.scope.company_id().as_ref(),namespace,key],|r|r.get(0)).optional().map_err(|_|SafetySnapshotError::CaptureFailed)?;
        Ok(OwnerPreimage { target:SnapshotTarget { category:SnapshotCategory::Preference,object_id:serde_json::to_string(&(namespace,key)).map_err(|_|SafetySnapshotError::CaptureFailed)? }, state:value.map(|s|PresentPreimage { revision:hex::encode(Sha256::digest(s.as_bytes())),bytes:s.into_bytes() }) })
    }).collect()
}
fn receipt(summary: &DataBackupSummary) -> Result<LocalBackupReceipt, StatusCode> {
    let digest = hex::decode(&summary.digest_sha256).map_err(|_| unavailable())?;
    serde_json::from_value(json!({"id":summary.backup_id.0,"digest":digest}))
        .map_err(|_| unavailable())
}
fn database(db: Option<Extension<GlobalDatabase>>) -> Result<GlobalDatabase, StatusCode> {
    db.map(|Extension(db)| db).ok_or_else(unavailable)
}
fn connect(db: &GlobalDatabase) -> Result<rusqlite::Connection, StatusCode> {
    if db.provider() != DatabaseProvider::Sqlite {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }
    rusqlite::Connection::open_with_flags(
        db.sqlite_path().map_err(|_| unavailable())?,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
    )
    .map_err(|_| unavailable())
}
async fn preview(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    headers: HeaderMap,
    Json(input): Json<Preview>,
) -> Result<Json<Value>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let root = authorize(&p, &t, &headers, &input.target)?;
        let _guard = backup_api::lock_directory(&root).map_err(error)?;
        let s = scope(&input.target);
        let summary = backup_api::summary(&root, &s, &input.backup_id.0).map_err(error)?;
        let backup = open_local_backup(&root, &receipt(&summary)?, &s).map_err(error)?;
        validate(backup.preimages(), &input.target).map_err(error)?;
        Ok(Json(json!({"backup":summary,"requiresConfirmation":true})))
    })
    .await
    .map_err(|_| unavailable())?
}
async fn diagnose(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Json(input): Json<ListDataBackupsRequest>,
) -> Result<Json<Value>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let root = authorize(&p, &t, &headers, &input.target)?;
        let diagnosis = recovery::diagnose_recovery(&root, &scope(&input.target), || {
            let Ok(db) = database(db) else { return recovery::TargetHealth::Unavailable };
            if db.provider() != DatabaseProvider::Sqlite {
                return recovery::TargetHealth::VersionUnsupported;
            }
            let Ok(path) = db.sqlite_path() else { return recovery::TargetHealth::Unavailable };
            let Ok(c) = rusqlite::Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            ) else {
                return recovery::TargetHealth::Unavailable;
            };
            let health = c.query_row("PRAGMA quick_check(1)", [], |r| r.get::<_, String>(0));
            match health {
                Ok(s) if s == "ok" => {
                    if capture(&c, &input.target).is_ok() {
                        recovery::TargetHealth::Healthy
                    } else {
                        recovery::TargetHealth::VersionUnsupported
                    }
                }
                _ => recovery::TargetHealth::Corrupt,
            }
        })
        .map_err(error)?;
        let health = match diagnosis.target {
            recovery::TargetHealth::Healthy => "healthy",
            recovery::TargetHealth::Corrupt => "corrupt",
            recovery::TargetHealth::VersionUnsupported => "version_unsupported",
            recovery::TargetHealth::Unavailable => "unavailable",
        };
        Ok(Json(json!({"targetHealth":health,"backups":diagnosis.backups.map_err(error)?})))
    })
    .await
    .map_err(|_| unavailable())?
}
fn load(
    db: &GlobalDatabase,
    t: &DataManagementTarget,
    id: &str,
) -> Result<Option<Record>, StatusCode> {
    GatewaySettingsStore::read_existing(db, t.scope.tenant_scope(), NAMESPACE, id)
        .map_err(|_| unavailable())?
        .map(|s| serde_json::from_str(&s).map_err(|_| unavailable()))
        .transpose()
}
fn save(db: &GlobalDatabase, r: &Record) -> Result<(), StatusCode> {
    GatewaySettingsStore::new(db.clone())
        .map_err(|_| unavailable())?
        .upsert(
            r.target.scope.tenant_scope(),
            NAMESPACE,
            &r.operation_id,
            &serde_json::to_string(r).map_err(|_| unavailable())?,
        )
        .map_err(|_| unavailable())
}
fn public(r: &Record) -> Value {
    json!({"target":r.target,"operationId":r.operation_id,"status":r.status,"rollbackBackupId":r.rollback_backup_id,"result":r.result})
}
async fn get(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Json(input): Json<GetDataOperationRequest>,
) -> Result<Json<Value>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        authorize(&p, &t, &headers, &input.target)?;
        let db = database(db)?;
        let mut r = load(&db, &input.target, input.operation_id.as_ref())?
            .filter(|r| r.target == input.target)
            .ok_or(StatusCode::NOT_FOUND)?;
        if r.status == "running" || r.status == "queued" {
            if let Ok(pid) = i32::try_from(r.executor_pid) {
                // Signal zero only probes liveness; never resumes an interrupted write.
                if pid > 0
                    && unsafe { libc::kill(pid, 0) } != 0
                    && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
                {
                    r.status = "failed".into();
                    r.result = Some(json!({"code":"host_unavailable"}));
                    save(&db, &r)?;
                }
            }
        }
        Ok(Json(public(&r)))
    })
    .await
    .map_err(|_| unavailable())?
}
async fn execute(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Json(input): Json<RestoreDataBackupRequest>,
) -> Result<Json<Value>, StatusCode> {
    let runtime = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || {
        let root = authorize(&p, &t, &headers, &input.target)?;
        if !input.confirmed
            || input.idempotency_key.trim().is_empty()
            || input.idempotency_key.len() > 256
        {
            return Err(StatusCode::BAD_REQUEST);
        }
        if input.categories != [DataCategory::Preferences] {
            return Err(StatusCode::NOT_IMPLEMENTED);
        }
        let db = database(db)?;
        let _guard = backup_api::lock_directory(&root).map_err(error)?;
        let id = hex::encode(Sha256::digest(
            serde_json::to_vec(&(&input.target, &input.idempotency_key))
                .map_err(|_| StatusCode::BAD_REQUEST)?,
        ));
        let fingerprint = hex::encode(Sha256::digest(
            serde_json::to_vec(&input).map_err(|_| StatusCode::BAD_REQUEST)?,
        ));
        if let Some(r) = load(&db, &input.target, &id)? {
            return if r.target == input.target && r.fingerprint == fingerprint {
                Ok(Json(public(&r)))
            } else {
                Err(StatusCode::CONFLICT)
            };
        }
        let summary =
            backup_api::summary(&root, &scope(&input.target), &input.backup_id.0).map_err(error)?;
        if summary.revision != input.expected_revision {
            return Err(StatusCode::CONFLICT);
        }
        let source = receipt(&summary)?;
        let backup = open_local_backup(&root, &source, &scope(&input.target)).map_err(error)?;
        validate(backup.preimages(), &input.target).map_err(error)?;
        connect(&db)?;
        let record = Record {
            target: input.target.clone(),
            operation_id: id,
            fingerprint,
            status: "queued".into(),
            executor_pid: std::process::id(),
            rollback_backup_id: None,
            result: None,
            receipt: None,
        };
        save(&db, &record)?;
        let response = public(&record);
        // The directory guard moves into the worker without an admission gap.
        runtime.spawn_blocking(move || {
            let mut record = record;
            let outcome = run(&p, &t, &headers, &db, &root, &source, &mut record);
            if outcome.is_err() && record.status != "completed" {
                record.status = "failed".into();
                record.result = Some(json!({"code":"storage_unavailable"}));
                let _ = save(&db, &record);
            }
            drop(backup);
            drop(_guard);
        });
        Ok(Json(response))
    })
    .await
    .map_err(|_| unavailable())?
}
fn run(
    p: &Policy,
    t: &TenantScope,
    headers: &HeaderMap,
    db: &GlobalDatabase,
    root: &Path,
    source: &LocalBackupReceipt,
    record: &mut Record,
) -> Result<(), StatusCode> {
    if authorize(p, t, headers, &record.target)? != root {
        return Err(StatusCode::FORBIDDEN);
    }
    let target = record.target.clone();
    let c = connect(db)?;
    let current = {
        let tx = c.unchecked_transaction().map_err(|_| unavailable())?;
        let images = capture(&tx, &target).map_err(error)?;
        tx.commit().map_err(|_| unavailable())?;
        images
    };
    let shared = std::cell::RefCell::new(record.clone());
    let report = recovery::recover_from_backup(
        root,
        root,
        source,
        &target,
        RetentionPolicy::default(),
        |images| validate(images, &target),
        || Ok(current.clone()),
        |receipt| {
            let mut r = shared.borrow_mut();
            r.status = "running".into();
            r.rollback_backup_id = Some(receipt.id().into());
            r.receipt = Some(receipt.clone());
            save(db, &r).map_err(|_| SafetySnapshotError::StorageUnavailable)
        },
        |images| -> Result<(), StatusCode> {
            let mut c = connect(db)?;
            let tx = c
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|_| unavailable())?;
            if capture(&tx, &target).map_err(error)? != current {
                return Err(StatusCode::CONFLICT);
            }
            for image in images {
                let (namespace, key): (String, String) =
                    serde_json::from_str(&image.target.object_id)
                        .map_err(|_| StatusCode::BAD_REQUEST)?;
                let args = params![
                    target.scope.platform_id().as_ref(),
                    target.scope.company_id().as_ref(),
                    namespace,
                    key
                ];
                tx.execute("DELETE FROM gateway_settings WHERE platform_id=?1 AND company_id=?2 AND namespace=?3 AND config_key=?4",args).map_err(|_|unavailable())?;
                if let Some(state) = &image.state {
                    let payload =
                        std::str::from_utf8(&state.bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
                    tx.execute("INSERT INTO gateway_settings(platform_id,company_id,namespace,config_key,payload_json,updated_at_ms) VALUES (?1,?2,?3,?4,?5,CAST(strftime('%s','now') AS INTEGER)*1000)",params![target.scope.platform_id().as_ref(),target.scope.company_id().as_ref(),namespace,key,payload]).map_err(|_|unavailable())?;
                }
            }
            // Atomic publication: process death cannot leave writes without a terminal record.
            let mut terminal = shared.borrow().clone();
            terminal.status = "completed".into();
            terminal.result = Some(json!({"committed":true,"projection":"refresh_required"}));
            let payload = serde_json::to_string(&terminal).map_err(|_| unavailable())?;
            let changed=tx.execute("UPDATE gateway_settings SET payload_json=?1 WHERE platform_id=?2 AND company_id=?3 AND namespace=?4 AND config_key=?5",params![payload,target.scope.platform_id().as_ref(),target.scope.company_id().as_ref(),NAMESPACE,terminal.operation_id]).map_err(|_|unavailable())?;
            if changed != 1 {
                return Err(unavailable());
            }
            tx.commit().map_err(|_| unavailable())?;
            *shared.borrow_mut() = terminal;
            Ok(())
        },
        |_, _| Err(()),
    );
    let report = match report {
        Ok(report) => report,
        Err(e) => {
            *record = shared.into_inner();
            return Err(error(e));
        }
    };
    *record = shared.into_inner();
    record.status = if report.restored.is_ok() { "completed" } else { "failed" }.into();
    record.result = Some(if report.restored.is_ok() {
        json!({"committed":true,"projection":"refresh_required","retentionFailed":report.retention.as_ref().is_some_and(|r|r.is_err())})
    } else {
        json!({"committed":false,"code":"restore_conflict"})
    });
    save(db, record)
}
