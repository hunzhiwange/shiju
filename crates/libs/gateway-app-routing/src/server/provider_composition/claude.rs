//! Provider routes and write adapter for host publication input.
use axum::Router;

use std::path::PathBuf;
pub use gateway_app_providers::claude::{Policy, WriteKind};
// Saved-provider dispatch is owned by gateway_app_providers.
pub fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub fn router_with_policy(policy: Option<PathBuf>) -> Router {
    router_with_context(Policy::new(policy))
}
pub fn router_with_context(policy: Policy) -> Router {
    let router = gateway_app_providers::claude::router_with_context(policy);
    #[cfg(target_os = "macos")]
    let router = router.layer(axum::middleware::from_fn(crate::server::provider_publication_host::inject));
    router
}
