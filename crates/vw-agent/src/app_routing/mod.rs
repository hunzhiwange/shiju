mod body_limits;
mod browser_guard;
#[cfg(target_os = "macos")]
pub(crate) mod config_apply;
mod decode_request;
mod decode_response;
mod dispatch;
pub(crate) mod gateway_owner;
mod header_wire;
mod listener;
mod override_apply;
pub(crate) mod reference_scope;
mod route_auth;
mod shared_transport;
mod shutdown;
pub use listener::{ListenerError, RoutingListener};
#[cfg(target_os = "macos")]
pub(crate) use route_auth::VerifiedRouteBinding;
pub use shared_transport::SharedTransport;

#[cfg(all(test, target_os = "macos"))]
mod body_limits_tests;

#[cfg(all(test, target_os = "macos"))]
mod config_apply_tests;

#[cfg(test)]
mod listener_tests;

#[cfg(test)]
mod shared_transport_tests;

#[cfg(test)]
mod reference_scope_tests;

#[cfg(test)]
mod shutdown_tests;

#[cfg(all(test, target_os = "macos"))]
mod gateway_owner_tests;
