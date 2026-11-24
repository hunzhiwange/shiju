//! xAI provider integration tests.
use aisdk::providers::xai::{Grok43, XAI};

// Include all macro definitions
include!("macros.rs");

// Generate all standard integration tests for xAI
generate_language_model_tests!(
    provider: XAI,
    api_key_var: "XAI_API_KEY",
    model_struct: Grok43,
    default_model: XAI::grok_4_3(),
    tool_model: XAI::grok_4_3(),
    structured_output_model: XAI::grok_4_3(),
    reasoning_model: XAI::grok_4_3(),
    embedding_model: XAI::grok_4_3(),
    skip_reasoning: false,
    skip_tool: false,
    skip_structured_output: true,  // xAI doesn't seem to have structured output models
    skip_streaming: false,
    skip_embedding: true
);
