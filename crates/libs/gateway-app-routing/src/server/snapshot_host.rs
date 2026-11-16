//! Concrete persisted snapshot reader for native routing publication.
use agent_database::GlobalDatabase;
#[derive(Clone)]
pub struct SnapshotHost;
impl gateway_aggregation::aggregation_codex_settings::SnapshotHost for SnapshotHost {
    #[cfg(target_os = "macos")]
    type Snapshot = gateway_aggregation::aggregation_read_snapshot::Snapshot;
    #[cfg(target_os = "macos")]
    fn load(&self, db: &GlobalDatabase, project: &vw_api_transport::ProjectScope, host: &str,
        app: vw_api_app_identity::AppId, home: &std::path::Path,
        state: &vw_agent_apps::providers::live::commit::AppState, candidate: Option<&str>)
        -> Result<Self::Snapshot, axum::http::StatusCode> {
        gateway_aggregation::aggregation_read_snapshot::load(db, project, host, app, home, state, candidate)
    }
    #[cfg(target_os = "macos")]
    fn revision(&self, project: &vw_api_transport::ProjectScope, host: &str,
        app: vw_api_app_identity::AppId, principal: &str,
        state: &vw_agent_apps::providers::live::commit::AppState, snapshot: &Self::Snapshot)
        -> Result<String, axum::http::StatusCode> {
        gateway_aggregation::aggregation_read_snapshot::revision(project, host, app, principal, state, snapshot)
    }
    #[cfg(target_os = "macos")]
    fn codex_inputs<'a>(&self, snapshot: &'a Self::Snapshot)
        -> Result<(Option<&'a [u8]>, Option<&'a [u8]>), axum::http::StatusCode> {
        gateway_aggregation::aggregation_read_snapshot::codex_inputs(snapshot)
    }
}
