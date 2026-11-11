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
    let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
    let editor = editor_merge::Editor::new(policy.clone());
    Router::new()
        .merge(catalog::router())
        .merge(basic::router())
        .merge(mode_state::router())
        .merge(mode_apply::router())
        .merge(paths::router())
        .merge(env_diagnose::router())
        .merge(env_clean::router())
        .merge(env_restore::router())
        .merge(preview::router())
        .merge(editor.clone().router())
        .merge(snippets::router())
        .merge(claude::router())
        .merge(codex_catalog::router())
        .merge(gemini::router())
        .merge(grok::router())
        .merge(opencode::router())
        .merge(openclaw::router())
        .merge(hermes::router())
        .merge(pi::router())
        .merge(omp::router())
        .merge(mcode::router())
        .merge(kimi::router())
        .merge(dsh::router())
        .merge(agy::router())
        .merge(import::router())
        .merge(operations::router_with_editor(policy, editor))
        .merge(claude_desktop::router())
        .merge(desktop_mapping::router())
        .merge(delete::router())
        .merge(duplicate::router())
        .merge(presentation::router())
        .merge(endpoints::router())
        .merge(request_options::router())
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
    policy: Option<Extension<claude::Policy>>,
    headers: axum::http::HeaderMap,
    body: Result<
        Json<vw_api_types::app_providers::claude::ClaudeOr<AppProviderWrite>>,
        JsonRejection,
    >,
) -> Response {
    let Ok(Json(profile)) = body else {
        return error("invalid input");
    };
    let profile = match profile {
        vw_api_types::app_providers::claude::ClaudeOr::Claude(request) => {
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
            )
            .await;
        }
        vw_api_types::app_providers::claude::ClaudeOr::Legacy(profile) => profile,
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

#[path = "app_providers/paths.rs"]
mod paths;
#[cfg(test)]
#[path = "app_providers/paths_tests.rs"]
mod paths_tests;

mod preview;
#[cfg(test)]
#[path = "app_providers/preview_tests.rs"]
mod preview_tests;

#[cfg(test)]
#[path = "app_providers/validation_tests.rs"]
mod validation_tests;

mod editor_merge;
#[cfg(all(test, target_os = "macos"))]
mod editor_merge_tests;

#[path = "app_providers/snippets.rs"]
mod snippets;
#[cfg(all(test, target_os = "macos"))]
#[path = "app_providers/snippets_tests.rs"]
mod snippets_tests;

mod claude;
#[cfg(all(test, target_os = "macos"))]
mod claude_tests;

#[cfg(all(test, target_os = "macos"))]
mod codex_tests;

mod codex_catalog;
#[cfg(all(test, target_os = "macos"))]
mod codex_catalog_tests;

mod gemini;
#[cfg(all(test, target_os = "macos"))]
mod gemini_tests;

mod claude_desktop;
mod grok;
#[cfg(all(test, target_os = "macos"))]
mod grok_tests;

#[cfg(all(test, target_os = "macos"))]
mod claude_desktop_tests;

mod desktop_mapping;
#[cfg(all(test, target_os = "macos"))]
mod desktop_mapping_tests;

mod hermes;
#[cfg(all(test, target_os = "macos"))]
mod hermes_tests;
mod openclaw;
#[cfg(all(test, target_os = "macos"))]
mod openclaw_tests;
mod opencode;
#[cfg(all(test, target_os = "macos"))]
mod opencode_tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "aggregation_pi_tests.rs"]
mod aggregation_pi_tests;
mod pi;
#[cfg(all(test, target_os = "macos"))]
mod pi_tests;

pub(super) mod omp;
#[cfg(all(test, target_os = "macos"))]
mod omp_tests;

mod mcode;
#[cfg(all(test, target_os = "macos"))]
mod mcode_tests;

mod kimi;
#[cfg(all(test, target_os = "macos"))]
mod kimi_tests;

mod dsh;
#[cfg(all(test, target_os = "macos"))]
mod dsh_tests;

pub(super) mod agy;

mod import;
#[cfg(all(test, target_os = "macos"))]
mod import_tests;

mod mode_state;
#[cfg(all(test, target_os = "macos"))]
mod mode_state_tests;

pub(crate) mod mode_apply;
#[cfg(all(test, target_os = "macos"))]
mod mode_apply_tests;

mod env_diagnose;
#[cfg(all(test, target_os = "macos"))]
mod env_diagnose_tests;

mod env_clean;
#[cfg(all(test, target_os = "macos"))]
mod env_clean_tests;

mod env_restore;
#[cfg(test)]
mod env_restore_tests;

mod operations;
#[cfg(all(test, target_os = "macos"))]
mod operations_tests;

#[cfg(all(test, target_os = "macos"))]
mod api_security_tests;

mod basic;
#[cfg(test)]
mod basic_tests;

mod request_options;
#[cfg(all(test, target_os = "macos"))]
mod request_options_tests;
