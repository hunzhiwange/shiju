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
pub(crate) mod generation;
mod protocol_bridge;
mod protocol_bridge_stream;
pub(crate) mod routing;
mod transport;
pub use transport::ProviderRelayBackend;
pub(crate) use transport::{chat_url, official_deepseek_messages_url};
#[cfg(test)]
mod chat_tests;
#[cfg(test)]
mod transport_tests;

pub(crate) mod messages;
#[cfg(test)]
mod messages_request_http_tests;
#[cfg(test)]
mod messages_request_tests;
pub(crate) mod messages_stream;
#[cfg(test)]
mod messages_tests;
pub(crate) mod messages_thinking;
mod messages_thinking_config;
#[cfg(test)]
mod messages_thinking_core_tests;
#[cfg(test)]
mod messages_thinking_http_tests;
#[cfg(test)]
mod messages_thinking_tests;

#[cfg(test)]
mod responses_tests;

mod responses;
pub(crate) mod responses_compaction;
#[cfg(test)]
mod responses_compaction_tests;
mod responses_stream;

#[cfg(test)]
mod responses_http_tests;

#[cfg(test)]
mod gemini_tests;

mod gemini;
mod gemini_stream;

mod kiro;

pub(crate) mod stream_observer;

#[cfg(test)]
mod sse_framing_tests;

pub(crate) mod sse_framing;

pub(crate) mod native_chat;
#[cfg(test)]
mod native_chat_tests;

#[cfg(test)]
mod native_messages_tests;

pub(crate) mod native_messages;

pub(crate) mod native_responses;
#[cfg(test)]
mod native_responses_tests;

pub(crate) mod native_gemini;
#[cfg(test)]
mod native_gemini_tests;

pub(crate) mod messages_chat_request;
#[cfg(test)]
mod messages_chat_request_tests;

pub(crate) mod messages_chat_result;
#[cfg(test)]
mod messages_chat_result_tests;

#[cfg(test)]
mod messages_chat_stream_tests;
