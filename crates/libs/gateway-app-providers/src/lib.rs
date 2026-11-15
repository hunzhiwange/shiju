//! Application-provider HTTP routes. Host publication is injected at composition.
#![cfg(not(target_arch = "wasm32"))]
use app_provider_services::AppProviderStore;
use agent_database::GlobalDatabase;
use axum::{Extension, Json, Router, extract::{DefaultBodyLimit, Path, rejection::JsonRejection}, http::StatusCode, response::{IntoResponse, Response}, routing::get};
use vw_api_app_identity::AppId;
use vw_api_app_provider_base::AppProviderWrite;
use vw_api_transport::id::TenantScope;
mod authorization;
#[cfg(target_os = "macos")]
pub mod paths;
pub mod agy;
pub mod basic;
pub mod catalog;
pub mod claude;
pub mod claude_desktop;
pub mod codex_catalog;
pub mod create;
pub mod delete;
pub mod desktop_mapping;
pub mod dsh;
pub mod duplicate;
pub mod editor_merge;
pub mod endpoints;
pub mod env_clean;
pub mod env_diagnose;
pub mod env_restore;
pub mod gemini;
pub mod grok;
pub mod hermes;
pub mod icons;
pub mod import;
pub mod kimi;
pub mod mcode;
pub mod mode_state;
pub mod mode_apply;
pub mod omp;
pub mod openclaw;
pub mod opencode;
pub mod operations;
pub mod path_routes;
pub fn error(message: &str) -> Response {
    let (status, safe) = match message {
        "application provider is referenced" => {
            (StatusCode::CONFLICT, "Application provider is still referenced")
        }
        "application provider references unavailable" => {
            (StatusCode::CONFLICT, "Application provider references cannot be verified")
        }
        "application provider changed during update" => {
            (StatusCode::CONFLICT, "Application provider changed during update; retry")
        }
        "application provider already exists" => {
            (StatusCode::CONFLICT, "Application provider already exists")
        }
        "custom icon not found" => (StatusCode::NOT_FOUND, "Custom icon not found"),
        "application provider not found" => {
            (StatusCode::NOT_FOUND, "Application provider not found")
        }
        "application provider storage unavailable"
        | "invalid stored application provider profile" => {
            (StatusCode::SERVICE_UNAVAILABLE, "Application provider storage unavailable")
        }
        _ => (
            StatusCode::BAD_REQUEST,
            "Invalid application provider input or unsupported projection",
        ),
    };
    (status, Json(serde_json::json!({"message":safe}))).into_response()
}
pub fn store(database: Option<Extension<GlobalDatabase>>) -> Result<AppProviderStore, String> {
    let database = match database {
        Some(Extension(database)) => database,
        None => GlobalDatabase::for_home(&agent_paths::paths().home)
            .map_err(|_| "application provider storage unavailable")?,
    };
    AppProviderStore::new(database)
}

#[derive(Default)]
pub struct PublicationInput {
    pub query: vw_api_app_aggregation::AggregationWriteQuery,
    #[cfg(target_os = "macos")]
    pub routing: Option<PublicationControl>,
}
#[cfg(target_os = "macos")]
impl PublicationInput {
    pub fn headers(&self, headers: axum::http::HeaderMap) -> Result<axum::http::HeaderMap, StatusCode> {
        if self.query.project_id.is_some() || self.query.expected_revision.is_some() {
            self.routing.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0.headers(headers)
        } else { Ok(headers) }
    }
}
/// Existing host publication operation, with strongly typed tenant and provider inputs.
#[cfg(target_os = "macos")]
pub trait ProviderPublication: Send + Sync {
    fn headers(&self, headers: axum::http::HeaderMap) -> Result<axum::http::HeaderMap, StatusCode>;
    #[allow(clippy::too_many_arguments)]
    fn configure(&self, store: AppProviderStore, database: &GlobalDatabase,
        policy: Option<&std::path::Path>, headers: &axum::http::HeaderMap,
        tenant: &TenantScope, host: &str, app: AppId,
        query: vw_api_app_aggregation::AggregationWriteQuery) -> Result<AppProviderStore, StatusCode>;
}
#[cfg(target_os = "macos")]
#[derive(Clone)]
pub struct PublicationControl(pub std::sync::Arc<dyn ProviderPublication>);
#[cfg(target_os = "macos")]
pub fn publication_store(store: AppProviderStore, policy: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap, tenant: &TenantScope, host: &str, app: AppId,
    input: PublicationInput) -> Result<AppProviderStore, StatusCode> {
    let database = store.database();
    input.routing.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0.configure(store, &database,
        policy, headers, tenant, host, app, input.query)
}

/// Provider-record GET, PUT and POST routes, including the legacy save API.
/// Compose PATCH and specialized routes separately. Native scoped writes consume
/// the host's `PublicationControl` extension on macOS.
pub fn router() -> Router {
    Router::new().route("/app-providers/{app}/{id}",
        get(read).put(save).post(create::create)
            .layer(DefaultBodyLimit::max(16 * 1024)))
}

pub async fn read(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
) -> Response {
    match tokio::task::spawn_blocking(move || store(database)?.get_summary(&tenant, app, &id)).await
    {
        Ok(Ok(summary)) => Json(summary).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
#[allow(clippy::too_many_arguments)]
pub async fn save(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
    policy: Option<Extension<claude::Policy>>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(query): axum::extract::Query<
        vw_api_app_aggregation::AggregationWriteQuery,
    >,
    #[cfg(target_os = "macos")] control: Option<Extension<PublicationControl>>,
    body: Result<
        Json<vw_api_app_provider_base::claude::ClaudeOr<AppProviderWrite>>,
        JsonRejection,
    >,
) -> Response {
    let Ok(Json(profile)) = body else {
        return error("invalid input");
    };
    let profile = match profile {
        vw_api_app_provider_base::claude::ClaudeOr::Claude(request) => {
            if app != AppId::Claude {
                return StatusCode::BAD_REQUEST.into_response();
            }
            return claude::write(
                tenant,
                database,
                headers,
                policy,
                id,
                request,
                claude::WriteKind::Save,
                PublicationInput {
                    query,
                    #[cfg(target_os = "macos")]
                    routing: control.map(|Extension(c)| c),
                },
            )
            .await;
        }
        vw_api_app_provider_base::claude::ClaudeOr::Legacy(profile) => profile,
    };
    match tokio::task::spawn_blocking(move || store(database)?.save(&tenant, app, &id, &profile))
        .await
    {
        Ok(Ok(summary)) => Json(summary).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
