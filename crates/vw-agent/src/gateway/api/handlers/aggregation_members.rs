//! Composition over the original routing listener and publication journal.
use axum::{Extension, Router};
use crate::global_database::GlobalDatabase;
use super::aggregation_codex_settings::SnapshotHost;
impl gateway_aggregation::publication_host::Host for SnapshotHost {
    #[cfg(target_os = "macos")]
    type Routing = super::app_routing::RoutingControl;
    #[cfg(not(target_os = "macos"))]
    type Routing = ();
    #[cfg(target_os = "macos")]
    fn provider_headers(&self, headers: axum::http::HeaderMap) -> Result<axum::http::HeaderMap, axum::http::StatusCode> {
        super::aggregation_resync::provider_headers(headers)
    }
    #[cfg(target_os = "macos")]
    fn authorize_mode(&self, control: &Self::Routing, headers: &axum::http::HeaderMap,
        project: &vw_api_transport::ProjectScope, host: &str, app: vw_api_app_identity::AppId)
        -> Result<(std::path::PathBuf, String), axum::http::StatusCode> {
        control.authorize_mode(headers, project, host, app)
    }
    #[cfg(target_os = "macos")]
    fn project(&self, app: vw_api_app_identity::AppId, tenant: &vw_api_transport::TenantScope,
        state: &vw_agent_apps::providers::live::commit::AppState, snapshot: &Self::Snapshot,
        request: Option<&vw_api_app_aggregation::AggregationPreviewRequest>)
        -> Result<vw_api_app_aggregation::AggregationPreview, axum::http::StatusCode> {
        super::aggregation_read_snapshot::project(app, tenant, state, snapshot, request)
    }
    #[cfg(target_os = "macos")]
    fn publish_candidate(&self, p: gateway_aggregation::publication_host::Publication,
        control: Self::Routing, candidate: &vw_agent_apps::providers::live::commit::AppState)
        -> Result<bool, axum::http::StatusCode> {
        crate::gateway::aggregation_resync::RecordPublicationContext {
            database: p.database, project: p.project, host: p.host, app: p.app, home: p.home,
            root: p.root, routing: Some(control.routing), expected: p.expected,
            principal: p.principal, revision: p.revision, failure: Default::default(),
        }.publish_candidate(candidate)
    }
    #[cfg(target_os = "macos")]
    fn sign(&self, bytes: &[u8]) -> Result<String, axum::http::StatusCode> {
        super::aggregation_read_snapshot::sign(bytes)
    }
    #[cfg(target_os = "macos")]
    fn discovery(&self, policy: Option<&std::path::Path>, headers: &axum::http::HeaderMap,
        tenant: &vw_api_transport::TenantScope, host: &str)
        -> Result<(vw_agent_apps::apps::discovery::read::LocalReadEnvironment, String), axum::http::StatusCode> {
        super::agent_apps::provider_discovery_host(policy, headers, tenant, host)
    }
    #[cfg(target_os = "macos")]
    fn replace(&self, p: gateway_aggregation::publication_host::Publication,
        control: Option<Self::Routing>, key: &str, before: &str, after: &str)
        -> Result<bool, axum::http::StatusCode> {
        crate::gateway::aggregation_resync::RecordPublicationContext {
            database: p.database, project: p.project, host: p.host, app: p.app, home: p.home,
            root: p.root, routing: control.map(|c| c.routing), expected: p.expected,
            principal: p.principal, revision: p.revision, failure: Default::default(),
        }.replace(key, before, after)
    }

}
pub(super) fn router() -> Router {
    let router = router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from));
    match GlobalDatabase::for_home(&crate::global::paths().home) {
        Ok(db) => router.layer(Extension(db)), Err(_) => router,
    }
}
pub(super) fn router_with_policy(path: Option<std::path::PathBuf>) -> Router {
    gateway_aggregation::aggregation_members::router_with_policy(path, SnapshotHost)
}
