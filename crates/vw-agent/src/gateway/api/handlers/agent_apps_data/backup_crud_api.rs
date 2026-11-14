//! Scope-authorized maintenance of executor-owned local backups.
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Json, Router,
    extract::DefaultBodyLimit,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use std::path::PathBuf;
use vw_api_types::{agent_apps::data_management::*, id::TenantScope};
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/data/backups/create", post(create))
        .route("/agent-apps/data/backups/list", post(list))
        .route("/agent-apps/data/backups/rename", post(rename))
        .route("/agent-apps/data/backups/delete", post(delete))
        .layer(Extension(Policy(policy)))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(
                axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"),
            );
            response
        }))
}
enum Operation {
    Create(CreateDataBackupRequest),
    List,
    Rename(RenameDataBackupRequest),
    Delete(DeleteDataBackupRequest),
}
macro_rules! endpoint {
    ($function:ident, $request:ty, $operation:expr) => {
        async fn $function(
            Extension(policy): Extension<Policy>,
            Extension(tenant): Extension<TenantScope>,
            db: Option<Extension<GlobalDatabase>>,
            headers: HeaderMap,
            Json(request): Json<$request>,
        ) -> Result<Json<serde_json::Value>, StatusCode> {
            let target = request.target.clone();
            execute(
                policy,
                tenant,
                db.map(|Extension(db)| db),
                headers,
                target,
                ($operation)(request),
            )
            .await
        }
    };
}
endpoint!(create, CreateDataBackupRequest, Operation::Create);
endpoint!(list, ListDataBackupsRequest, |_| Operation::List);
endpoint!(rename, RenameDataBackupRequest, Operation::Rename);
endpoint!(delete, DeleteDataBackupRequest, Operation::Delete);
async fn execute(
    policy: Policy,
    tenant: TenantScope,
    db: Option<GlobalDatabase>,
    headers: HeaderMap,
    target: DataManagementTarget,
    operation: Operation,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        if target.scope.tenant_scope() != &tenant {
            return Err(StatusCode::FORBIDDEN);
        }
        let root = super::super::agent_apps::authorize_data_backups(
            policy.0.as_deref(),
            &headers,
            &target,
        )?;
        #[cfg(target_os = "macos")]
        {
            run(&root, db, target, operation).map(Json)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (root, db, operation);
            Err(StatusCode::NOT_IMPLEMENTED)
        }
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
#[cfg(target_os = "macos")]
fn error(
    error: vw_agent_apps::data_management::safety_snapshot::SafetySnapshotError,
) -> StatusCode {
    use vw_agent_apps::data_management::safety_snapshot::SafetySnapshotError::*;
    match error {
        InvalidSelection | UnsafeStorage => StatusCode::BAD_REQUEST,
        ScopeMismatch => StatusCode::NOT_FOUND,
        Conflict => StatusCode::CONFLICT,
        VersionUnsupported => StatusCode::UNPROCESSABLE_ENTITY,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}
#[cfg(target_os = "macos")]
fn run(
    root: &std::path::Path,
    db: Option<GlobalDatabase>,
    target: DataManagementTarget,
    operation: Operation,
) -> Result<serde_json::Value, StatusCode> {
    use sha2::{Digest, Sha256};
    use vw_agent_apps::data_management::{
        backup_api, backup_create, backup_list, backup_rename, safety_snapshot::SafetySnapshotScope,
    };
    let scope = SafetySnapshotScope {
        project: target.scope,
        host_id: target.target_host_id,
        principal_id: target.principal_id,
    };
    let _guard = backup_api::lock_directory(root).map_err(error)?;
    let entries = backup_list::list_local_backups(root, &scope).map_err(error)?;
    let find = |id: &str| -> Result<(), StatusCode> {
        if entries.iter().any(|entry| entry.id == id) { Ok(()) } else { Err(StatusCode::NOT_FOUND) }
    };
    let result = match operation {
        Operation::List => {
            let summaries = entries
                .iter()
                .map(|entry| backup_api::summary(root, &scope, &entry.id).map_err(error))
                .collect::<Result<Vec<_>, _>>()?;
            serde_json::to_value(summaries)
        }
        Operation::Rename(request) => {
            find(&request.backup_id.0)?;
            backup_api::rename_label(
                root,
                &scope,
                &request.backup_id.0,
                &request.expected_revision,
                &request.name,
            )
            .map_err(error)?;
            serde_json::to_value(
                backup_api::summary(root, &scope, &request.backup_id.0).map_err(error)?,
            )
        }
        Operation::Delete(request) => {
            if !request.confirmed {
                return Err(StatusCode::BAD_REQUEST);
            }
            find(&request.backup_id.0)?;
            backup_api::delete_checked(
                root,
                &scope,
                &request.backup_id.0,
                &request.expected_revision,
            )
            .map_err(error)?;
            Ok(serde_json::Value::Null)
        }
        Operation::Create(request) => {
            // Only this complete owner bundle is wired. Never substitute portable
            // projections for credentials, native files or unsupported categories.
            if request.kind != DataArtifactKind::LocalRollback
                || request.categories != [DataCategory::Preferences]
            {
                return Err(StatusCode::NOT_IMPLEMENTED);
            }
            if request.idempotency_key.is_empty()
                || request.idempotency_key.len() > 256
                || request.name.trim().is_empty()
                || request.name.len() > 256
                || request.name.chars().any(char::is_control)
            {
                return Err(StatusCode::BAD_REQUEST);
            }
            let identity = serde_json::to_vec(&(&scope, &request.idempotency_key))
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            let id = hex::encode(Sha256::digest(identity));
            if entries.iter().any(|entry| entry.id == id) {
                let previous = backup_api::summary(root, &scope, &id).map_err(error)?;
                if previous.name != request.name || previous.categories != request.categories {
                    return Err(StatusCode::CONFLICT);
                }
                return serde_json::to_value(previous).map_err(|_| StatusCode::SERVICE_UNAVAILABLE);
            }
            let database = db.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
            let receipt =
                super::backup_capture::capture_preferences(root, &scope, &database, &request.name)?;
            let Some(receipt) = receipt else { return Ok(serde_json::Value::Null) };
            let receipt =
                backup_rename::rename_local_backup(root, &receipt, &scope, &id).map_err(error)?;
            // Both metadata and archive content are read from the real owner format.
            let _ = backup_create::open_local_backup(root, &receipt, &scope).map_err(error)?;
            serde_json::to_value(backup_api::summary(root, &scope, &id).map_err(error)?)
        }
    };
    result.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
