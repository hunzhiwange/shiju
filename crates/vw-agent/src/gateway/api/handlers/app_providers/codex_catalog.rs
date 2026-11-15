//! Composition of provider routes with the existing publication transaction.
use super::*;
use std::path::PathBuf;
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(in crate::gateway::api::handlers) fn router_with_policy(path: Option<PathBuf>) -> Router {
    let router = gateway_app_providers::codex_catalog::router_with_policy(path,
        #[cfg(target_os = "macos")] super::publication_host::control(None));
    #[cfg(target_os = "macos")]
    let router = router.layer(axum::middleware::from_fn(super::publication_host::inject));
    router
}
#[cfg(target_os = "macos")]
pub(super) async fn sync_saved(database: GlobalDatabase, policy: Option<PathBuf>,
    headers: axum::http::HeaderMap, target: &vw_api_app_operations::DataManagementTarget,
    id: &str) -> Result<(), ()> {
    gateway_app_providers::codex_catalog::sync_saved(database, policy, headers, target, id,
        super::publication_host::control(None)).await
}
