//! Full preference values captured in one SQLite owner read transaction.
use crate::global_database::{GlobalDatabase, config::DatabaseProvider};
use axum::http::StatusCode;
use rusqlite::{OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::path::Path;
use vw_agent_apps::data_management::{
    backup_create::{LocalBackupReceipt, create_local_backup},
    safety_snapshot::{
        OwnerPreimage, PresentPreimage, SafetySnapshotError, SafetySnapshotScope, SnapshotCategory,
        SnapshotTarget,
    },
};

pub(super) fn capture_preferences(
    root: &Path,
    scope: &SafetySnapshotScope,
    database: &GlobalDatabase,
    name: &str,
) -> Result<Option<LocalBackupReceipt>, StatusCode> {
    // Other backends need their own transaction capture, never a SQLite fallback.
    if database.provider() != DatabaseProvider::Sqlite {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }
    let mut connection = rusqlite::Connection::open_with_flags(
        database.sqlite_path().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let transaction = connection.transaction().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let selections = [
        ("agent_apps_user", scope.principal_id.as_str()),
        ("agent_apps_host", scope.host_id.as_str()),
    ];
    let targets: Vec<_> = selections
        .iter()
        .map(|(namespace, key)| SnapshotTarget {
            category: SnapshotCategory::Preference,
            object_id: serde_json::to_string(&(namespace, key))
                .expect("string tuple serialization"),
        })
        .collect();
    let mut images = Vec::new();
    for ((namespace, key), target) in selections.iter().zip(&targets) {
        let payload: Option<String> = transaction.query_row(
            "SELECT payload_json FROM gateway_settings WHERE platform_id=?1 AND company_id=?2 AND namespace=?3 AND config_key=?4",
            params![scope.project.platform_id().as_ref(), scope.project.company_id().as_ref(), namespace, key],
            |row| row.get(0)).optional().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        images.push(OwnerPreimage {
            target: target.clone(),
            state: payload.map(|payload| PresentPreimage {
                revision: hex::encode(Sha256::digest(payload.as_bytes())),
                bytes: payload.into_bytes(),
            }),
        });
    }
    // Keep the snapshot transaction alive through durable publication/readback.
    let result = create_local_backup(root, scope, &targets, name, || Ok(images)).map_err(
        |error| match error {
            SafetySnapshotError::InvalidSelection => StatusCode::BAD_REQUEST,
            SafetySnapshotError::Conflict => StatusCode::CONFLICT,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        },
    )?;
    transaction.commit().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(result)
}
