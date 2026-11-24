//! Capabilities for wandb models.
//!
//! This module defines model types and their capabilities for wandb providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::wandb::Wandb;

model_capabilities! {
    provider: Wandb,
    models: {
        MinimaxaiMinimaxM3 {
            model_name: "MiniMaxAI/MiniMax-M3",
            constructor_name: minimaxai_minimax_m3,
            display_name: "MiniMax M3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3635bA3b {
            model_name: "Qwen/Qwen3.6-35B-A3B",
            constructor_name: qwen_qwen3_6_35b_a3b,
            display_name: "Qwen3.6 35B A3B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3827b {
            model_name: "Qwen/Qwen3.8-27B",
            constructor_name: qwen_qwen3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV31 {
            model_name: "deepseek-ai/DeepSeek-V3.1",
            constructor_name: deepseek_ai_deepseek_v3_1,
            display_name: "DeepSeek V3.1",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV4Flash0731 {
            model_name: "deepseek-ai/DeepSeek-V4-Flash-0731",
            constructor_name: deepseek_ai_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV4Pro0813 {
            model_name: "deepseek-ai/DeepSeek-V4-Pro-0813",
            constructor_name: deepseek_ai_deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV41Flash {
            model_name: "deepseek-ai/DeepSeek-V4.1-Flash",
            constructor_name: deepseek_ai_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemma426bA4bIt {
            model_name: "google/gemma-4-26B-A4B-it",
            constructor_name: google_gemma_4_26b_a4b_it,
            display_name: "Gemma 4 26B A4B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemma431bIt {
            model_name: "google/gemma-4-31B-it",
            constructor_name: google_gemma_4_31b_it,
            display_name: "Gemma 4 31B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        IbmGraniteGranite428b {
            model_name: "ibm-granite/granite-4.2-8b",
            constructor_name: ibm_granite_granite_4_2_8b,
            display_name: "Granite 4.2 8B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaLlamaLlama318bInstruct {
            model_name: "meta-llama/Llama-3.1-8B-Instruct",
            constructor_name: meta_llama_llama_3_1_8b_instruct,
            display_name: "Llama 3.1 8B",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaLlamaLlama3370bInstruct {
            model_name: "meta-llama/Llama-3.3-70B-Instruct",
            constructor_name: meta_llama_llama_3_3_70b_instruct,
            display_name: "Llama 3.3 70B",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK26 {
            model_name: "moonshotai/Kimi-K2.6",
            constructor_name: moonshotai_kimi_k2_6,
            display_name: "Kimi K2.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK27Code {
            model_name: "moonshotai/Kimi-K2.7-Code",
            constructor_name: moonshotai_kimi_k2_7_code,
            display_name: "Kimi K2.7 Code",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NvidiaNvidiaNemotron3Ultra550bA55b {
            model_name: "nvidia/NVIDIA-Nemotron-3-Ultra-550B-A55B",
            constructor_name: nvidia_nvidia_nemotron_3_ultra_550b_a55b,
            display_name: "Nemotron 3 Ultra",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NvidiaNvidiaNemotron35Lightning30bA3b {
            model_name: "nvidia/NVIDIA-Nemotron-3.5-Lightning-30B-A3B",
            constructor_name: nvidia_nvidia_nemotron_3_5_lightning_30b_a3b,
            display_name: "Nemotron 3.5 Lightning",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NvidiaNvidiaNemotron35SuperVl120bA12b {
            model_name: "nvidia/NVIDIA-Nemotron-3.5-Super-VL-120B-A12B",
            constructor_name: nvidia_nvidia_nemotron_3_5_super_vl_120b_a12b,
            display_name: "Nemotron 3.5 Super VL",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOss120b {
            model_name: "openai/gpt-oss-120b",
            constructor_name: openai_gpt_oss_120b,
            display_name: "gpt-oss-120b",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOss20b {
            model_name: "openai/gpt-oss-20b",
            constructor_name: openai_gpt_oss_20b,
            display_name: "gpt-oss-20b",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ReflectionBeam10501bA23b {
            model_name: "reflection/Beam-1.0-501B-A23B",
            constructor_name: reflection_beam_1_0_501b_a23b,
            display_name: "Beam 1.0 501B A23B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm52 {
            model_name: "zai-org/GLM-5.2",
            constructor_name: zai_org_glm_5_2,
            display_name: "GLM 5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm53 {
            model_name: "zai-org/GLM-5.3",
            constructor_name: zai_org_glm_5_3,
            display_name: "GLM 5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm53Flash {
            model_name: "zai-org/GLM-5.3-Flash",
            constructor_name: zai_org_glm_5_3_flash,
            display_name: "GLM 5.3 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
