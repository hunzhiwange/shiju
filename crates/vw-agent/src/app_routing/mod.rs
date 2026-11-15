//! Application routing composition; runtime and state belong to app-routing-services.
pub use app_routing_services::{ListenerError, RoutingListener, SharedTransport};
#[cfg(target_os = "macos")]
pub use app_routing_services::adapter_selection;
pub(crate) use app_routing_services::{dispatch, reference_scope, request_status};
#[cfg(target_os = "macos")]
pub(crate) use app_routing_services::{
    aggregation_routing, config_apply, desktop_routing, failover_routing,
    outbound_config, provider_publication, RouteCommitError, RouteRevocation, VerifiedRouteBinding,
};
pub(crate) mod gateway_owner;

#[cfg(all(test, target_os = "macos"))]
mod gateway_owner_tests;
#[cfg(all(test, target_os = "macos"))]
mod failover_routing_tests;
