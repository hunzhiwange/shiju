//! Compose the publication HTTP route with the unique provider operation registry.
use super::aggregation_codex_settings::SnapshotHost;
use gateway_aggregation::publish_host::{PublishHost, Owner};
impl PublishHost for SnapshotHost {
    type State = super::app_providers::operations::State;
    #[cfg(target_os = "macos")]
    type Context = crate::gateway::aggregation_resync::RecordPublicationContext;
    #[cfg(target_os = "macos")]
    fn policy<'a>(&self, state: &'a Self::State) -> Option<&'a std::path::Path> { state.aggregation_policy() }
    #[cfg(target_os = "macos")]
    fn replay(&self, state: &Self::State, o: &Owner, request: &str, fingerprint: &[u8;32])
        -> Result<Option<serde_json::Value>, axum::http::StatusCode> {
        state.aggregation_replay(&super::app_providers::operations::aggregation::Owner {
            project: o.project.clone(), host: o.host.clone(), app: o.app, principal: o.principal.clone(),
        }, request, fingerprint)
    }
    #[cfg(target_os = "macos")]
    fn start(&self, state: &Self::State, o: Owner, request: String, fingerprint: [u8;32],
        work: impl FnOnce() -> vw_api_app_aggregation::AggregationOutcome + Send + 'static)
        -> Result<serde_json::Value, axum::http::StatusCode> {
        state.aggregation_start(super::app_providers::operations::aggregation::Owner {
            project: o.project, host: o.host, app: o.app, principal: o.principal,
        }, request, fingerprint, work)
    }
    #[cfg(target_os = "macos")]
    fn database(&self) -> Result<crate::global_database::GlobalDatabase, axum::http::StatusCode> {
        crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)
    }
    #[cfg(target_os = "macos")]
    fn publication_context(&self, p: gateway_aggregation::publication_host::Publication, control: Self::Routing) -> Self::Context {
        crate::gateway::aggregation_resync::RecordPublicationContext {
            database: p.database, project: p.project, host: p.host, app: p.app, home: p.home,
            root: p.root, routing: Some(control.routing), expected: p.expected,
            principal: p.principal, revision: p.revision, failure: Default::default(),
        }
    }
}
// PublishContext is implemented by the agent-gateway context owner.

pub(super) fn router() -> axum::Router { gateway_aggregation::aggregation_publish::router(SnapshotHost) }

pub(super) use gateway_aggregation::aggregation_publish::operation_error;
