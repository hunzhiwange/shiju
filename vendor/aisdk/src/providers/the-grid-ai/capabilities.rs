//! Capabilities for the_grid_ai models.
//!
//! This module defines model types and their capabilities for the_grid_ai providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::the_grid_ai::TheGridAi;

model_capabilities! {
    provider: TheGridAi,
    models: {
        AgentMax {
            model_name: "agent-max",
            constructor_name: agent_max,
            display_name: "Agent Max",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AgentPrime {
            model_name: "agent-prime",
            constructor_name: agent_prime,
            display_name: "Agent Prime",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AgentStandard {
            model_name: "agent-standard",
            constructor_name: agent_standard,
            display_name: "Agent Standard",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceProLatest {
            model_name: "bytedance-pro-latest",
            constructor_name: bytedance_pro_latest,
            display_name: "ByteDance Pro Latest",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ClaudeOpusLatest {
            model_name: "claude-opus-latest",
            constructor_name: claude_opus_latest,
            display_name: "Claude Opus Latest",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CodeMax {
            model_name: "code-max",
            constructor_name: code_max,
            display_name: "Code Max",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CodePrime {
            model_name: "code-prime",
            constructor_name: code_prime,
            display_name: "Code Prime",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CodeStandard {
            model_name: "code-standard",
            constructor_name: code_standard,
            display_name: "Code Standard",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekProLatest {
            model_name: "deepseek-pro-latest",
            constructor_name: deepseek_pro_latest,
            display_name: "DeepSeek Pro Latest",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GeminiProLatest {
            model_name: "gemini-pro-latest",
            constructor_name: gemini_pro_latest,
            display_name: "Gemini Pro Latest",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlmLatest {
            model_name: "glm-latest",
            constructor_name: glm_latest,
            display_name: "GLM Latest",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GptSolLatest {
            model_name: "gpt-sol-latest",
            constructor_name: gpt_sol_latest,
            display_name: "GPT Sol Latest",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        KimiLatest {
            model_name: "kimi-latest",
            constructor_name: kimi_latest,
            display_name: "Kimi Latest",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxLatest {
            model_name: "minimax-latest",
            constructor_name: minimax_latest,
            display_name: "MiniMax Latest",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TextMax {
            model_name: "text-max",
            constructor_name: text_max,
            display_name: "Text Max",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TextPrime {
            model_name: "text-prime",
            constructor_name: text_prime,
            display_name: "Text Prime",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TextStandard {
            model_name: "text-standard",
            constructor_name: text_standard,
            display_name: "Text Standard",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
