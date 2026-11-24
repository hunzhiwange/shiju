//! Capabilities for siliconflow models.
//!
//! This module defines model types and their capabilities for siliconflow providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::siliconflow::Siliconflow;

model_capabilities! {
    provider: Siliconflow,
    models: {
        BytedanceSeedSeedOss36bInstruct {
            model_name: "ByteDance-Seed/Seed-OSS-36B-Instruct",
            constructor_name: bytedance_seed_seed_oss_36b_instruct,
            display_name: "ByteDance-Seed/Seed-OSS-36B-Instruct",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxaiMinimaxM3 {
            model_name: "MiniMaxAI/MiniMax-M3",
            constructor_name: minimaxai_minimax_m3,
            display_name: "MiniMax-M3",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen2572bInstruct {
            model_name: "Qwen/Qwen2.5-72B-Instruct",
            constructor_name: qwen_qwen2_5_72b_instruct,
            display_name: "Qwen/Qwen2.5-72B-Instruct",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen257bInstruct {
            model_name: "Qwen/Qwen2.5-7B-Instruct",
            constructor_name: qwen_qwen2_5_7b_instruct,
            display_name: "Qwen/Qwen2.5-7B-Instruct",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen314b {
            model_name: "Qwen/Qwen3-14B",
            constructor_name: qwen_qwen3_14b,
            display_name: "Qwen/Qwen3-14B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen330bA3bInstruct2507 {
            model_name: "Qwen/Qwen3-30B-A3B-Instruct-2507",
            constructor_name: qwen_qwen3_30b_a3b_instruct_2507,
            display_name: "Qwen/Qwen3-30B-A3B-Instruct-2507",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen332b {
            model_name: "Qwen/Qwen3-32B",
            constructor_name: qwen_qwen3_32b,
            display_name: "Qwen/Qwen3-32B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen38b {
            model_name: "Qwen/Qwen3-8B",
            constructor_name: qwen_qwen3_8b,
            display_name: "Qwen/Qwen3-8B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3Coder30bA3bInstruct {
            model_name: "Qwen/Qwen3-Coder-30B-A3B-Instruct",
            constructor_name: qwen_qwen3_coder_30b_a3b_instruct,
            display_name: "Qwen/Qwen3-Coder-30B-A3B-Instruct",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3Coder480bA35bInstruct {
            model_name: "Qwen/Qwen3-Coder-480B-A35B-Instruct",
            constructor_name: qwen_qwen3_coder_480b_a35b_instruct,
            display_name: "Qwen/Qwen3-Coder-480B-A35B-Instruct",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3Vl30bA3bInstruct {
            model_name: "Qwen/Qwen3-VL-30B-A3B-Instruct",
            constructor_name: qwen_qwen3_vl_30b_a3b_instruct,
            display_name: "Qwen/Qwen3-VL-30B-A3B-Instruct",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3Vl30bA3bThinking {
            model_name: "Qwen/Qwen3-VL-30B-A3B-Thinking",
            constructor_name: qwen_qwen3_vl_30b_a3b_thinking,
            display_name: "Qwen/Qwen3-VL-30B-A3B-Thinking",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3Vl32bInstruct {
            model_name: "Qwen/Qwen3-VL-32B-Instruct",
            constructor_name: qwen_qwen3_vl_32b_instruct,
            display_name: "Qwen/Qwen3-VL-32B-Instruct",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3Vl32bThinking {
            model_name: "Qwen/Qwen3-VL-32B-Thinking",
            constructor_name: qwen_qwen3_vl_32b_thinking,
            display_name: "Qwen/Qwen3-VL-32B-Thinking",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3Vl8bInstruct {
            model_name: "Qwen/Qwen3-VL-8B-Instruct",
            constructor_name: qwen_qwen3_vl_8b_instruct,
            display_name: "Qwen/Qwen3-VL-8B-Instruct",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen35122bA10b {
            model_name: "Qwen/Qwen3.5-122B-A10B",
            constructor_name: qwen_qwen3_5_122b_a10b,
            display_name: "Qwen3.5 122B-A10B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3527b {
            model_name: "Qwen/Qwen3.5-27B",
            constructor_name: qwen_qwen3_5_27b,
            display_name: "Qwen3.5 27B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3535bA3b {
            model_name: "Qwen/Qwen3.5-35B-A3B",
            constructor_name: qwen_qwen3_5_35b_a3b,
            display_name: "Qwen3.5 35B-A3B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen359b {
            model_name: "Qwen/Qwen3.5-9B",
            constructor_name: qwen_qwen3_5_9b,
            display_name: "Qwen/Qwen3.5-9B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3627b {
            model_name: "Qwen/Qwen3.6-27B",
            constructor_name: qwen_qwen3_6_27b,
            display_name: "Qwen3.6 27B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3635bA3b {
            model_name: "Qwen/Qwen3.6-35B-A3B",
            constructor_name: qwen_qwen3_6_35b_a3b,
            display_name: "Qwen3.6 35B-A3B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3824tA95b {
            model_name: "Qwen/Qwen3.8-2.4T-A95B",
            constructor_name: qwen_qwen3_8_2_4t_a95b,
            display_name: "Qwen3.8 2.4T A95B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekR1 {
            model_name: "deepseek-ai/DeepSeek-R1",
            constructor_name: deepseek_ai_deepseek_r1,
            display_name: "deepseek-ai/DeepSeek-R1",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV3 {
            model_name: "deepseek-ai/DeepSeek-V3",
            constructor_name: deepseek_ai_deepseek_v3,
            display_name: "deepseek-ai/DeepSeek-V3",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV31 {
            model_name: "deepseek-ai/DeepSeek-V3.1",
            constructor_name: deepseek_ai_deepseek_v3_1,
            display_name: "deepseek-ai/DeepSeek-V3.1",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV31Terminus {
            model_name: "deepseek-ai/DeepSeek-V3.1-Terminus",
            constructor_name: deepseek_ai_deepseek_v3_1_terminus,
            display_name: "deepseek-ai/DeepSeek-V3.1-Terminus",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV32 {
            model_name: "deepseek-ai/DeepSeek-V3.2",
            constructor_name: deepseek_ai_deepseek_v3_2,
            display_name: "deepseek-ai/DeepSeek-V3.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV32Exp {
            model_name: "deepseek-ai/DeepSeek-V3.2-Exp",
            constructor_name: deepseek_ai_deepseek_v3_2_exp,
            display_name: "deepseek-ai/DeepSeek-V3.2-Exp",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV4Flash {
            model_name: "deepseek-ai/DeepSeek-V4-Flash",
            constructor_name: deepseek_ai_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV4Flash0731 {
            model_name: "deepseek-ai/DeepSeek-V4-Flash-0731",
            constructor_name: deepseek_ai_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV4FlashVisionExp {
            model_name: "deepseek-ai/DeepSeek-V4-Flash-Vision-Exp",
            constructor_name: deepseek_ai_deepseek_v4_flash_vision_exp,
            display_name: "DeepSeek V4 Flash Vision Exp",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV4Pro {
            model_name: "deepseek-ai/DeepSeek-V4-Pro",
            constructor_name: deepseek_ai_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV4Pro0813 {
            model_name: "deepseek-ai/DeepSeek-V4-Pro-0813",
            constructor_name: deepseek_ai_deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemma412bIt {
            model_name: "google/gemma-4-12B-it",
            constructor_name: google_gemma_4_12b_it,
            display_name: "Gemma 4 12B IT",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemma426bA4bIt {
            model_name: "google/gemma-4-26B-A4B-it",
            constructor_name: google_gemma_4_26b_a4b_it,
            display_name: "Gemma 4 26B A4B IT",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemma431bIt {
            model_name: "google/gemma-4-31B-it",
            constructor_name: google_gemma_4_31b_it,
            display_name: "Gemma 4 31B IT",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        InclusionaiLingFlash20 {
            model_name: "inclusionAI/Ling-flash-2.0",
            constructor_name: inclusionai_ling_flash_2_0,
            display_name: "inclusionAI/Ling-flash-2.0",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MeituanLongcatLongcat20 {
            model_name: "meituan-longcat/LongCat-2.0",
            constructor_name: meituan_longcat_longcat_2_0,
            display_name: "LongCat-2.0",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK25 {
            model_name: "moonshotai/Kimi-K2.5",
            constructor_name: moonshotai_kimi_k2_5,
            display_name: "moonshotai/Kimi-K2.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK26 {
            model_name: "moonshotai/Kimi-K2.6",
            constructor_name: moonshotai_kimi_k2_6,
            display_name: "moonshotai/Kimi-K2.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK27Code {
            model_name: "moonshotai/Kimi-K2.7-Code",
            constructor_name: moonshotai_kimi_k2_7_code,
            display_name: "Kimi K2.7 Code",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK3 {
            model_name: "moonshotai/Kimi-K3",
            constructor_name: moonshotai_kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOss120b {
            model_name: "openai/gpt-oss-120b",
            constructor_name: openai_gpt_oss_120b,
            display_name: "openai/gpt-oss-120b",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        OpenaiGptOss20b {
            model_name: "openai/gpt-oss-20b",
            constructor_name: openai_gpt_oss_20b,
            display_name: "openai/gpt-oss-20b",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        StepfunAiStep35Flash {
            model_name: "stepfun-ai/Step-3.5-Flash",
            constructor_name: stepfun_ai_step_3_5_flash,
            display_name: "stepfun-ai/Step-3.5-Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentHunyuanA13bInstruct {
            model_name: "tencent/Hunyuan-A13B-Instruct",
            constructor_name: tencent_hunyuan_a13b_instruct,
            display_name: "tencent/Hunyuan-A13B-Instruct",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        TencentHy3 {
            model_name: "tencent/Hy3",
            constructor_name: tencent_hy3,
            display_name: "Hy3",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm45Air {
            model_name: "zai-org/GLM-4.5-Air",
            constructor_name: zai_org_glm_4_5_air,
            display_name: "zai-org/GLM-4.5-Air",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm5 {
            model_name: "zai-org/GLM-5",
            constructor_name: zai_org_glm_5,
            display_name: "zai-org/GLM-5",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm51 {
            model_name: "zai-org/GLM-5.1",
            constructor_name: zai_org_glm_5_1,
            display_name: "zai-org/GLM-5.1",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm52 {
            model_name: "zai-org/GLM-5.2",
            constructor_name: zai_org_glm_5_2,
            display_name: "GLM-5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm53 {
            model_name: "zai-org/GLM-5.3",
            constructor_name: zai_org_glm_5_3,
            display_name: "GLM-5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm53Flash {
            model_name: "zai-org/GLM-5.3-Flash",
            constructor_name: zai_org_glm_5_3_flash,
            display_name: "GLM-5.3-Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm5vTurbo {
            model_name: "zai-org/GLM-5V-Turbo",
            constructor_name: zai_org_glm_5v_turbo,
            display_name: "zai-org/GLM-5V-Turbo",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
