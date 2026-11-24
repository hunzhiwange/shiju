//! Capabilities for umans_ai_coding_plan models.
//!
//! This module defines model types and their capabilities for umans_ai_coding_plan providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::umans_ai_coding_plan::UmansAiCodingPlan;

model_capabilities! {
    provider: UmansAiCodingPlan,
    models: {
        UmansCoder {
            model_name: "umans-coder",
            constructor_name: umans_coder,
            display_name: "Umans Coder",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        UmansDeepseekV4Flash0731 {
            model_name: "umans-deepseek-v4-flash-0731",
            constructor_name: umans_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UmansDeepseekV41Flash {
            model_name: "umans-deepseek-v4.1-flash",
            constructor_name: umans_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UmansDeepseekV41Flash1017 {
            model_name: "umans-deepseek-v4.1-flash-1017",
            constructor_name: umans_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UmansFlash {
            model_name: "umans-flash",
            constructor_name: umans_flash,
            display_name: "Umans Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UmansGlm53 {
            model_name: "umans-glm-5.3",
            constructor_name: umans_glm_5_3,
            display_name: "GLM 5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UmansGlm53Flash {
            model_name: "umans-glm-5.3-flash",
            constructor_name: umans_glm_5_3_flash,
            display_name: "GLM 5.3 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        UmansKimiK3 {
            model_name: "umans-kimi-k3",
            constructor_name: umans_kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UmansQwen3635bA3b {
            model_name: "umans-qwen3.6-35b-a3b",
            constructor_name: umans_qwen3_6_35b_a3b,
            display_name: "Qwen3.6 35B A3B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
