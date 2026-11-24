//! Deepseek provider integration tests.
use aisdk::providers::deepseek::{Deepseek, DeepseekFlash};

// Include all macro definitions
include!("macros.rs");

// Generate all standard integration tests for Deepseek
generate_language_model_tests!(
    provider: Deepseek,
    api_key_var: "DEEPSEEK_API_KEY",
    model_struct: DeepseekFlash,
    default_model: Deepseek::deepseek_flash(),
    tool_model: Deepseek::deepseek_flash(),
    structured_output_model: Deepseek::deepseek_flash(),
    reasoning_model: Deepseek::model_name("deepseek-flash".to_string()),
    embedding_model: Deepseek::deepseek_flash(),
    skip_reasoning: true,
    skip_tool: false,
    skip_structured_output: true,
    skip_streaming: false,
    skip_embedding: true
);
