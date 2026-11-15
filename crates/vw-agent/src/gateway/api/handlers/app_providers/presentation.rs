//! Host publication extension is injected at composition.
use super::*;
pub(super) fn router() -> Router {
    let router = gateway_app_providers::presentation::router();
    #[cfg(target_os = "macos")]
    let router = router.layer(axum::middleware::from_fn(super::publication_host::inject));
    router
}
