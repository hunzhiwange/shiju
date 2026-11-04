//! Loopback-only model transport boundary. Never enters the session/tool execution loop.
mod binding;
mod server;
pub use binding::{RelayBinding, RelayIdentity};
pub use server::{RelayBackend, RelayRequest, RelayServer, upstream_http_client};
#[cfg(test)]
mod binding_tests;
#[cfg(test)]
mod server_tests;

mod chat;
mod transport;
pub use transport::ProviderRelayBackend;
#[cfg(test)]
mod chat_tests;
#[cfg(test)]
mod transport_tests;

mod messages;
mod messages_stream;
#[cfg(test)]
mod messages_tests;
