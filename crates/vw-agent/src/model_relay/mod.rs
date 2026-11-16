//! Loopback-only model transport boundary. Never enters the session/tool execution loop.
mod binding;
mod server;
pub use binding::{RelayBinding, RelayIdentity};
pub use server::{RelayBackend, RelayRequest, RelayServer};
pub use model_transport::upstream_http_client;
// Binding unit tests live in model-transport.
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
mod transport_tests;

pub(crate) mod messages;
pub(crate) mod messages_stream;
pub(crate) mod messages_thinking;
mod messages_thinking_config;
// Pure thinking receipt tests live in model-protocol.

// Pure Responses tests live in model-protocol.

mod responses;
mod responses_chat_request;
// responses_chat_request tests live in model-protocol.
pub(crate) mod responses_compaction;
// responses_compaction tests live in model-protocol.
mod responses_stream;



mod gemini;
mod gemini_stream;

mod kiro;

pub(crate) mod stream_observer;


pub(crate) mod sse_framing;

pub(crate) mod native_chat;


pub(crate) mod native_messages;

pub(crate) mod native_responses;

pub(crate) mod native_gemini;

pub(crate) mod messages_chat_request;

pub(crate) mod messages_chat_result;


pub(crate) mod messages_gemini_request;

pub(crate) mod messages_responses_request;

pub(crate) mod messages_responses_result;

pub(crate) mod messages_gemini_result;

pub(crate) mod messages_gemini_stream;

pub(crate) mod messages_responses_stream;

pub(crate) mod gemini_shadow;


pub(crate) mod hosted_search;

pub(crate) mod search_citations;
// Search citation tests live in model-protocol.

mod responses_chat_result;
// responses_chat_result tests live in model-protocol.

mod responses_chat_stream;

pub(crate) mod responses_history;
// Responses history tests live in model-protocol.

mod tool_namespace;

mod responses_anthropic_request;
// responses_anthropic_request tests live in model-protocol.

mod responses_anthropic_result;
// responses_anthropic_result tests live in model-protocol.

mod responses_anthropic_stream;
// responses_anthropic_stream tests live in model-protocol.

pub(crate) mod reasoning_bridge;

// reasoning_bridge tests live in model-protocol.

mod inline_think;

pub(crate) mod late_arguments;


// xAI Responses tests live in model-protocol.

pub(crate) mod signature_rectify;
// Signature repair tests live in model-protocol.

mod tool_media;
mod tool_media_request;

#[cfg(target_os = "macos")]
pub(crate) mod copilot_transport;
// Copilot transport tests live in model-transport.

pub(crate) mod codex_oauth_transport;

pub(crate) mod quota_gate;


pub(crate) mod gemini_oauth_transport;

pub(crate) mod compaction;

pub(crate) mod compaction_v2;

pub(crate) mod models;

pub(crate) mod gemini_metadata;


pub(crate) mod alpha_search;

pub(crate) mod image_generation;

pub(crate) mod image_edit;


pub(crate) mod forced_sse;
