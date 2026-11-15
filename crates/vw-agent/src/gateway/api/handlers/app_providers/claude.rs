//! Provider routes and write adapter for host publication input.
use super::*;
use std::path::PathBuf;
pub use gateway_app_providers::claude::{Policy, WriteKind};
#[cfg(target_os = "macos")]
pub use gateway_app_providers::claude::sync_saved;
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(in crate::gateway::api::handlers) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    router_with_context(Policy::new(policy))
}
pub(super) fn router_with_context(policy: Policy) -> Router {
    let router = gateway_app_providers::claude::router_with_context(policy);
    #[cfg(target_os = "macos")]
    let router = router.layer(axum::middleware::from_fn(super::publication_host::inject));
    router
}
#[allow(clippy::too_many_arguments)]
pub(super) async fn write(tenant: TenantScope, db: Option<Extension<GlobalDatabase>>,
    headers: axum::http::HeaderMap, policy: Option<Extension<Policy>>, id: String,
    req: vw_api_app_provider_base::claude::ClaudeWrite, kind: WriteKind,
    publication: PublicationInput) -> Response {
    #[cfg(target_os = "macos")]
    let publication = super::publication_host::input(publication);
    #[cfg(not(target_os = "macos"))]
    let publication = gateway_app_providers::PublicationInput { query: publication.query };
    gateway_app_providers::claude::write(tenant, db, headers, policy, id, req, kind, publication).await
}
