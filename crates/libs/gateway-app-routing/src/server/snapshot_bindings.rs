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
    fn project(&self, app: vw_api_app_identity::AppId, tenant: &vw_api_transport::TenantScope,
        state: &vw_agent_apps::providers::live::commit::AppState, snapshot: &Self::Snapshot,
        request: Option<&vw_api_app_aggregation::AggregationPreviewRequest>)
        -> Result<vw_api_app_aggregation::AggregationPreview, axum::http::StatusCode> {
        gateway_aggregation::aggregation_read_snapshot::project(app, tenant, state, snapshot, request)
    }
    #[cfg(target_os = "macos")]
    fn publish_candidate(&self, p: gateway_aggregation::publication_host::Publication,
        control: Self::Routing, candidate: &vw_agent_apps::providers::live::commit::AppState)
        -> Result<bool, axum::http::StatusCode> {
        agent_gateway::aggregation_resync::RecordPublicationContext {
            database: p.database, project: p.project, host: p.host, app: p.app, home: p.home,
            root: p.root, routing: Some(control.routing), expected: p.expected,
            principal: p.principal, revision: p.revision, failure: Default::default(),
        }.publish_candidate(candidate)
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
impl gateway_aggregation::publish_host::PublishHost for SnapshotHost {
    type State = gateway_app_providers::operations::State;
    #[cfg(target_os = "macos")]
    type Context = agent_gateway::aggregation_resync::RecordPublicationContext;
    #[cfg(target_os = "macos")]
    fn policy<'a>(&self, state: &'a Self::State) -> Option<&'a std::path::Path> { state.aggregation_policy() }
    #[cfg(target_os = "macos")]
    fn replay(&self, state: &Self::State, o: &gateway_aggregation::publish_host::Owner, request: &str, fingerprint: &[u8;32])
        -> Result<Option<serde_json::Value>, axum::http::StatusCode> {
        state.aggregation_replay(&gateway_app_providers::operations::aggregation::Owner {
            project: o.project.clone(), host: o.host.clone(), app: o.app, principal: o.principal.clone(),
        }, request, fingerprint)
    }
    #[cfg(target_os = "macos")]
    fn start(&self, state: &Self::State, o: gateway_aggregation::publish_host::Owner, request: String, fingerprint: [u8;32],
        work: impl FnOnce() -> vw_api_app_aggregation::AggregationOutcome + Send + 'static)
        -> Result<serde_json::Value, axum::http::StatusCode> {
        state.aggregation_start(gateway_app_providers::operations::aggregation::Owner {
            project: o.project, host: o.host, app: o.app, principal: o.principal,
        }, request, fingerprint, work)
    }
    #[cfg(target_os = "macos")]
    fn database(&self) -> Result<agent_database::GlobalDatabase, axum::http::StatusCode> {
        agent_database::GlobalDatabase::for_home(&agent_paths::paths().home)
            .map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)
    }
    #[cfg(target_os = "macos")]
    fn publication_context(&self, p: gateway_aggregation::publication_host::Publication, control: Self::Routing) -> Self::Context {
        agent_gateway::aggregation_resync::RecordPublicationContext {
            database: p.database, project: p.project, host: p.host, app: p.app, home: p.home,
            root: p.root, routing: Some(control.routing), expected: p.expected,
            principal: p.principal, revision: p.revision, failure: Default::default(),
        }
    }
}
impl gateway_aggregation::restart_host::RestartHost for SnapshotHost {
    type State = gateway_app_providers::operations::State;
    #[cfg(target_os = "macos")]
    fn policy<'a>(&self, state: &'a Self::State) -> Option<&'a std::path::Path> { state.aggregation_policy() }
    #[cfg(target_os = "macos")]
    fn replay(&self, state: &Self::State, o: &gateway_aggregation::restart_host::Owner, request: &str, fingerprint: &[u8;32])
        -> Result<Option<serde_json::Value>, axum::http::StatusCode> {
        state.aggregation_restart_replay(&gateway_app_providers::operations::aggregation::Owner {
            project: o.project.clone(), host: o.host.clone(), app: o.app, principal: o.principal.clone(),
        }, request, fingerprint)
    }
    #[cfg(target_os = "macos")]
    fn start(&self, state: &Self::State, o: gateway_aggregation::restart_host::Owner, request: String, fingerprint: [u8;32],
        work: impl FnOnce() -> vw_api_app_aggregation::AggregationOutcome + Send + 'static)
        -> Result<serde_json::Value, axum::http::StatusCode> {
        state.aggregation_restart_start(gateway_app_providers::operations::aggregation::Owner {
            project: o.project, host: o.host, app: o.app, principal: o.principal,
        }, request, fingerprint, work)
    }
}
impl gateway_aggregation::exit_host::ExitHost for SnapshotHost {
    type State = gateway_app_providers::operations::State;
    #[cfg(target_os = "macos")]
    type Context = agent_gateway::aggregation_resync::RecordPublicationContext;
    #[cfg(target_os = "macos")]
    fn policy<'a>(&self, state: &'a Self::State) -> Option<&'a std::path::Path> { state.aggregation_policy() }
    #[cfg(target_os = "macos")]
    fn replay(&self, state: &Self::State, o: &gateway_aggregation::exit_host::Owner, request: &str, fingerprint: &[u8;32])
        -> Result<Option<serde_json::Value>, axum::http::StatusCode> {
        state.aggregation_exit_replay(&gateway_app_providers::operations::aggregation::Owner {
            project: o.project.clone(), host: o.host.clone(), app: o.app, principal: o.principal.clone(),
        }, request, fingerprint)
    }
    #[cfg(target_os = "macos")]
    fn start(&self, state: &Self::State, o: gateway_aggregation::exit_host::Owner, request: String, fingerprint: [u8;32],
        work: impl FnOnce() -> vw_api_app_aggregation::AggregationOutcome + Send + 'static)
        -> Result<serde_json::Value, axum::http::StatusCode> {
        state.aggregation_exit_start(gateway_app_providers::operations::aggregation::Owner {
            project: o.project, host: o.host, app: o.app, principal: o.principal,
        }, request, fingerprint, work)
    }
    #[cfg(target_os = "macos")]
    fn database(&self) -> Result<agent_database::GlobalDatabase, axum::http::StatusCode> {
        agent_database::GlobalDatabase::for_home(&agent_paths::paths().home)
            .map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)
    }
    #[cfg(target_os = "macos")]
    fn exit_context(&self, p: gateway_aggregation::publication_host::Publication, control: Self::Routing) -> Self::Context {
        agent_gateway::aggregation_resync::RecordPublicationContext {
            database: p.database, project: p.project, host: p.host, app: p.app, home: p.home,
            root: p.root, routing: Some(control.routing), expected: p.expected,
            principal: p.principal, revision: p.revision, failure: Default::default(),
        }
    }
}
