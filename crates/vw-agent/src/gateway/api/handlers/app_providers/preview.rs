//! Explicit read permission is checked before resolving any host path.
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use std::path::PathBuf;
use vw_api_types::{
    agent_apps::AppId, app_providers::preview::ProviderPreviewRequest, id::TenantScope,
};
#[derive(Clone)]
struct PolicyFile(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route(
            "/app-providers/{app}/config/preview",
            post(read).layer(DefaultBodyLimit::max(256 * 1024)),
        )
        .layer(Extension(PolicyFile(path)))
}
async fn read(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    database: Option<Extension<crate::global_database::GlobalDatabase>>,
    Path(app): Path<AppId>,
    body: Result<Json<ProviderPreviewRequest>, JsonRejection>,
) -> Result<Response, StatusCode> {
    let Json(mut request) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        let (home, private_root) = super::super::agent_apps::provider_preview_context(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &request.target_host_id,
        )?;
        let owner = owner(
            private_root.as_deref(),
            &request.target_host_id,
            app,
            &tenant,
            database.clone(),
        )?;
        if let Some(id) = request.provider_id.as_deref() {
            let record = crate::app_providers::AppProviderStore::preview_record(
                &database.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0,
                &tenant,
                app,
                id,
            )
            .map_err(|message| match message.as_str() {
                "application provider not found" => StatusCode::NOT_FOUND,
                "application provider storage unavailable"
                | "invalid stored application provider profile" => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::BAD_REQUEST,
            })?;
            if request.settings.is_null() {
                let Some(settings) = record.get("settingsConfig").filter(|v| v.is_object()) else {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                };
                request.settings = settings.clone();
            }
            if !request.settings.is_object() {
                return Err(StatusCode::BAD_REQUEST);
            }
            request.category = vw_agent_apps::providers::preview::editor_category(
                app,
                id,
                &record,
                request.category.as_deref(),
            );
            // Explicit empty draft metadata clears stored metadata, matching the source hook.
            if request.meta.is_none() {
                request.meta = record.get("meta").filter(|v| !v.is_null()).cloned();
            }
            if let Some(options) = record.get("requestOptions") {
                let saved = serde_json::from_value(options.clone())
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                // Validate saved fields even if this preview supplies a partial override.
                vw_agent_apps::providers::validation::validate(app, &saved)
                    .map_err(|_| StatusCode::BAD_REQUEST)?;
                request.request_options = Some(match &request.request_options {
                    Some(incoming) => {
                        vw_agent_apps::providers::validation::merge(app, Some(&saved), incoming)
                            .map_err(|_| StatusCode::BAD_REQUEST)?
                    }
                    None => saved,
                });
            }
        }
        vw_agent_apps::providers::preview::read(&home, app, &request, &owner)
            .map(|value| Json(value).into_response())
            .map_err(|_| StatusCode::BAD_REQUEST)
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

#[cfg(target_os = "macos")]
fn owner(
    root: Option<&std::path::Path>,
    host: &str,
    app: AppId,
    tenant: &TenantScope,
    database: Option<Extension<crate::global_database::GlobalDatabase>>,
) -> Result<vw_agent_apps::providers::preview::PreviewOwner, StatusCode> {
    use vw_agent_apps::providers::{
        live::commit::{ExecutionHostId, ProviderTransactions},
        preview::PreviewOwner,
    };
    use vw_api_types::app_providers::preview::PreviewUnavailable;
    let mut result = PreviewOwner::default();
    let Some(root) = root else {
        return Ok(result);
    };
    let Some(transactions) = ProviderTransactions::open_existing(
        root,
        ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?,
    )
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    else {
        return Ok(result);
    };
    if transactions.pending(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.is_some() {
        result.unavailable = Some(PreviewUnavailable::PendingTransaction);
        return Ok(result);
    }
    let state = transactions.state(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if let Some(mode) = state.mode.filter(|m| m.attached) {
        if state.mode_tenant.as_ref() != Some(tenant) {
            result.unavailable = Some(PreviewUnavailable::Ownership);
        } else {
            result.exclusive =
                Some(mode.contract.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.exclusive);
        }
    } else if let Some(pointer) = state.pointer {
        if pointer.tenant != *tenant {
            result.unavailable = Some(PreviewUnavailable::Ownership);
        } else {
            // Only open tenant storage when there is an actual native owner to resolve.
            match crate::app_providers::AppProviderStore::preview_record(
                &database.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0,
                tenant,
                app,
                &pointer.id,
            ) {
                Ok(record) => match record.get("settingsConfig").filter(|v| v.is_object()) {
                    Some(settings) => result.settings = Some(settings.clone()),
                    None => result.unavailable = Some(PreviewUnavailable::Ownership),
                },
                Err(_) => result.unavailable = Some(PreviewUnavailable::Ownership),
            }
        }
    }
    Ok(result)
}
#[cfg(not(target_os = "macos"))]
fn owner(
    _root: Option<&std::path::Path>,
    _host: &str,
    _app: AppId,
    _tenant: &TenantScope,
    _database: Option<Extension<crate::global_database::GlobalDatabase>>,
) -> Result<vw_agent_apps::providers::preview::PreviewOwner, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}
