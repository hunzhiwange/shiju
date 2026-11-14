//! Revision-aware activation; paths and transaction roots come from host policy.
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::Deserialize;
use std::path::PathBuf;
use vw_api_types::{ProjectId, TenantScope, agent_apps::AppId};
#[derive(Clone)]
struct Policy(Option<PathBuf>);
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    pub(super) project_id: ProjectId,
    pub(super) target_host_id: String,
    pub(super) app_id: AppId,
    pub(super) id: String,
    pub(super) revision: u64,
    /// Exact preview content, including absent versus empty; never substituted.
    pub(super) expected_content: Option<String>,
}
type Failure = (StatusCode, &'static str);
pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/prompts/activate", post(activate))
        .route("/agent-apps/prompts/deactivate", post(deactivate))
        .layer(DefaultBodyLimit::max(6 * 1024 * 1024 + 4096))
        .layer(Extension(Policy(
            std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from),
        )))
}
fn error((status, code): Failure) -> Response {
    use vw_api_types::error::{ApiErrorBody, ApiErrorDetail};
    (
        status,
        [(header::CACHE_CONTROL, "no-store")],
        Json(ApiErrorBody {
            error: ApiErrorDetail { code: code.into(), message: code.into(), details: None },
        }),
    )
        .into_response()
}
macro_rules! handler {
    ($name:ident, $enabled:expr) => {
        async fn $name(
            Extension(p): Extension<Policy>,
            Extension(t): Extension<TenantScope>,
            database: Option<Extension<GlobalDatabase>>,
            h: HeaderMap,
            body: Result<Json<Request>, JsonRejection>,
        ) -> Response {
            let Ok(Json(q)) = body else {
                return error((StatusCode::BAD_REQUEST, "invalid_request"));
            };
            let Some(Extension(db)) = database else {
                return error((StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable"));
            };
            match tokio::task::spawn_blocking(move || execute(p.0, t, h, db, q, $enabled)).await {
                Ok(Ok(value)) => {
                    ([(header::CACHE_CONTROL, "no-store")], Json(value)).into_response()
                }
                Ok(Err(e)) => error(e),
                Err(_) => error((StatusCode::SERVICE_UNAVAILABLE, "target_unavailable")),
            }
        }
    };
}
handler!(activate, true);
handler!(deactivate, false);
#[cfg(not(target_os = "macos"))]
pub(super) fn execute(
    _: Option<PathBuf>,
    _: TenantScope,
    _: HeaderMap,
    _: GlobalDatabase,
    _: Request,
    _: bool,
) -> Result<serde_json::Value, Failure> {
    Err((StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))
}
#[cfg(target_os = "macos")]
pub(super) fn execute(
    policy: Option<PathBuf>,
    tenant: TenantScope,
    headers: HeaderMap,
    db: GlobalDatabase,
    q: Request,
    enabled: bool,
) -> Result<serde_json::Value, Failure> {
    use crate::agent_apps::prompts_store::{PromptScope, PromptsStore};
    use vw_agent_apps::providers::live::{
        commit::{ExecutionHostId, ProviderTransactions},
        write_guard::WriteError,
    };
    use vw_api_types::{ProjectScope, agent_apps::PromptResource};
    let unavailable = || (StatusCode::SERVICE_UNAVAILABLE, "target_unavailable");
    let conflict = || (StatusCode::CONFLICT, "revision_conflict");
    let project = ProjectScope::new(tenant, q.project_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_request"))?;
    vw_agent_apps::providers::validate_identity(&q.target_host_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_host"))?;
    vw_agent_apps::providers::validate_identity(&q.id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_request"))?;
    PromptResource::Application { app_id: q.app_id }
        .validate_application_access()
        .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))?;
    let authorize = || {
        super::agent_apps::authorize_prompts_activation(
            policy.as_deref(),
            &headers,
            &project,
            &q.target_host_id,
            q.app_id,
        )
    };
    let context = authorize().map_err(|s| {
        (s, if s == StatusCode::UNAUTHORIZED { "unauthenticated" } else { "forbidden" })
    })?;
    let tx = ProviderTransactions::open_existing(
        &context.3,
        ExecutionHostId::new(&q.target_host_id).map_err(|_| unavailable())?,
    )
    .map_err(|_| unavailable())?
    .ok_or_else(conflict)?;
    let state = tx.state(q.app_id).map_err(|_| unavailable())?;
    if state.project_scope.as_ref().is_some_and(|s| s != &project) {
        return Err((StatusCode::FORBIDDEN, "forbidden"));
    }
    state.require_project(&project).map_err(|_| conflict())?;
    let environment = std::env::vars_os()
        .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
        .collect();
    let directory = if q.app_id == AppId::Pi {
        context
            .1
            .as_deref()
            .map(|p| p.to_str().ok_or((StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration")))
            .transpose()?
    } else {
        None
    };
    let store = PromptsStore::new(db).map_err(|_| unavailable())?;
    let result = store
        .toggle_native(
            PromptScope { project: &project, host: &q.target_host_id, app: q.app_id },
            q.revision,
            &q.id,
            enabled,
            q.expected_content.as_deref(),
            &context.0,
            directory,
            &environment,
            &tx,
            || {
                if authorize().map_err(|_| WriteError::Conflict)? != context {
                    return Err(WriteError::Conflict);
                }
                Ok(())
            },
        )
        .map_err(|e| {
            use vw_agent_apps::prompts::activation::ActivationError;
            let text = e.to_string();
            if text == "prompt not found" {
                (StatusCode::NOT_FOUND, "not_found")
            } else if text.contains("conflict")
                || text.contains("Conflict")
                || text.contains("Recovered")
                || e.downcast_ref::<ActivationError>().is_some_and(|e| {
                    matches!(
                        e,
                        ActivationError::File(WriteError::Conflict | WriteError::Recovered)
                            | ActivationError::Store
                    )
                })
            {
                conflict()
            } else if text == "prompt content too large" {
                (StatusCode::PAYLOAD_TOO_LARGE, "content_too_large")
            } else if text == "prompt recovery pending" {
                (StatusCode::CONFLICT, "recovery_pending")
            } else {
                unavailable()
            }
        })?;
    serde_json::to_value(result).map_err(|_| unavailable())
}
