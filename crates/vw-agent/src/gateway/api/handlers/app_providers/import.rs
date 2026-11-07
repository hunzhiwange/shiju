//! Authorized native catalog import, with no activation or client-supplied paths.
use super::*;
use axum::{http::HeaderMap, routing::post};
use std::path::PathBuf;
use vw_api_types::app_providers::import::*;
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/{app}/catalog/import", post(import))
        .layer(DefaultBodyLimit::max(4096))
        .layer(Extension(Policy(path)))
}
async fn import(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    body: Result<Json<ProviderImport>, JsonRejection>,
) -> Response {
    let Ok(Json(req)) = body else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let result = tokio::task::spawn_blocking(move || {
        // Require both capabilities before storage or native configuration is opened.
        let (home, _) = super::super::agent_apps::provider_editor_context(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &req.target_host_id,
            true,
        )?;
        super::super::agent_apps::provider_editor_context(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &req.target_host_id,
            false,
        )?;
        let unsupported = match app {
            AppId::ClaudeDesktop => Some(ProviderImportErrorCode::UseClaudeCompatibleImport),
            AppId::Dsh => Some(ProviderImportErrorCode::DshNativeProfileImportUnsupported),
            _ => None,
        };
        if let Some(code) = unsupported {
            return Ok((StatusCode::UNPROCESSABLE_ENTITY, Json(ProviderImportError { code }))
                .into_response());
        }
        #[cfg(target_os = "macos")]
        {
            store(database)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .import_native(&tenant, &home, app, req.trigger)
                .map(|r| Json(r).into_response())
                .map_err(|message| match message.as_str() {
                    "provider storage unavailable"
                    | "application provider storage unavailable"
                    | "invalid provider record"
                    | "invalid stored application provider profile" => {
                        StatusCode::SERVICE_UNAVAILABLE
                    }
                    "provider changed"
                    | "application provider changed during update"
                    | "application provider already exists" => StatusCode::CONFLICT,
                    _ => StatusCode::UNPROCESSABLE_ENTITY,
                })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (database, home, app, req);
            Err::<Response, _>(StatusCode::NOT_IMPLEMENTED)
        }
    })
    .await;
    match result {
        Ok(Ok(r)) => r,
        Ok(Err(s)) => s.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
