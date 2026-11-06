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
    Router::new()
        .merge(catalog::router())
        .merge(delete::router())
        .merge(duplicate::router())
        .merge(presentation::router())
        .merge(endpoints::router())
        .merge(icons::router())
        .merge(universal::router())
        .merge(universal_sync::router())
        .route(
            "/app-providers/{app}/{id}",
            get(read)
                .put(save)
                .post(create::create)
                .patch(update::update)
                .layer(DefaultBodyLimit::max(16 * 1024)),
        )
}
fn error(message: &str) -> Response {
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

mod create;
#[cfg(test)]
#[path = "app_providers/create_tests.rs"]
mod create_tests;

mod update;
#[cfg(test)]
#[path = "app_providers/update_tests.rs"]
mod update_tests;

mod delete;
#[cfg(all(test, target_os = "macos"))]
#[path = "app_providers/delete_tests.rs"]
mod delete_tests;

#[path = "app_providers/duplicate.rs"]
mod duplicate;
#[cfg(test)]
#[path = "app_providers/duplicate_tests.rs"]
mod duplicate_tests;

#[path = "app_providers/presentation.rs"]
mod presentation;
#[cfg(test)]
#[path = "app_providers/presentation_tests.rs"]
mod presentation_tests;

#[path = "app_providers/endpoints.rs"]
mod endpoints;
#[cfg(test)]
#[path = "app_providers/endpoints_tests.rs"]
mod endpoints_tests;

#[path = "app_providers/icons.rs"]
mod icons;
#[cfg(test)]
#[path = "app_providers/icons_tests.rs"]
mod icons_tests;

mod universal;
#[cfg(test)]
#[path = "app_providers/universal_tests.rs"]
mod universal_tests;

mod universal_sync;
#[cfg(test)]
#[path = "app_providers/universal_sync_tests.rs"]
mod universal_sync_tests;
