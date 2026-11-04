//! Loopback-only model transport boundary. Never enters the session/tool execution loop.
mod binding;
mod server;
pub use binding::{RelayBinding, RelayIdentity};
pub use server::{RelayBackend, RelayRequest, RelayServer, upstream_http_client};
#[cfg(test)]
mod binding_tests;
#[cfg(test)]
mod server_tests;
