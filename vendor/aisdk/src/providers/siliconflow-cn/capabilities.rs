//! Capabilities for siliconflow_cn models.
//!
//! This module defines model types and their capabilities for siliconflow_cn providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::siliconflow_cn::SiliconflowCn;

model_capabilities! {
    provider: SiliconflowCn,
    models: {
        BytedanceSeedSeedOss36bInstruct {
            model_name: "ByteDance-Seed/Seed-OSS-36B-Instruct",
            constructor_name: bytedance_seed_seed_oss_36b_instruct,
            display_name: "ByteDance-Seed/Seed-OSS-36B-Instruct",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        PaddlepaddlePaddleocrVl15 {
            model_name: "PaddlePaddle/PaddleOCR-VL-1.5",
            constructor_name: paddlepaddle_paddleocr_vl_1_5,
            display_name: "PaddlePaddle/PaddleOCR-VL-1.5",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        ProDeepseekAiDeepseekR1 {
            model_name: "Pro/deepseek-ai/DeepSeek-R1",
            constructor_name: pro_deepseek_ai_deepseek_r1,
            display_name: "Pro/deepseek-ai/DeepSeek-R1",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ProDeepseekAiDeepseekV3 {
            model_name: "Pro/deepseek-ai/DeepSeek-V3",
            constructor_name: pro_deepseek_ai_deepseek_v3,
            display_name: "Pro/deepseek-ai/DeepSeek-V3",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ProDeepseekAiDeepseekV31Terminus {
            model_name: "Pro/deepseek-ai/DeepSeek-V3.1-Terminus",
            constructor_name: pro_deepseek_ai_deepseek_v3_1_terminus,
            display_name: "Pro/deepseek-ai/DeepSeek-V3.1-Terminus",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ProDeepseekAiDeepseekV32 {
            model_name: "Pro/deepseek-ai/DeepSeek-V3.2",
            constructor_name: pro_deepseek_ai_deepseek_v3_2,
            display_name: "Pro/deepseek-ai/DeepSeek-V3.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ProMoonshotaiKimiK26 {
            model_name: "Pro/moonshotai/Kimi-K2.6",
            constructor_name: pro_moonshotai_kimi_k2_6,
            display_name: "Pro/moonshotai/Kimi-K2.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ProZaiOrgGlm51 {
            model_name: "Pro/zai-org/GLM-5.1",
            constructor_name: pro_zai_org_glm_5_1,
            display_name: "Pro/zai-org/GLM-5.1",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
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
        QwenQwen3Vl30bA3bInstruct {
            model_name: "Qwen/Qwen3-VL-30B-A3B-Instruct",
            constructor_name: qwen_qwen3_vl_30b_a3b_instruct,
            display_name: "Qwen/Qwen3-VL-30B-A3B-Instruct",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen3Vl30bA3bThinking {
            model_name: "Qwen/Qwen3-VL-30B-A3B-Thinking",
            constructor_name: qwen_qwen3_vl_30b_a3b_thinking,
            display_name: "Qwen/Qwen3-VL-30B-A3B-Thinking",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen3Vl32bInstruct {
            model_name: "Qwen/Qwen3-VL-32B-Instruct",
            constructor_name: qwen_qwen3_vl_32b_instruct,
            display_name: "Qwen/Qwen3-VL-32B-Instruct",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen3Vl32bThinking {
            model_name: "Qwen/Qwen3-VL-32B-Thinking",
            constructor_name: qwen_qwen3_vl_32b_thinking,
            display_name: "Qwen/Qwen3-VL-32B-Thinking",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen3Vl8bInstruct {
            model_name: "Qwen/Qwen3-VL-8B-Instruct",
            constructor_name: qwen_qwen3_vl_8b_instruct,
            display_name: "Qwen/Qwen3-VL-8B-Instruct",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen35122bA10b {
            model_name: "Qwen/Qwen3.5-122B-A10B",
            constructor_name: qwen_qwen3_5_122b_a10b,
            display_name: "Qwen/Qwen3.5-122B-A10B",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen3527b {
            model_name: "Qwen/Qwen3.5-27B",
            constructor_name: qwen_qwen3_5_27b,
            display_name: "Qwen/Qwen3.5-27B",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen3535bA3b {
            model_name: "Qwen/Qwen3.5-35B-A3B",
            constructor_name: qwen_qwen3_5_35b_a3b,
            display_name: "Qwen/Qwen3.5-35B-A3B",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen354b {
            model_name: "Qwen/Qwen3.5-4B",
            constructor_name: qwen_qwen3_5_4b,
            display_name: "Qwen/Qwen3.5-4B",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen359b {
            model_name: "Qwen/Qwen3.5-9B",
            constructor_name: qwen_qwen3_5_9b,
            display_name: "Qwen/Qwen3.5-9B",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen3635bA3b {
            model_name: "Qwen/Qwen3.6-35B-A3B",
            constructor_name: qwen_qwen3_6_35b_a3b,
            display_name: "Qwen/Qwen3.6-35B-A3B",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DeepseekAiDeepseekOcr {
            model_name: "deepseek-ai/DeepSeek-OCR",
            constructor_name: deepseek_ai_deepseek_ocr,
            display_name: "deepseek-ai/DeepSeek-OCR",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
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
        DeepseekAiDeepseekV4Flash {
            model_name: "deepseek-ai/DeepSeek-V4-Flash",
            constructor_name: deepseek_ai_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV4Pro {
            model_name: "deepseek-ai/DeepSeek-V4-Pro",
            constructor_name: deepseek_ai_deepseek_v4_pro,
            display_name: "deepseek-ai/DeepSeek-V4-Pro",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        InclusionaiLingFlash20 {
            model_name: "inclusionAI/Ling-flash-2.0",
            constructor_name: inclusionai_ling_flash_2_0,
            display_name: "inclusionAI/Ling-flash-2.0",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
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
        ZaiOrgGlm45Air {
            model_name: "zai-org/GLM-4.5-Air",
            constructor_name: zai_org_glm_4_5_air,
            display_name: "zai-org/GLM-4.5-Air",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm52 {
            model_name: "zai-org/GLM-5.2",
            constructor_name: zai_org_glm_5_2,
            display_name: "GLM-5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
