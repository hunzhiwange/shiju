#[cfg(target_os = "macos")]
pub(crate) use gateway_app_routing::server::provider_composition::codex_catalog::*;
#[cfg(not(target_os = "macos"))]
mod portable {
//! Composition of provider routes with the existing publication transaction.
use super::super::*;
use std::path::PathBuf;
pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(crate) fn router_with_policy(path: Option<PathBuf>) -> Router {
    let router = gateway_app_providers::codex_catalog::router_with_policy(path,
        #[cfg(target_os = "macos")] super::super::publication_host::control(None));
    #[cfg(target_os = "macos")]
    let router = router.layer(axum::middleware::from_fn(super::super::publication_host::inject));
    router
}
// Saved-provider dispatch and publication forwarding live in gateway_app_providers.
}
#[cfg(not(target_os = "macos"))]
pub(crate) use portable::*;
