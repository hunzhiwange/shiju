#[cfg(target_os = "macos")]
pub(crate) mod aggregation_routing;
mod body_limits;
mod browser_guard;
mod cancellation;
mod capacity;
#[cfg(target_os = "macos")]
pub(crate) mod config_apply;
mod decode_request;
mod decode_response;
pub(crate) mod dispatch;
pub(crate) mod gateway_owner;
mod header_wire;
mod listener;
#[cfg(target_os = "macos")]
pub(crate) mod outbound_config;
mod outbound_custom;
#[cfg(target_os = "macos")]
mod outbound_system;
mod override_apply;
mod protocol_errors;
pub(crate) mod reference_scope;
mod route_auth;
mod shared_transport;
mod shutdown;
mod timeouts;
mod transport_security;
pub use listener::{ListenerError, RoutingListener};
#[cfg(target_os = "macos")]
pub(crate) use route_auth::{RouteCommitError, RouteRevocation, VerifiedRouteBinding};
pub use shared_transport::SharedTransport;

#[cfg(all(test, target_os = "macos"))]
mod body_limits_tests;

#[cfg(all(test, target_os = "macos"))]
mod config_apply_tests;

#[cfg(test)]
mod listener_tests;

#[cfg(test)]
mod shared_transport_tests;

#[cfg(all(test, target_os = "macos"))]
mod outbound_config_tests;

#[cfg(all(test, target_os = "macos"))]
mod outbound_custom_tests;

#[cfg(all(test, target_os = "macos"))]
mod outbound_system_tests;

#[cfg(test)]
mod reference_scope_tests;

#[cfg(test)]
mod shutdown_tests;

#[cfg(all(test, target_os = "macos"))]
mod gateway_owner_tests;

#[cfg(all(test, target_os = "macos"))]
mod timeouts_tests;

#[cfg(all(test, target_os = "macos"))]
mod cancellation_tests;

#[cfg(all(test, target_os = "macos"))]
mod capacity_tests;

#[cfg(all(test, target_os = "macos"))]
mod protocol_errors_tests;

#[cfg(all(test, target_os = "macos"))]
mod transport_security_tests;

#[cfg(all(test, target_os = "macos"))]
mod compaction_tests;
