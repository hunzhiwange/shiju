//! Capabilities for chutes models.
//!
//! This module defines model types and their capabilities for chutes providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::chutes::Chutes;

model_capabilities! {
    provider: Chutes,
    models: {
        Nemotron3NanoOmni30bTee {
            model_name: "Nemotron-3-Nano-Omni-30B-TEE",
            constructor_name: nemotron_3_nano_omni_30b_tee,
            display_name: "Nemotron 3 Nano Omni 30B TEE",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        QwenQwen3235bA22bThinking2507Tee {
            model_name: "Qwen/Qwen3-235B-A22B-Thinking-2507-TEE",
            constructor_name: qwen_qwen3_235b_a22b_thinking_2507_tee,
            display_name: "Qwen3 235B A22B Thinking 2507 TEE",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen332bTee {
            model_name: "Qwen/Qwen3-32B-TEE",
            constructor_name: qwen_qwen3_32b_tee,
            display_name: "Qwen3 32B TEE",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen35397bA17bTee {
            model_name: "Qwen/Qwen3.5-397B-A17B-TEE",
            constructor_name: qwen_qwen3_5_397b_a17b_tee,
            display_name: "Qwen3.5 397B A17B TEE",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3627bTee {
            model_name: "Qwen/Qwen3.6-27B-TEE",
            constructor_name: qwen_qwen3_6_27b_tee,
            display_name: "Qwen3.6 27B TEE",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3827bTee {
            model_name: "Qwen/Qwen3.8-27B-TEE",
            constructor_name: qwen_qwen3_8_27b_tee,
            display_name: "Qwen3.8 27B TEE",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV32Tee {
            model_name: "deepseek-ai/DeepSeek-V3.2-TEE",
            constructor_name: deepseek_ai_deepseek_v3_2_tee,
            display_name: "DeepSeek V3.2 TEE",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV4Flash0731Tee {
            model_name: "deepseek-ai/DeepSeek-V4-Flash-0731-TEE",
            constructor_name: deepseek_ai_deepseek_v4_flash_0731_tee,
            display_name: "DeepSeek V4 Flash 0731 TEE",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemma431bTurboTee {
            model_name: "google/gemma-4-31B-turbo-TEE",
            constructor_name: google_gemma_4_31b_turbo_tee,
            display_name: "gemma 4 31B turbo TEE",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK26Tee {
            model_name: "moonshotai/Kimi-K2.6-TEE",
            constructor_name: moonshotai_kimi_k2_6_tee,
            display_name: "Kimi K2.6 TEE",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MoonshotaiKimiK3Tee {
            model_name: "moonshotai/Kimi-K3-TEE",
            constructor_name: moonshotai_kimi_k3_tee,
            display_name: "Kimi K3 TEE",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        UnslothMistralNemoInstruct2407Tee {
            model_name: "unsloth/Mistral-Nemo-Instruct-2407-TEE",
            constructor_name: unsloth_mistral_nemo_instruct_2407_tee,
            display_name: "Mistral Nemo Instruct 2407 TEE",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        ZaiOrgGlm51Tee {
            model_name: "zai-org/GLM-5.1-TEE",
            constructor_name: zai_org_glm_5_1_tee,
            display_name: "GLM 5.1 TEE",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm52Tee {
            model_name: "zai-org/GLM-5.2-TEE",
            constructor_name: zai_org_glm_5_2_tee,
            display_name: "GLM 5.2 TEE",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
