//! Publication registry and journal actions retain their existing host owners.
#[cfg(target_os = "macos")]
use axum::http::StatusCode;
#[cfg(target_os = "macos")]
use vw_api_app_aggregation::*;
use super::publication_host::Host;
#[cfg(target_os = "macos")]
use super::publication_host::Publication;
pub use super::exit_host::Owner;
pub trait PublishHost: Host {
    type State: Clone + Send + Sync + 'static;
    #[cfg(target_os = "macos")]
    type Context: PublishContext + Send + 'static;
    #[cfg(target_os = "macos")]
    fn policy<'a>(&self, state: &'a Self::State) -> Option<&'a std::path::Path>;
    #[cfg(target_os = "macos")]
    fn replay(&self, state: &Self::State, owner: &Owner, request: &str, fingerprint: &[u8;32])
        -> Result<Option<serde_json::Value>, StatusCode>;
    #[cfg(target_os = "macos")]
    fn start(&self, state: &Self::State, owner: Owner, request: String, fingerprint: [u8;32],
        work: impl FnOnce() -> AggregationOutcome + Send + 'static) -> Result<serde_json::Value, StatusCode>;
    #[cfg(target_os = "macos")]
    fn database(&self) -> Result<agent_database::GlobalDatabase, StatusCode>;
    #[cfg(target_os = "macos")]
    fn publication_context(&self, publication: Publication, control: Self::Routing) -> Self::Context;
}
#[cfg(target_os = "macos")]
pub trait PublishContext {
    fn home(&self) -> &std::path::PathBuf;
    fn principal(&self) -> &String;
    fn publish_aggregation(&self, default: &str,
        policy: vw_agent_apps::aggregation::codex_publish::CatalogPolicy, record: &serde_json::Value)
        -> Result<bool, StatusCode>;
    fn published_state(&self) -> Result<AggregationState, StatusCode>;
}
