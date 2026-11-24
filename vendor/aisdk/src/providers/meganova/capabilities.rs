//! Capabilities for meganova models.
//!
//! This module defines model types and their capabilities for meganova providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::meganova::Meganova;

model_capabilities! {
    provider: Meganova,
    models: {
        MinimaxaiMinimaxM21 {
            model_name: "MiniMaxAI/MiniMax-M2.1",
            constructor_name: minimaxai_minimax_m2_1,
            display_name: "MiniMax M2.1",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxaiMinimaxM25 {
            model_name: "MiniMaxAI/MiniMax-M2.5",
            constructor_name: minimaxai_minimax_m2_5,
            display_name: "MiniMax M2.5",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen25Vl32bInstruct {
            model_name: "Qwen/Qwen2.5-VL-32B-Instruct",
            constructor_name: qwen_qwen2_5_vl_32b_instruct,
            display_name: "Qwen2.5 VL 32B Instruct",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3235bA22bInstruct2507 {
            model_name: "Qwen/Qwen3-235B-A22B-Instruct-2507",
            constructor_name: qwen_qwen3_235b_a22b_instruct_2507,
            display_name: "Qwen3 235B A22B Instruct 2507",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen35Plus {
            model_name: "Qwen/Qwen3.5-Plus",
            constructor_name: qwen_qwen3_5_plus,
            display_name: "Qwen3.5 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DeepseekAiDeepseekR10528 {
            model_name: "deepseek-ai/DeepSeek-R1-0528",
            constructor_name: deepseek_ai_deepseek_r1_0528,
            display_name: "DeepSeek R1 0528",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        DeepseekAiDeepseekV30324 {
            model_name: "deepseek-ai/DeepSeek-V3-0324",
            constructor_name: deepseek_ai_deepseek_v3_0324,
            display_name: "DeepSeek V3 0324",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV31 {
            model_name: "deepseek-ai/DeepSeek-V3.1",
            constructor_name: deepseek_ai_deepseek_v3_1,
            display_name: "DeepSeek V3.1",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV32 {
            model_name: "deepseek-ai/DeepSeek-V3.2",
            constructor_name: deepseek_ai_deepseek_v3_2,
            display_name: "DeepSeek V3.2",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV32Exp {
            model_name: "deepseek-ai/DeepSeek-V3.2-Exp",
            constructor_name: deepseek_ai_deepseek_v3_2_exp,
            display_name: "DeepSeek V3.2 Exp",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaLlamaLlama3370bInstruct {
            model_name: "meta-llama/Llama-3.3-70B-Instruct",
            constructor_name: meta_llama_llama_3_3_70b_instruct,
            display_name: "Llama 3.3 70B Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMistralNemoInstruct2407 {
            model_name: "mistralai/Mistral-Nemo-Instruct-2407",
            constructor_name: mistralai_mistral_nemo_instruct_2407,
            display_name: "Mistral Nemo Instruct 2407",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMistralSmall3224bInstruct2506 {
            model_name: "mistralai/Mistral-Small-3.2-24B-Instruct-2506",
            constructor_name: mistralai_mistral_small_3_2_24b_instruct_2506,
            display_name: "Mistral Small 3.2 24B Instruct",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK2Thinking {
            model_name: "moonshotai/Kimi-K2-Thinking",
            constructor_name: moonshotai_kimi_k2_thinking,
            display_name: "Kimi K2 Thinking",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK25 {
            model_name: "moonshotai/Kimi-K2.5",
            constructor_name: moonshotai_kimi_k2_5,
            display_name: "Kimi K2.5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm46 {
            model_name: "zai-org/GLM-4.6",
            constructor_name: zai_org_glm_4_6,
            display_name: "GLM-4.6",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm47 {
            model_name: "zai-org/GLM-4.7",
            constructor_name: zai_org_glm_4_7,
            display_name: "GLM-4.7",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm5 {
            model_name: "zai-org/GLM-5",
            constructor_name: zai_org_glm_5,
            display_name: "GLM-5",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
