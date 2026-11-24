//! Capabilities for modal models.
//!
//! This module defines model types and their capabilities for modal providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::modal::Modal;

model_capabilities! {
    provider: Modal,
    models: {
        QwenQwen3824tA95b {
            model_name: "Qwen/Qwen3.8-2.4T-A95B",
            constructor_name: qwen_qwen3_8_2_4t_a95b,
            display_name: "Qwen3.8-Max",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekAiDeepseekV41Flash {
            model_name: "deepseek-ai/DeepSeek-V4.1-Flash",
            constructor_name: deepseek_ai_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaModelsMuseSparkOpen816bA42b {
            model_name: "meta-models/Muse-Spark-Open-816B-A42B",
            constructor_name: meta_models_muse_spark_open_816b_a42b,
            display_name: "Muse Spark Open",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MoonshotaiKimiK3 {
            model_name: "moonshotai/Kimi-K3",
            constructor_name: moonshotai_kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NvidiaGlm53FlashNvfp4 {
            model_name: "nvidia/GLM-5.3-Flash-NVFP4",
            constructor_name: nvidia_glm_5_3_flash_nvfp4,
            display_name: "GLM-5.3-Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ThinkingmachinesInklingNvfp4 {
            model_name: "thinkingmachines/Inkling-NVFP4",
            constructor_name: thinkingmachines_inkling_nvfp4,
            display_name: "Inkling",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
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
            display_name: "GLM 5.3 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
