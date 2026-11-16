//! Native application provider records. Responses and errors never contain credentials.
use crate::{app_providers::AppProviderStore, global_database::GlobalDatabase};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use vw_api_app_identity::AppId;
use vw_api_app_provider_base::AppProviderWrite;
use vw_api_transport::id::TenantScope;

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
use gateway_app_providers::{error, store};

use gateway_app_providers::read;

#[allow(clippy::too_many_arguments)]
async fn save(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
    policy: Option<Extension<claude::Policy>>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(query): axum::extract::Query<
        vw_api_app_aggregation::AggregationWriteQuery,
    >,
    #[cfg(target_os = "macos")] control: Option<Extension<super::app_routing::RoutingControl>>,
    body: Result<
        Json<vw_api_app_provider_base::claude::ClaudeOr<AppProviderWrite>>,
        JsonRejection,
    >,
) -> Response {
    gateway_app_providers::save(Extension(tenant), database, Path((app, id)), policy,
        headers, axum::extract::Query(query),
        #[cfg(target_os = "macos")]
        Some(Extension(publication_host::control(control.map(|Extension(c)| c)))), body).await
}

#[cfg(test)]
#[path = "app_providers/catalog_tests.rs"]
mod catalog_tests;

mod catalog;

mod create;

mod update;
#[cfg(test)]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/update_tests.rs"]
mod update_tests;

mod delete;
#[cfg(all(test, target_os = "macos"))]
#[path = "app_providers/delete_tests.rs"]
mod delete_tests;

#[path = "app_providers/duplicate.rs"]
mod duplicate;
#[cfg(test)]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/duplicate_tests.rs"]
mod duplicate_tests;

#[path = "app_providers/presentation.rs"]
mod presentation;
#[cfg(test)]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/presentation_tests.rs"]
mod presentation_tests;

#[path = "app_providers/endpoints.rs"]
mod endpoints;
#[cfg(test)]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/endpoints_tests.rs"]
mod endpoints_tests;

#[path = "app_providers/icons.rs"]
mod icons;
#[cfg(test)]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/icons_tests.rs"]
mod icons_tests;

mod universal;
#[cfg(test)]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/universal_tests.rs"]
mod universal_tests;

mod universal_sync;
#[cfg(test)]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/universal_sync_tests.rs"]
mod universal_sync_tests;

#[path = "app_providers/paths.rs"]
mod paths;
#[cfg(test)]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/paths_tests.rs"]
mod paths_tests;

mod preview;
#[cfg(test)]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/preview_tests.rs"]
mod preview_tests;

#[cfg(test)]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/validation_tests.rs"]
mod validation_tests;

mod editor_merge;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/editor_merge_tests.rs"]
mod editor_merge_tests;

#[path = "app_providers/snippets.rs"]
mod snippets;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/snippets_tests.rs"]
mod snippets_tests;

pub(super) mod claude;
#[cfg(all(test, target_os = "macos"))]
mod claude_tests;

pub(super) mod codex_catalog;

mod gemini;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/gemini_tests.rs"]
mod gemini_tests;

mod claude_desktop;
mod grok;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/grok_tests.rs"]
mod grok_tests;

pub(super) mod desktop_mapping;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/desktop_mapping_tests.rs"]
mod desktop_mapping_tests;

mod hermes;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/hermes_tests.rs"]
mod hermes_tests;
mod openclaw;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/openclaw_tests.rs"]
mod openclaw_tests;
mod opencode;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/opencode_tests.rs"]
mod opencode_tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "aggregation_pi_tests.rs"]
mod aggregation_pi_tests;
mod pi;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/pi_tests.rs"]
mod pi_tests;

pub(super) mod omp;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/omp_tests.rs"]
mod omp_tests;

mod mcode;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/mcode_tests.rs"]
mod mcode_tests;

mod kimi;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/kimi_tests.rs"]
mod kimi_tests;

mod dsh;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/dsh_tests.rs"]
mod dsh_tests;

pub(super) mod agy;

mod import;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/import_tests.rs"]
mod import_tests;

mod mode_state;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/mode_state_tests.rs"]
mod mode_state_tests;

pub(crate) mod mode_apply;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/mode_apply_tests.rs"]
mod mode_apply_tests;

mod env_diagnose;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/env_diagnose_tests.rs"]
mod env_diagnose_tests;

mod env_clean;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/env_clean_tests.rs"]
mod env_clean_tests;

mod env_restore;
#[cfg(test)]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/env_restore_tests.rs"]
mod env_restore_tests;

pub(super) mod operations;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/operations_tests.rs"]
mod operations_tests;

#[cfg(all(test, target_os = "macos"))]
mod api_security_tests;

mod basic;

mod request_options;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-app-providers/src/host_tests/request_options_tests.rs"]
mod request_options_tests;

#[derive(Default)]
struct PublicationInput {
    query: vw_api_app_aggregation::AggregationWriteQuery,
    #[cfg(target_os = "macos")]
    routing: Option<super::app_routing::RoutingControl>,
}
#[cfg(target_os = "macos")]
impl PublicationInput {
    fn headers(&self, headers: axum::http::HeaderMap) -> Result<axum::http::HeaderMap, StatusCode> {
        gateway_app_providers::PublicationInput { query: self.query.clone(),
            routing: Some(publication_host::control(self.routing.clone())) }.headers(headers)
    }
}
#[cfg(target_os = "macos")]
fn publication_store(
    store: AppProviderStore,
    policy: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    host: &str,
    app: AppId,
    input: PublicationInput,
) -> Result<AppProviderStore, StatusCode> {
    gateway_app_providers::publication_store(store, policy, headers, tenant, host, app,
        publication_host::input(input))
}

/// Reapply saved current selections through the existing owner authorization and
/// native journal. This never activates an inactive record or consumes remote JSON.
#[cfg(target_os = "macos")]
pub(super) async fn sync_saved_provider(database: GlobalDatabase, policy: Option<std::path::PathBuf>,
    headers: axum::http::HeaderMap, target: &vw_api_app_operations::DataManagementTarget,
    app: vw_api_app_identity::AppId, id: &str,
) -> Result<(), ()> {
    gateway_app_providers::sync_saved_provider(
        database, policy, headers, target, app, id, publication_host::control(None),
    ).await
}

#[cfg(target_os = "macos")]
mod publication_host;
