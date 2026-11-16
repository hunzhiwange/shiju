//! Protocol and transport contracts migrated from the agent host.
use model_protocol::*;
use model_transport::{RelayIdentity, routing_selection as routing};
mod compaction_v2_tests;
mod hosted_search_tests;
mod messages_chat_stream_tests;
mod sse_framing_tests;
mod stream_observer_tests;
mod gemini_tests;
mod routing_tests;
mod thinking_stream_tests;

pub(crate) mod fixture;
use fixture::{RelayServer, ProviderRelayBackend, global_async_env_lock};
use model_transport::RelayBinding;
mod chat_tests;
mod gemini_http_tests;
mod messages_tests;
mod responses_http_tests;
mod messages_request_http_tests;
mod messages_thinking_http_tests;
mod messages_request_tests;
mod messages_thinking_tests;
mod messages_stream_tests;
