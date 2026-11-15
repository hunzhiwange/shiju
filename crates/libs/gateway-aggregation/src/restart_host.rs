//! Restart requests use the unique host provider operation registry.
#[cfg(target_os = "macos")]
use axum::http::StatusCode;
#[cfg(target_os = "macos")]
use vw_api_app_aggregation::*;
pub use super::exit_host::Owner;
pub trait RestartHost: Clone + Send + Sync + 'static {
    type State: Clone + Send + Sync + 'static;
    #[cfg(target_os = "macos")]
    fn policy<'a>(&self, state: &'a Self::State) -> Option<&'a std::path::Path>;
    #[cfg(target_os = "macos")]
    fn replay(&self, state: &Self::State, owner: &Owner, request: &str, fingerprint: &[u8;32])
        -> Result<Option<serde_json::Value>, StatusCode>;
    #[cfg(target_os = "macos")]
    fn start(&self, state: &Self::State, owner: Owner, request: String, fingerprint: [u8;32],
        work: impl FnOnce() -> AggregationOutcome + Send + 'static) -> Result<serde_json::Value, StatusCode>;
}
