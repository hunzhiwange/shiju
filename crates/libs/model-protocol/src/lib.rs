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
