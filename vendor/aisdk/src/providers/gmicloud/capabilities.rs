//! Capabilities for gmicloud models.
//!
//! This module defines model types and their capabilities for gmicloud providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::gmicloud::Gmicloud;

model_capabilities! {
    provider: Gmicloud,
    models: {
        MinimaxaiMinimaxM27 {
            model_name: "MiniMaxAI/MiniMax-M2.7",
            constructor_name: minimaxai_minimax_m2_7,
            display_name: "MiniMax-M2.7",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxaiMinimaxM3 {
            model_name: "MiniMaxAI/MiniMax-M3",
            constructor_name: minimaxai_minimax_m3,
            display_name: "MiniMax-M3",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen37Max {
            model_name: "Qwen/Qwen3.7-Max",
            constructor_name: qwen_qwen3_7_max,
            display_name: "Qwen3.7 Max",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen38Flash {
            model_name: "Qwen/Qwen3.8-Flash",
            constructor_name: qwen_qwen3_8_flash,
            display_name: "Qwen3.8 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen38Max {
            model_name: "Qwen/Qwen3.8-Max",
            constructor_name: qwen_qwen3_8_max,
            display_name: "Qwen3.8 Max",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AnthropicClaudeOpus46 {
            model_name: "anthropic/claude-opus-4.6",
            constructor_name: anthropic_claude_opus_4_6,
            display_name: "Claude Opus 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus47 {
            model_name: "anthropic/claude-opus-4.7",
            constructor_name: anthropic_claude_opus_4_7,
            display_name: "Claude Opus 4.7",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus48 {
            model_name: "anthropic/claude-opus-4.8",
            constructor_name: anthropic_claude_opus_4_8,
            display_name: "Claude Opus 4.8",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet46 {
            model_name: "anthropic/claude-sonnet-4.6",
            constructor_name: anthropic_claude_sonnet_4_6,
            display_name: "Claude Sonnet 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV4Flash {
            model_name: "deepseek-ai/DeepSeek-V4-Flash",
            constructor_name: deepseek_ai_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV4Pro {
            model_name: "deepseek-ai/DeepSeek-V4-Pro",
            constructor_name: deepseek_ai_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK26 {
            model_name: "moonshotai/Kimi-K2.6",
            constructor_name: moonshotai_kimi_k2_6,
            display_name: "Kimi K2.6",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK27CodeHighspeed {
            model_name: "moonshotai/kimi-k2.7-code-highspeed",
            constructor_name: moonshotai_kimi_k2_7_code_highspeed,
            display_name: "Kimi K2.7 Code Highspeed",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt55 {
            model_name: "openai/gpt-5.5",
            constructor_name: openai_gpt_5_5,
            display_name: "GPT-5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm5Fp8 {
            model_name: "zai-org/GLM-5-FP8",
            constructor_name: zai_org_glm_5_fp8,
            display_name: "GLM-5",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm51Fp8 {
            model_name: "zai-org/GLM-5.1-FP8",
            constructor_name: zai_org_glm_5_1_fp8,
            display_name: "GLM-5.1",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm52Fp8 {
            model_name: "zai-org/GLM-5.2-FP8",
            constructor_name: zai_org_glm_5_2_fp8,
            display_name: "GLM-5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
