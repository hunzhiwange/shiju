pub(crate) mod gateway_owner;
mod listener;
pub(crate) mod reference_scope;
mod shared_transport;
mod shutdown;
pub use listener::{ListenerError, RoutingListener};
pub use shared_transport::SharedTransport;

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
