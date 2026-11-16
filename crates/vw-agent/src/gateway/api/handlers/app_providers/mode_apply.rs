//! Gateway publication and managed-account adapter for the mode route.
use super::*;
// Mode authorization and publication belong to gateway-app-routing.
pub(crate) use gateway_app_providers::mode_apply::ProviderRouteRuntime;
#[cfg(target_os = "macos")]
use super::super::app_routing::RoutingControl;
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<std::path::PathBuf>) -> Router {
    let router = gateway_app_providers::mode_apply::router_with_policy(path);
    #[cfg(target_os = "macos")]
    let router = router.layer(axum::middleware::from_fn(|mut req: axum::extract::Request, next: axum::middleware::Next| async move {
        let control = req.extensions().get::<RoutingControl>().cloned();
        req.extensions_mut().insert(gateway_app_providers::mode_apply::ModeHost(std::sync::Arc::new(Host(control, !cfg!(test)))));
        next.run(req).await
    }));
    router
}
#[cfg(target_os = "macos")]
use gateway_app_routing::server::provider_mode_host::Host;
