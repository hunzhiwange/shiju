//! Pi templates on the authorized execution host; native variables remain opaque.
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::Deserialize;
use std::path::PathBuf;
use vw_api_types::{ProjectId, TenantScope};

#[derive(Clone)]
struct Policy(Option<PathBuf>);
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadRequest {
    project_id: ProjectId,
    target_host_id: String,
    #[serde(default = "pi_app")]
    app: String,
}
fn pi_app() -> String { "pi".into() }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplaceRequest {
    project_id: ProjectId,
    target_host_id: String,
    #[serde(default = "pi_app")]
    app: String,
    slug: String,
    original_slug: Option<String>,
    expected_revision: String,
    content: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeleteRequest {
    slug: String,
    project_id: ProjectId,
    target_host_id: String,
    #[serde(default = "pi_app")]
    app: String,
    expected_revision: String,
}
enum Operation { Read, Replace(String, Option<String>, String, String), Delete(String, String) }
type Failure = (StatusCode, &'static str);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/pi/prompt-templates/list", post(read))
        .route("/agent-apps/pi/prompt-templates/upsert", post(replace))
        .route("/agent-apps/pi/prompt-templates/delete", post(delete))
        // JSON escaping can expand a valid 1 MiB UTF-8 file by up to six times.
        .layer(DefaultBodyLimit::max(6 * 1024 * 1024 + 4096))
        .layer(Extension(Policy(policy)))
}
async fn read(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap,
    body: Result<Json<ReadRequest>, JsonRejection>) -> Response {
    let Ok(Json(q)) = body else { return error((StatusCode::BAD_REQUEST, "invalid_request")) };
    dispatch(p.0, t, h, q, Operation::Read).await
}
async fn replace(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap,
    body: Result<Json<ReplaceRequest>, JsonRejection>) -> Response {
    let Ok(Json(q)) = body else { return error((StatusCode::BAD_REQUEST, "invalid_request")) };
    dispatch(p.0, t, h, ReadRequest { project_id: q.project_id, target_host_id: q.target_host_id, app: q.app },
        Operation::Replace(q.slug, q.original_slug, q.expected_revision, q.content)).await
}
async fn delete(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap,
    body: Result<Json<DeleteRequest>, JsonRejection>) -> Response {
    let Ok(Json(q)) = body else { return error((StatusCode::BAD_REQUEST, "invalid_request")) };
    dispatch(p.0, t, h, ReadRequest { project_id: q.project_id, target_host_id: q.target_host_id, app: q.app },
        Operation::Delete(q.slug, q.expected_revision)).await
}
fn error((status, code): Failure) -> Response {
    use vw_api_types::error::{ApiErrorBody, ApiErrorDetail};
    (status, [(header::CACHE_CONTROL, "no-store")], Json(ApiErrorBody {
        error: ApiErrorDetail { code: code.into(), message: if code == "unsupported" { "Prompt templates are only supported for Pi; OMP templates are unavailable.".into() } else { code.into() }, details: None },
    })).into_response()
}
async fn dispatch(p: Option<PathBuf>, t: TenantScope, h: HeaderMap, q: ReadRequest, op: Operation) -> Response {
    match tokio::task::spawn_blocking(move || execute(p, t, h, q, op)).await {
        Ok(Ok(value)) => ([(header::CACHE_CONTROL, "no-store")], Json(value)).into_response(),
        Ok(Err(failure)) => error(failure),
        Err(_) => error((StatusCode::SERVICE_UNAVAILABLE, "target_unavailable")),
    }
}
#[cfg(not(target_os = "macos"))]
fn execute(_: Option<PathBuf>, _: TenantScope, _: HeaderMap, _: ReadRequest, _: Operation)
    -> Result<serde_json::Value, Failure> {
    Err((StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))
}
#[cfg(target_os = "macos")]
fn execute(p: Option<PathBuf>, t: TenantScope, h: HeaderMap, q: ReadRequest, op: Operation)
    -> Result<serde_json::Value, Failure> {
    use vw_agent_apps::{
        prompts::templates::{PiPromptTemplateService as Service, PiTemplateError},
        providers::{pi::PiFiles, live::{commit::{ExecutionHostId, ProviderTransactions}, write_guard::WriteError}},
    };
    use vw_api_types::{ProjectScope, agent_apps::AppId};
    let unavailable = || (StatusCode::SERVICE_UNAVAILABLE, "target_unavailable");
    let conflict = || (StatusCode::CONFLICT, "revision_conflict");
    let auth_error = |s| (s, if s == StatusCode::UNAUTHORIZED { "unauthenticated" } else { "forbidden" });
    let scope = ProjectScope::new(t, q.project_id).map_err(|_| (StatusCode::BAD_REQUEST, "invalid_request"))?;
    vw_agent_apps::providers::validate_identity(&q.target_host_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_request"))?;
    let write = !matches!(op, Operation::Read);
    let authorize = || super::agent_apps::authorize_pi_directory(p.as_deref(), &h, &scope, &q.target_host_id, write);
    let context = authorize().map_err(auth_error)?;
    let tx = ProviderTransactions::open_existing(
        context.credential_root.as_deref().ok_or_else(unavailable)?,
        ExecutionHostId::new(&q.target_host_id).map_err(|_| unavailable())?,
    ).map_err(|_| unavailable())?.ok_or_else(conflict)?;
    let state = tx.state(AppId::Pi).map_err(|_| unavailable())?;
    if state.project_scope.as_ref().is_some_and(|s| s != &scope) {
        return Err((StatusCode::FORBIDDEN, "forbidden"));
    }
    state.require_project(&scope).map_err(|_| conflict())?;
    if tx.pending(AppId::Pi).map_err(|_| unavailable())?.is_some() {
        return Err((StatusCode::CONFLICT, "recovery_pending"));
    }
    let preflight = || {
        let now = authorize().map_err(|_| WriteError::Conflict)?;
        if now.home != context.home || now.directory != context.directory
            || now.credential_root != context.credential_root || now.principal != context.principal
            || tx.state(AppId::Pi)? != state || tx.pending(AppId::Pi)?.is_some() {
            return Err(WriteError::Conflict);
        }
        Ok(())
    };
    if q.app != "pi" {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "unsupported"));
    }
    let env = std::env::var_os("PI_CODING_AGENT_DIR");
    let mut guard = PiFiles::open_prompt_templates(&context.home, context.directory.as_deref(), env.as_deref(), write)
        .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration"))?;
    let map = |e| match e {
        PiTemplateError::InvalidSlug => (StatusCode::BAD_REQUEST, "invalid_slug"),
        PiTemplateError::ContentTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "content_too_large"),
        PiTemplateError::InvalidUtf8 => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_utf8"),
        PiTemplateError::File(WriteError::Conflict | WriteError::Recovered) => conflict(),
        PiTemplateError::File(WriteError::InvalidResource) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration"),
        PiTemplateError::File(WriteError::Io) => unavailable(),
    };
    let value = match op {
        Operation::Read => {
            let values = Service::list(guard.as_ref()).map_err(map)?;
            preflight().map_err(|_| conflict())?;
            serde_json::Value::Array(values.into_iter().map(template_json).collect())
        },
        Operation::Replace(slug, original, revision, content) => template_json(Service::upsert_with_original(
            guard.as_mut().ok_or_else(conflict)?, &tx, scope.tenant_scope(), &slug, original.as_deref(),
            &revision, &content, preflight).map_err(map)?),
        Operation::Delete(slug, revision) => serde_json::json!({"deleted": Service::delete(
            guard.as_mut().ok_or_else(conflict)?, &tx, scope.tenant_scope(), &slug, &revision, preflight).map_err(map)?}),
    };
    Ok(value)
}
#[cfg(target_os = "macos")]
fn template_json(value: vw_agent_apps::prompts::templates::PiPromptTemplate) -> serde_json::Value {
    serde_json::json!({
        "slug": value.slug, "content": value.content, "revision": value.revision,
        "summary": {"description": value.summary.description, "argument_hint": value.summary.argument_hint}
    })
}
