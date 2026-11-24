//! Together AI provider integration tests.
use aisdk::providers::togetherai::{MoonshotaiKimiK3, TogetherAI};

// Include all macro definitions
include!("macros.rs");

// Generate all standard integration tests for Together AI
generate_language_model_tests!(
    provider: TogetherAI,
    api_key_var: "TOGETHER_API_KEY",
    model_struct: MoonshotaiKimiK3,
    default_model: TogetherAI::moonshotai_kimi_k3(),
    tool_model: TogetherAI::moonshotai_kimi_k3(),
    structured_output_model: TogetherAI::deepseek_ai_deepseek_v4_pro_0813(),
    reasoning_model: TogetherAI::deepseek_ai_deepseek_v4_pro_0813(),
    embedding_model: TogetherAI::deepseek_ai_deepseek_v4_pro_0813(),
    skip_reasoning: false,
    skip_tool: false,
    skip_structured_output: true,
    skip_streaming: false,
    skip_embedding: true
);
