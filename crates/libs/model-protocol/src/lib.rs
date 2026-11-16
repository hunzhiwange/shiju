//! Model wire protocol validation and endpoint policies.
pub mod alpha_search;
pub mod chat;
pub mod compaction;
pub mod image_edit;
pub mod image_generation;
pub mod inline_think;
pub mod kiro;

#[cfg(test)]
mod image_edit_tests;

pub mod validation;
pub mod sse_framing;
pub mod gemini;
pub mod late_arguments;
pub mod hosted_search;
#[cfg(test)]
mod late_arguments_tests;
#[cfg(test)]
mod sse_framing_tests;
pub mod endpoint_urls;
pub mod gemini_metadata;
#[cfg(test)]
mod gemini_metadata_tests;
pub mod stream_observer;
pub mod compaction_prompt;
pub mod forced_sse;
pub mod gemini_stream;
pub mod compaction_v2;
#[cfg(test)]
mod compaction_v2_tests;
pub mod gemini_shadow;
pub mod messages;
pub mod responses;
pub mod generation;
pub mod protocol_bridge;
#[cfg(test)]
mod gemini_shadow_tests;
pub mod dsml;
pub mod models;
pub mod messages_chat_request;
pub mod messages_chat_result;
pub mod messages_gemini_request;
pub mod messages_gemini_result;
pub mod messages_gemini_stream;
pub mod messages_responses_request;
pub mod messages_responses_result;
pub mod messages_responses_stream;
pub mod messages_stream;
pub mod native_chat;
pub mod native_gemini;
pub mod native_messages;
pub mod native_responses;
pub mod reasoning_bridge;
pub mod responses_anthropic_request;
pub mod responses_anthropic_result;
pub mod responses_anthropic_stream;
pub mod responses_chat_request;
pub mod responses_chat_result;
pub mod responses_chat_stream;
pub mod responses_compaction;
pub mod responses_history;
#[cfg(test)]
mod responses_history_tests;
#[cfg(test)]
mod responses_tests;
pub mod responses_stream;
pub mod search_citations;
#[cfg(test)]
mod search_citations_tests;
pub mod signature_rectify;
#[cfg(test)]
mod signature_rectify_tests;
pub mod tool_media;
pub mod tool_media_request;
pub mod tool_namespace;
pub mod xai_responses;
#[cfg(test)]
mod xai_responses_tests;

pub mod messages_thinking;
pub mod messages_thinking_config;
pub mod protocol_bridge_stream;
#[cfg(test)]
mod messages_thinking_core_tests;

// Messages protocol tests follow their production modules.
#[cfg(test)]
mod messages_chat_request_tests;
#[cfg(test)]
mod messages_chat_result_tests;
#[cfg(test)]
mod messages_gemini_request_tests;
#[cfg(test)]
mod messages_gemini_result_tests;
#[cfg(test)]
mod messages_gemini_stream_tests;
#[cfg(test)]
mod messages_responses_request_tests;
#[cfg(test)]
mod messages_responses_result_tests;
#[cfg(test)]
mod messages_responses_stream_tests;
#[cfg(test)]
mod messages_request_core_tests;

// Native protocol and Messages conversion tests follow their implementation owners.
#[cfg(test)]
mod native_chat_tests;
#[cfg(test)]
mod native_gemini_tests;
#[cfg(test)]
mod native_messages_tests;
#[cfg(test)]
mod native_responses_tests;
#[cfg(test)]
mod messages_tests;

// Responses bridge tests follow their production owners.
#[cfg(test)]
mod reasoning_bridge_tests;
#[cfg(test)]
mod responses_anthropic_request_tests;
#[cfg(test)]
mod responses_anthropic_result_tests;
#[cfg(test)]
mod responses_anthropic_stream_tests;
#[cfg(test)]
mod responses_chat_request_tests;
#[cfg(test)]
mod responses_chat_result_tests;
#[cfg(test)]
mod responses_compaction_tests;
