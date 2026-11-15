//! Narrow host actions used by aggregation HTTP transactions.
#[cfg(target_os = "macos")]
use axum::http::{HeaderMap, StatusCode};
#[cfg(target_os = "macos")]
use vw_api_app_identity::AppId;
#[cfg(target_os = "macos")]
use vw_api_transport::{ProjectScope, TenantScope};
use super::aggregation_codex_settings::SnapshotHost;
#[cfg(target_os = "macos")]
use vw_agent_apps::providers::live::commit::AppState;
#[cfg(target_os = "macos")]
pub struct Publication {
    pub database: agent_database::GlobalDatabase,
    pub project: ProjectScope,
    pub host: String,
    pub app: AppId,
    pub home: std::path::PathBuf,
    pub root: std::path::PathBuf,
    pub expected: AppState,
    pub principal: String,
    pub revision: String,
}
pub trait Host: SnapshotHost {
    type Routing: Clone + Send + Sync + 'static;
    #[cfg(target_os = "macos")]
    fn provider_headers(&self, headers: HeaderMap) -> Result<HeaderMap, StatusCode>;
    #[cfg(target_os = "macos")]
    fn authorize_mode(&self, control: &Self::Routing, headers: &HeaderMap,
        project: &ProjectScope, host: &str, app: AppId) -> Result<(std::path::PathBuf, String), StatusCode>;
    #[cfg(target_os = "macos")]
    fn project(&self, app: AppId, tenant: &TenantScope, state: &AppState,
        snapshot: &Self::Snapshot, request: Option<&vw_api_app_aggregation::AggregationPreviewRequest>)
        -> Result<vw_api_app_aggregation::AggregationPreview, StatusCode>;
    #[cfg(target_os = "macos")]
    fn publish_candidate(&self, publication: Publication, control: Self::Routing, candidate: &AppState)
        -> Result<bool, StatusCode>;
    #[cfg(target_os = "macos")]
    fn sign(&self, bytes: &[u8]) -> Result<String, StatusCode>;
    #[cfg(target_os = "macos")]
    fn discovery(&self, policy: Option<&std::path::Path>, headers: &HeaderMap, tenant: &TenantScope, host: &str)
        -> Result<(vw_agent_apps::apps::discovery::read::LocalReadEnvironment, String), StatusCode>;
    #[cfg(target_os = "macos")]
    fn replace(&self, publication: Publication, control: Option<Self::Routing>, key: &str, before: &str, after: &str)
        -> Result<bool, StatusCode>;
}
