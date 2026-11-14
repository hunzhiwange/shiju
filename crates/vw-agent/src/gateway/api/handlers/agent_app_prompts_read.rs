//! Read-only native prompt endpoints; catalog initialization imports inert data once.
use axum::{
    Extension, Json, Router,
    extract::{Query, rejection::QueryRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use std::path::PathBuf;
use vw_api_types::{ProjectId, TenantScope, agent_apps::AppId};
use crate::global_database::GlobalDatabase;

#[derive(Clone)]
struct Policy(Option<PathBuf>);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadQuery {
    project_id: ProjectId,
    target_host_id: String,
    app_id: AppId,
    #[serde(default)]
    query: String,
}
#[derive(Clone, Copy)]
enum Operation { Catalog, Current, Location }
type Failure = (StatusCode, &'static str);

pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/prompts", get(read).layer(Extension(Operation::Catalog)))
        .route("/agent-apps/prompts/current", get(read).layer(Extension(Operation::Current)))
        .route("/agent-apps/prompts/location", get(read).layer(Extension(Operation::Location)))
        .layer(Extension(Policy(policy)))
}
async fn read(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    Extension(operation): Extension<Operation>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<ReadQuery>, QueryRejection>,
) -> Response {
    let Ok(Query(query)) = query else {
        return error((StatusCode::BAD_REQUEST, "invalid_request"));
    };
    match tokio::task::spawn_blocking(move || execute(
        policy.0, tenant, database.map(|Extension(db)| db), headers, query, operation,
    )).await {
        Ok(Ok(value)) => ([(header::CACHE_CONTROL, "no-store")], Json(value)).into_response(),
        Ok(Err(failure)) => error(failure),
        Err(_) => error((StatusCode::SERVICE_UNAVAILABLE, "target_unavailable")),
    }
}

fn error((status, code): Failure) -> Response {
    use vw_api_types::error::{ApiErrorBody, ApiErrorDetail};
    (status, [(header::CACHE_CONTROL, "no-store")], Json(ApiErrorBody {
        error: ApiErrorDetail { code: code.into(), message: code.into(), details: None },
    })).into_response()
}
#[cfg(not(target_os = "macos"))]
fn execute(_: Option<PathBuf>, _: TenantScope, _: Option<GlobalDatabase>, _: HeaderMap,
    _: ReadQuery, _: Operation) -> Result<serde_json::Value, Failure> {
    Err((StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))
}
#[cfg(target_os = "macos")]
fn execute(policy: Option<PathBuf>, tenant: TenantScope, database: Option<GlobalDatabase>,
    headers: HeaderMap, query: ReadQuery, operation: Operation) -> Result<serde_json::Value, Failure> {
    use crate::agent_apps::prompts_store::{PromptsStore, PromptScope};
    use vw_agent_apps::prompts::{live_file, targets, listing::list_entries};
    use vw_api_types::{ProjectScope, agent_apps::PromptResource};
    let unavailable = || (StatusCode::SERVICE_UNAVAILABLE, "target_unavailable");
    let conflict = || (StatusCode::CONFLICT, "revision_conflict");
    let project = ProjectScope::new(tenant, query.project_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_request"))?;
    vw_agent_apps::providers::validate_identity(&query.target_host_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_request"))?;
    PromptResource::Application { app_id: query.app_id }.validate_application_access()
        .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))?;
    let authorize = || super::agent_apps::authorize_prompts_read(
        policy.as_deref(), &headers, &project, &query.target_host_id, query.app_id,
    );
    let context = authorize().map_err(|status| (status,
        if status == StatusCode::UNAUTHORIZED { "unauthenticated" } else { "forbidden" }))?;
    let reauthorize = || {
        let now = authorize().map_err(|_| conflict())?;
        if now != context { return Err(conflict()); }
        Ok(())
    };
    // Only administrator-selected Pi paths and executor environment are used.
    let override_dir = if query.app_id == AppId::Pi {
        context.1.as_deref().map(|p| p.to_str().ok_or((StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration"))).transpose()?
    } else { None };
    let environment = std::env::vars_os().filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?))).collect();
    let value = match operation {
        Operation::Catalog => {
            let store = PromptsStore::new(database.ok_or_else(unavailable)?).map_err(|_| unavailable())?;
            let mut catalog = store.list_on_first_access(
                PromptScope { project: &project, host: &query.target_host_id, app: query.app_id },
                &query.target_host_id, &context.0, override_dir, &environment,
                || reauthorize().map_err(|_| anyhow::anyhow!("prompt authorization changed")),
            ).map_err(|e| {
                if e.chain().any(|cause| cause.to_string().contains("revision conflict")) { conflict() }
                else { (StatusCode::SERVICE_UNAVAILABLE, "prompt_initialization_failed") }
            })?;
            catalog.entries = list_entries(catalog.entries, &query.query);
            serde_json::to_value(catalog)
        }
        Operation::Current => serde_json::to_value(live_file::read(
            &query.target_host_id, &query.target_host_id, &context.0, query.app_id,
            override_dir, &environment,
        ).map_err(|e| match e {
            live_file::LiveFileError::ChangedDuringRead => conflict(),
            live_file::LiveFileError::InvalidUtf8 => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_utf8"),
            live_file::LiveFileError::UnsafePath => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration"),
            _ => unavailable(),
        })?),
        Operation::Location => {
            let target = targets::resolve(&context.0, query.app_id, override_dir, &environment)
                .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration"))?;
            let relative = target.file.strip_prefix(&context.0)
                .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration"))?;
            Ok(serde_json::json!({"resource": PromptResource::Application { app_id: query.app_id },
                "displayPath": format!("~/{}", relative.display()), "writable": false}))
        }
    }.map_err(|_| unavailable())?;
    reauthorize()?;
    Ok(value)
}
