//! Host-owned provider authorization and publication context wiring.
use super::publication_host::{Host, Publication};
use axum::http::{HeaderMap, StatusCode};
use app_provider_services::AppProviderStore;
use std::path::{Path, PathBuf};
use vw_api_transport::TenantScope;
pub trait ResyncHost: Host {
    type Context;
    fn editor_context(&self, policy: Option<&Path>, headers: &HeaderMap,
        tenant: &TenantScope, host: &str, write: bool) -> Result<(PathBuf, PathBuf), StatusCode>;
    fn attach(&self, store: AppProviderStore, publication: Publication, routing: Option<Self::Routing>)
        -> (AppProviderStore, Self::Context);
}
