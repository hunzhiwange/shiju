//! Native application provider records. Responses and errors never contain credentials.
use crate::{app_providers::AppProviderStore, global_database::GlobalDatabase};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use vw_api_types::{agent_apps::AppId, app_providers::AppProviderWrite, id::TenantScope};

pub(crate) fn router() -> Router {
    Router::new().merge(catalog::router()).route(
        "/app-providers/{app}/{id}",
        get(read).put(save).layer(DefaultBodyLimit::max(16 * 1024)),
    )
}
fn error(message: &str) -> Response {
    let (status, safe) = match message {
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
fn store(database: Option<Extension<GlobalDatabase>>) -> Result<AppProviderStore, String> {
    let database = match database {
        Some(Extension(database)) => database,
        None => GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| "application provider storage unavailable")?,
    };
    AppProviderStore::new(database)
}
async fn read(
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
async fn save(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
    body: Result<Json<AppProviderWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(profile)) = body else {
        return error("invalid input");
    };
    match tokio::task::spawn_blocking(move || store(database)?.save(&tenant, app, &id, &profile))
        .await
    {
        Ok(Ok(summary)) => Json(summary).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}

#[cfg(test)]
#[path = "app_providers/catalog_tests.rs"]
mod catalog_tests;

mod catalog;
