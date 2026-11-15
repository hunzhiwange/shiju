//! Compose the restart HTTP route with the unique provider operation registry.
use super::aggregation_codex_settings::SnapshotHost;
use gateway_aggregation::restart_host::RestartHost;
#[cfg(target_os = "macos")]
use gateway_aggregation::restart_host::Owner;
impl RestartHost for SnapshotHost {
    type State = super::app_providers::operations::State;
    #[cfg(target_os = "macos")]
    fn policy<'a>(&self, state: &'a Self::State) -> Option<&'a std::path::Path> { state.aggregation_policy() }
    #[cfg(target_os = "macos")]
    fn replay(&self, state: &Self::State, o: &Owner, request: &str, fingerprint: &[u8;32])
        -> Result<Option<serde_json::Value>, axum::http::StatusCode> {
        state.aggregation_restart_replay(&super::app_providers::operations::aggregation::Owner {
            project: o.project.clone(), host: o.host.clone(), app: o.app, principal: o.principal.clone(),
        }, request, fingerprint)
    }
    #[cfg(target_os = "macos")]
    fn start(&self, state: &Self::State, o: Owner, request: String, fingerprint: [u8;32],
        work: impl FnOnce() -> vw_api_app_aggregation::AggregationOutcome + Send + 'static)
        -> Result<serde_json::Value, axum::http::StatusCode> {
        state.aggregation_restart_start(super::app_providers::operations::aggregation::Owner {
            project: o.project, host: o.host, app: o.app, principal: o.principal,
        }, request, fingerprint, work)
    }
}

pub(super) fn router() -> axum::Router { gateway_aggregation::aggregation_restart::router(SnapshotHost) }
