//! Loopback-only model transport boundary. Never enters the session/tool execution loop.
mod binding;
mod server;
pub(crate) mod upstream;
pub use binding::{RelayBinding, RelayIdentity};
pub use server::{RelayBackend, RelayRequest, RelayServer};
pub use upstream::upstream_http_client;
#[cfg(test)]
mod binding_tests;
#[cfg(test)]
mod server_tests;

mod chat;
mod transport;
pub use transport::ProviderRelayBackend;
pub(crate) use transport::{chat_url, official_deepseek_messages_url};
#[cfg(test)]
mod chat_tests;
#[cfg(test)]
mod transport_tests;

mod messages;
mod messages_stream;
#[cfg(test)]
mod messages_tests;

#[cfg(test)]
mod responses_tests;

mod responses;
mod responses_stream;

#[cfg(test)]
mod responses_http_tests;

#[cfg(test)]
mod gemini_tests;

mod gemini;
mod gemini_stream;

mod kiro;
