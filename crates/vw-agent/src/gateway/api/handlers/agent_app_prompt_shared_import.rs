//! Confirmed shares use scoped create-only storage, then guarded native activation.
use axum::{Extension, Json, Router, extract::{DefaultBodyLimit, rejection::JsonRejection}, http::{HeaderMap, StatusCode, header}, response::{IntoResponse, Response}, routing::post};
use std::path::PathBuf;
use crate::global_database::GlobalDatabase;
use vw_api_types::{TenantScope, agent_apps::ImportPromptShareApiRequest};

#[derive(Clone)]
struct Policy(Option<PathBuf>);
type Failure = (StatusCode, &'static str);
pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/prompts/import-share", post(import))
        .layer(DefaultBodyLimit::max(6 * 1024 * 1024 + 4096))
        .layer(Extension(Policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))))
}
async fn import(Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>, headers: HeaderMap,
    body: Result<Json<ImportPromptShareApiRequest>, JsonRejection>) -> Response {
    let result = match (body, database) {
        (Ok(Json(request)), Some(Extension(db))) => {
            tokio::task::spawn_blocking(move || execute(policy.0, tenant, headers, db, request))
                .await.unwrap_or(Err((StatusCode::SERVICE_UNAVAILABLE, "target_unavailable")))
        }
        (Err(_), _) => Err((StatusCode::BAD_REQUEST, "invalid_request")),
        (_, None) => Err((StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable")),
    };
    match result {
        Ok(value) => ([(header::CACHE_CONTROL, "no-store")], Json(value)).into_response(),
        Err((status, code)) => {
            use vw_api_types::error::{ApiErrorBody, ApiErrorDetail};
            (status, [(header::CACHE_CONTROL, "no-store")], Json(ApiErrorBody {
                error: ApiErrorDetail { code: code.into(), message: code.into(), details: None },
            })).into_response()
        }
    }
}
#[cfg(not(target_os = "macos"))]
fn execute(_: Option<PathBuf>, _: TenantScope, _: HeaderMap, _: GlobalDatabase,
    _: ImportPromptShareApiRequest) -> Result<vw_api_types::agent_apps::ImportPromptShareResult, Failure> {
    Err((StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))
}
#[cfg(target_os = "macos")]
fn execute(policy: Option<PathBuf>, tenant: TenantScope, headers: HeaderMap, db: GlobalDatabase,
    q: ImportPromptShareApiRequest) -> Result<vw_api_types::agent_apps::ImportPromptShareResult, Failure> {
    use crate::agent_apps::prompts_store::{PromptScope, PromptsStore, PromptEntry as StoredEntry, PromptCatalog};
    use vw_api_types::{ProjectScope, agent_apps::*};
    use vw_agent_apps::prompts::{shared_import::{self, SharedImportError, SharedImportResult}, live_file};
    let invalid = || (StatusCode::BAD_REQUEST, "invalid_request");
    let unavailable = || (StatusCode::SERVICE_UNAVAILABLE, "storage_or_target_unavailable");
    q.import.validate().map_err(|_| invalid())?;
    vw_agent_apps::providers::validate_identity(&q.target_host_id).map_err(|_| invalid())?;
    let project = ProjectScope::new(tenant.clone(), q.project_id.clone()).map_err(|_| invalid())?;
    let app = q.import.target_app;
    let authorize = || super::agent_apps::authorize_prompts_write(policy.as_deref(), &headers, &project, &q.target_host_id, app)
        .map_err(|s| (s, if s == StatusCode::UNAUTHORIZED { "unauthenticated" } else { "forbidden" }));
    let context = authorize()?;
    // Activation authority is checked before persisting; metadata suggestions do not enter this branch.
    if matches!(&q.import.activation, PromptActivationChoice::Activate { .. }) {
        super::agent_apps::authorize_prompts_activation(policy.as_deref(), &headers, &project, &q.target_host_id, app)
            .map_err(|s| (s, if s == StatusCode::UNAUTHORIZED { "unauthenticated" } else { "forbidden" }))?;
    }
    let store = PromptsStore::new(db.clone()).map_err(|_| unavailable())?;
    let scope = PromptScope { project: &project, host: &q.target_host_id, app };
    let entry_from = |catalog: PromptCatalog, id: &str, enabled: bool| -> Result<PromptEntry, Failure> {
        let e = catalog.entries.into_iter().find(|e| e.id == id).ok_or_else(unavailable)?;
        Ok(PromptEntry { id: e.id, app_id: app, name: e.name, description: e.description,
            content: e.content, enabled, created_at: e.created_at, updated_at: None,
            revision: PromptRevision(catalog.revision.to_string()) })
    };
    let result = shared_import::import(q.import, |input| {
        if authorize()? != context { return Err((StatusCode::FORBIDDEN, "forbidden")); }
        let id = input.id.clone();
        let catalog = store.create_shared(scope, q.revision, StoredEntry { id: input.id, name: input.name,
            content: input.content, description: input.description, created_at: Some(input.timestamp_ms) })
            .map_err(|e| if e.to_string().contains("revision conflict") { (StatusCode::CONFLICT, "revision_conflict") } else { unavailable() })?;
        entry_from(catalog, &id, false)
    }, |activation| {
        let directory = if app == AppId::Pi {
            context.1.as_deref().map(|p| p.to_str().ok_or_else(invalid)).transpose()?
        } else { None };
        let environment = std::env::vars_os().filter_map(|(k,v)| Some((k.into_string().ok()?, v.into_string().ok()?))).collect();
        let preview = live_file::read(&q.target_host_id, &q.target_host_id, &context.0, app, directory, &environment)
            .map_err(|_| unavailable())?;
        if preview.snapshot.revision != activation.expected_target_revision {
            return Err((StatusCode::CONFLICT, "revision_conflict"));
        }
        // Existing owner reauthorizes and compares these exact bytes under its native target lock.
        let value = super::agent_app_prompts_activation::execute(policy.clone(), tenant.clone(), headers.clone(), db.clone(),
            super::agent_app_prompts_activation::Request { project_id: q.project_id.clone(), target_host_id: q.target_host_id.clone(),
                app_id: app, id: activation.entry.id.clone(), revision: activation.entry.expected_revision.0.parse().map_err(|_| invalid())?,
                expected_content: preview.snapshot.content }, true)?;
        entry_from(serde_json::from_value(value).map_err(|_| unavailable())?, &activation.entry.id, true)
    }).map_err(|e| match e { SharedImportError::InvalidInput(_) => invalid(), SharedImportError::Save(e) => e })?;
    Ok(match result {
        SharedImportResult::ImportedDisabled(entry) => ImportPromptShareResult::ImportedDisabled { entry },
        SharedImportResult::Activated(entry) => ImportPromptShareResult::Activated { entry },
        SharedImportResult::ImportedActivationFailed { entry, error: (status, code) } => ImportPromptShareResult::ImportedActivationFailed {
            entry, error: PromptShareActivationError { status: status.as_u16(), code: code.into() } },
    })
}
