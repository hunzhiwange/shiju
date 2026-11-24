//! Capabilities for runinfra models.
//!
//! This module defines model types and their capabilities for runinfra providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::runinfra::Runinfra;

model_capabilities! {
    provider: Runinfra,
    models: {
        QwenQwen3827b {
            model_name: "Qwen/Qwen3.8-27B",
            constructor_name: qwen_qwen3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NvidiaNvidiaNemotron35Lightning30bA3bBf16 {
            model_name: "nvidia/NVIDIA-Nemotron-3.5-Lightning-30B-A3B-BF16",
            constructor_name: nvidia_nvidia_nemotron_3_5_lightning_30b_a3b_bf16,
            display_name: "Nemotron 3.5 Lightning 30B A3B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OrnithAiOrnith1535bA3b {
            model_name: "ornith-ai/Ornith-1.5-35B-A3B",
            constructor_name: ornith_ai_ornith_1_5_35b_a3b,
            display_name: "Ornith 1.5 35B A3B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
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
    }
}
