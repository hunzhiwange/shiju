//! Amazon Bedrock provider integration tests.
use aisdk::providers::amazon_bedrock::{AmazonBedrock, AnthropicClaudeHaiku4520251001V10};

// Include all macro definitions
include!("macros.rs");

// Generate all standard integration tests for Amazon Bedrock
generate_language_model_tests!(
    provider: AmazonBedrock,
    api_key_var: "BEDROCK_API_KEY",
    model_struct: AnthropicClaudeHaiku4520251001V10,
    default_model: AmazonBedrock::anthropic_claude_haiku_4_5_20251001_v1_0(),
    tool_model: AmazonBedrock::anthropic_claude_haiku_4_5_20251001_v1_0(),
    structured_output_model: AmazonBedrock::anthropic_claude_haiku_4_5_20251001_v1_0(),
    reasoning_model: AmazonBedrock::anthropic_claude_haiku_4_5_20251001_v1_0(),
    embedding_model: AmazonBedrock::anthropic_claude_haiku_4_5_20251001_v1_0(),
    skip_reasoning: false,
    skip_tool: false,
    skip_structured_output: false,
    skip_streaming: false,
    skip_embedding: true
);
