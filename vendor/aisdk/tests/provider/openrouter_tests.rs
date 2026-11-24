//! OpenRouter provider integration tests.
use aisdk::providers::openrouter::{AnthropicClaudeHaiku45, Openrouter};

// Include all macro definitions
include!("macros.rs");

// Generate all standard integration tests for OpenRouter
generate_language_model_tests!(
    provider: Openrouter,
    api_key_var: "OPENROUTER_API_KEY",
    model_struct: AnthropicClaudeHaiku45,
    default_model: Openrouter::anthropic_claude_haiku_4_5(),
    tool_model: Openrouter::anthropic_claude_haiku_4_5(),
    structured_output_model: Openrouter::model_name("openai/gpt-5-1".to_string()),
    reasoning_model: Openrouter::anthropic_claude_haiku_4_5(),
    embedding_model: Openrouter::model_name("anthropic/claude-haiku-4.5".to_string()),
    skip_reasoning: false,
    skip_tool: false,
    skip_structured_output: true,
    skip_streaming: false,
    skip_embedding: true  // Couldn't find free embedding model
);
