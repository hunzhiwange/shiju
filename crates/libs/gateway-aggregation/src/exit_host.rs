//! Exit operation registry and native journal ports; no duplicate state.
#[cfg(target_os = "macos")]
use axum::http::StatusCode;
#[cfg(target_os = "macos")]
use vw_api_app_aggregation::*;
use super::publication_host::Host;
#[cfg(target_os = "macos")]
use super::publication_host::Publication;
#[derive(Clone)]
pub struct Owner { pub project: vw_api_transport::ProjectScope, pub host: String,
    pub app: vw_api_app_identity::AppId, pub principal: String }
pub trait ExitHost: Host {
    type State: Clone + Send + Sync + 'static;
    #[cfg(target_os = "macos")]
    type Context: ExitContext + Send + 'static;
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
    fn exit_context(&self, publication: Publication, control: Self::Routing) -> Self::Context;
}
#[cfg(target_os = "macos")]
pub trait ExitContext {
    fn home(&self) -> &std::path::PathBuf;
    fn root(&self) -> &std::path::PathBuf;
    fn principal(&self) -> &String;
    fn exit_aggregation(&self, authorized: impl Fn() -> bool) -> Result<(), StatusCode>;
    fn exited_state(&self) -> Result<AggregationState, StatusCode>;
}
