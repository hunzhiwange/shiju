//! Capabilities for io_net models.
//!
//! This module defines model types and their capabilities for io_net providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::io_net::IoNet;

model_capabilities! {
    provider: IoNet,
    models: {
        IntelQwen3Coder480bA35bInstructInt4MixedAr {
            model_name: "Intel/Qwen3-Coder-480B-A35B-Instruct-int4-mixed-ar",
            constructor_name: intel_qwen3_coder_480b_a35b_instruct_int4_mixed_ar,
            display_name: "Qwen 3 Coder 480B",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3Next80bA3bInstruct {
            model_name: "Qwen/Qwen3-Next-80B-A3B-Instruct",
            constructor_name: qwen_qwen3_next_80b_a3b_instruct,
            display_name: "Qwen 3 Next 80B Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekR10528 {
            model_name: "deepseek-ai/DeepSeek-R1-0528",
            constructor_name: deepseek_ai_deepseek_r1_0528,
            display_name: "DeepSeek R1",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaLlamaLlama3370bInstruct {
            model_name: "meta-llama/Llama-3.3-70B-Instruct",
            constructor_name: meta_llama_llama_3_3_70b_instruct,
            display_name: "Llama 3.3 70B Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaLlamaLlama4Maverick17b128eInstructFp8 {
            model_name: "meta-llama/Llama-4-Maverick-17B-128E-Instruct-FP8",
            constructor_name: meta_llama_llama_4_maverick_17b_128e_instruct_fp8,
            display_name: "Llama 4 Maverick 17B 128E Instruct",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMistralNemoInstruct2407 {
            model_name: "mistralai/Mistral-Nemo-Instruct-2407",
            constructor_name: mistralai_mistral_nemo_instruct_2407,
            display_name: "Mistral Nemo Instruct 2407",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK2Instruct0905 {
            model_name: "moonshotai/Kimi-K2-Instruct-0905",
            constructor_name: moonshotai_kimi_k2_instruct_0905,
            display_name: "Kimi K2 Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK2Thinking {
            model_name: "moonshotai/Kimi-K2-Thinking",
            constructor_name: moonshotai_kimi_k2_thinking,
            display_name: "Kimi K2 Thinking",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOss120b {
            model_name: "openai/gpt-oss-120b",
            constructor_name: openai_gpt_oss_120b,
            display_name: "GPT-OSS 120B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOss20b {
            model_name: "openai/gpt-oss-20b",
            constructor_name: openai_gpt_oss_20b,
            display_name: "GPT-OSS 20B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm46 {
            model_name: "zai-org/GLM-4.6",
            constructor_name: zai_org_glm_4_6,
            display_name: "GLM 4.6",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
