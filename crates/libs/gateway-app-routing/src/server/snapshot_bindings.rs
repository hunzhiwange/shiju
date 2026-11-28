//! Concrete aggregation publication and operation registry bindings.
use super::snapshot_host::SnapshotHost;
impl gateway_aggregation::publication_host::Host for SnapshotHost {
    #[cfg(target_os = "macos")]
    type Routing = super::lifecycle_api::RoutingControl;
    #[cfg(target_os = "macos")]
    fn provider_headers(&self, headers: axum::http::HeaderMap) -> Result<axum::http::HeaderMap, axum::http::StatusCode> {
        super::record_resync::provider_headers(headers)
    }
    #[cfg(target_os = "macos")]
    fn authorize_mode(&self, control: &Self::Routing, headers: &axum::http::HeaderMap,
        project: &vw_api_transport::ProjectScope, host: &str, app: vw_api_app_identity::AppId)
        -> Result<(std::path::PathBuf, String), axum::http::StatusCode> {
        control.authorize_mode(headers, project, host, app)
    }
    #[cfg(target_os = "macos")]
    fn sign(&self, bytes: &[u8]) -> Result<String, axum::http::StatusCode> {
        gateway_aggregation::aggregation_read_snapshot::sign(bytes)
    }
    #[cfg(target_os = "macos")]
    fn discovery(&self, policy: Option<&std::path::Path>, headers: &axum::http::HeaderMap,
        tenant: &vw_api_transport::TenantScope, host: &str)
        -> Result<(vw_agent_apps::apps::discovery::read::LocalReadEnvironment, String), axum::http::StatusCode> {
        gateway_agent_apps::host_context::provider_discovery_host(policy, headers, tenant, host)
    }
    #[cfg(target_os = "macos")]
    fn replace(&self, p: gateway_aggregation::publication_host::Publication,
        control: Option<Self::Routing>, key: &str, before: &str, after: &str)
        -> Result<bool, axum::http::StatusCode> {
        agent_gateway::aggregation_resync::RecordPublicationContext {
            database: p.database, project: p.project, host: p.host, app: p.app, home: p.home,
            root: p.root, routing: control.map(|c| c.routing), expected: p.expected,
            principal: p.principal, revision: p.revision, failure: Default::default(),
        }.replace(key, before, after)
    }

}
