//! Inert prompt catalog writes. Native activation has a separate owner.
use axum::{Extension, Json, Router, extract::{DefaultBodyLimit, rejection::JsonRejection}, http::{HeaderMap, StatusCode, header}, response::{IntoResponse, Response}, routing::post};
use serde::Deserialize;
use std::path::PathBuf;
use vw_api_types::{ProjectId, ProjectScope, TenantScope, agent_apps::AppId};
use crate::{global_database::GlobalDatabase, agent_apps::prompts_store::{PromptsStore, PromptScope, PromptEntry}};

#[derive(Clone)]
struct Policy(Option<PathBuf>);
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    project_id: ProjectId,
    target_host_id: String,
    app_id: AppId,
    operation: Operation,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Save { revision: u64, id: Option<String>, name: String, content: String, description: Option<String> },
    Delete { revision: u64, id: String },
    Copy { id: String, targets: Vec<AppId> },
    Import { revision: u64, source: ImportSource },
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ImportSource {
    UploadedContent { name: String, content: String, description: Option<String> },
    CurrentFile,
}
type Failure = (StatusCode, &'static str);
pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/prompts/save", post(save))
        .route("/agent-apps/prompts/delete", post(delete))
        .route("/agent-apps/prompts/copy", post(copy))
        .route("/agent-apps/prompts/import", post(import))
        .layer(DefaultBodyLimit::max(6 * 1024 * 1024 + 4096))
        .layer(Extension(Policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))))
}
fn error((status, code): Failure) -> Response {
    use vw_api_types::error::{ApiErrorBody, ApiErrorDetail};
    (status, [(header::CACHE_CONTROL, "no-store")], Json(ApiErrorBody {
        error: ApiErrorDetail { code: code.into(), message: code.into(), details: None },
    })).into_response()
}
macro_rules! handler {
    ($name:ident, $pattern:pat) => {
        async fn $name(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>,
            database: Option<Extension<GlobalDatabase>>, h: HeaderMap,
            body: Result<Json<Request>, JsonRejection>) -> Response {
            let Ok(Json(q)) = body else { return error((StatusCode::BAD_REQUEST, "invalid_request")); };
            if !matches!(&q.operation, $pattern) { return error((StatusCode::BAD_REQUEST, "invalid_operation")); }
            let Some(Extension(db)) = database else { return error((StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable")); };
            match tokio::task::spawn_blocking(move || execute(p.0, t, h, db, q)).await {
                Ok(Ok(value)) => ([(header::CACHE_CONTROL, "no-store")], Json(value)).into_response(),
                Ok(Err(e)) => error(e),
                Err(_) => error((StatusCode::SERVICE_UNAVAILABLE, "target_unavailable")),
            }
        }
    }
}
handler!(save, Operation::Save { .. });
handler!(delete, Operation::Delete { .. });
handler!(copy, Operation::Copy { .. });
handler!(import, Operation::Import { .. });

#[cfg(not(target_os = "macos"))]
fn execute(_: Option<PathBuf>, _: TenantScope, _: HeaderMap, _: GlobalDatabase, _: Request) -> Result<serde_json::Value, Failure> {
    Err((StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))
}
#[cfg(target_os = "macos")]
fn execute(policy: Option<PathBuf>, tenant: TenantScope, headers: HeaderMap, db: GlobalDatabase, q: Request) -> Result<serde_json::Value, Failure> {
    use vw_api_types::agent_apps::PromptResource;
    let project = ProjectScope::new(tenant, q.project_id).map_err(|_| (StatusCode::BAD_REQUEST, "invalid_request"))?;
    vw_agent_apps::providers::validate_identity(&q.target_host_id).map_err(|_| (StatusCode::BAD_REQUEST, "invalid_host"))?;
    let authorize = |app, write| {
        PromptResource::Application { app_id: app }.validate_application_access()
            .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))?;
        let result = if write {
            super::agent_apps::authorize_prompts_write(policy.as_deref(), &headers, &project, &q.target_host_id, app)
        } else {
            super::agent_apps::authorize_prompts_read(policy.as_deref(), &headers, &project, &q.target_host_id, app)
        };
        result.map_err(|s| (s, if s == StatusCode::UNAUTHORIZED { "unauthenticated" } else { "forbidden" }))
    };
    let (home, directory, _) = authorize(q.app_id, !matches!(&q.operation, Operation::Copy { .. }))?;
    let store = PromptsStore::new(db).map_err(map_error)?;
    let scope = PromptScope { project: &project, host: &q.target_host_id, app: q.app_id };
    let catalog = match q.operation {
        Operation::Save { revision, id, name, content, description } => {
            if q.app_id == AppId::Mcode && vw_agent_apps::prompts::mcode::validate_content(&content).is_err() {
                return Err((StatusCode::PAYLOAD_TOO_LARGE, "content_too_large"));
            }
            let updating = id.is_some();
            let entry = PromptEntry { id: id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()), name, content, description, created_at: None };
            if updating { store.update(scope, revision, entry) } else { store.create(scope, revision, entry) }
        }
        Operation::Delete { revision, id } => store.delete(scope, revision, &id),
        Operation::Copy { id, targets } => {
            if targets.is_empty() || targets.len() > 14 { return Err((StatusCode::BAD_REQUEST, "invalid_targets")); }
            authorize(q.app_id, false)?;
            // Keep target authorization codes intact instead of converting them to storage errors.
            let mut results = Vec::new();
            for (index, app) in targets.iter().copied().enumerate() {
                let result = if app == q.app_id || targets[..index].contains(&app) {
                    Err((StatusCode::BAD_REQUEST, "invalid_target"))
                } else {
                    authorize(app, true).map(|_| ())
                };
                let value = match result {
                    Err((status, code)) => serde_json::json!({"app_id": app, "error": {"code": code, "status": status.as_u16()}}),
                    Ok(()) => {
                        let mut copies = store.copy_to_apps(scope, &id, &[app], |s| {
                            authorize(s.app, s.app != q.app_id).map(|_| ()).map_err(|_| anyhow::anyhow!("prompt authorization changed"))
                        }).map_err(map_error)?;
                        match copies.remove(0).result {
                            Ok(entry) => serde_json::json!({"app_id": app, "entry": entry}),
                            Err(e) => { let (status, code) = map_error(e); serde_json::json!({"app_id": app, "error": {"code": code, "status": status.as_u16()}}) }
                        }
                    }
                };
                results.push(value);
            }
            return Ok(serde_json::json!({"results": results}));
        }
        Operation::Import { revision, source: ImportSource::UploadedContent { name, content, description } } =>
            store.import_uploaded_content(scope, revision, name, content.into_bytes(), description),
        Operation::Import { revision, source: ImportSource::CurrentFile } => {
            authorize(q.app_id, false)?;
            // Overrides and environment are execution-host state, never client fields.
            let environment = std::env::vars().collect();
            store.import_current_file(scope, revision, &q.target_host_id, &home, if q.app_id == AppId::Pi { directory.as_ref().and_then(|p| p.to_str()) } else { None }, &environment)
        }
    }.map_err(map_error)?;
    serde_json::to_value(catalog).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "serialization_failed"))
}
fn map_error(error: anyhow::Error) -> Failure {
    // Only known static errors become public codes; never expose payloads or paths.
    match error.to_string().as_str() {
        "prompt catalog revision conflict" | "prompt catalog revision exhausted" => (StatusCode::CONFLICT, "revision_conflict"),
        "prompt already exists" => (StatusCode::CONFLICT, "already_exists"),
        "prompt not found" => (StatusCode::NOT_FOUND, "not_found"),
        "active prompt edits require activate_ordinary" | "deactivate prompt before deletion" => (StatusCode::CONFLICT, "active_prompt"),
        "invalid prompt identity" | "prompt name is empty" | "imported prompt name is empty" => (StatusCode::BAD_REQUEST, "invalid_request"),
        "prompt file does not exist" => (StatusCode::NOT_FOUND, "missing_file"),
        "MCode prompt exceeds 32768 UTF-8 bytes" => (StatusCode::PAYLOAD_TOO_LARGE, "content_too_large"),
        "prompt authorization changed" => (StatusCode::FORBIDDEN, "forbidden"),
        "prompt file is not valid UTF-8" => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_utf8"),
        "unsafe prompt file path" => (StatusCode::UNPROCESSABLE_ENTITY, "unsafe_path"),
        "prompt file changed during preview; retry" => (StatusCode::CONFLICT, "revision_conflict"),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "storage_or_target_unavailable"),
    }
}
