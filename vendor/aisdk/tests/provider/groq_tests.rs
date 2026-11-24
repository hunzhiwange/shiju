//! Groq provider integration tests.
use aisdk::providers::groq::{Groq, OpenaiGptOss20b};

// Include all macro definitions
include!("macros.rs");

// Generate all standard integration tests for Groq
generate_language_model_tests!(
    provider: Groq,
    api_key_var: "GROQ_API_KEY",
    model_struct: OpenaiGptOss20b,
    default_model: Groq::openai_gpt_oss_20b(),
    tool_model: Groq::qwen_qwen3_8_27b(),
    structured_output_model: Groq::qwen_qwen3_8_27b(),
    reasoning_model: Groq::openai_gpt_oss_120b(),
    embedding_model: Groq::qwen_qwen3_8_27b(),
    skip_reasoning: true,
    skip_tool: false,
    skip_structured_output: true,
    skip_streaming: false,
    skip_embedding: true
);
