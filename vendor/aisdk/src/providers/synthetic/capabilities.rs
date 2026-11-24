//! Capabilities for synthetic models.
//!
//! This module defines model types and their capabilities for synthetic providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::synthetic::Synthetic;

model_capabilities! {
    provider: Synthetic,
    models: {
        HfDeepseekAiDeepseekV41Flash {
            model_name: "hf:deepseek-ai/DeepSeek-V4.1-Flash",
            constructor_name: hf_deepseek_ai_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        HfDeepseekAiDeepseekV41Flash1017 {
            model_name: "hf:deepseek-ai/DeepSeek-V4.1-Flash-1017",
            constructor_name: hf_deepseek_ai_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        HfMoonshotaiKimiK3 {
            model_name: "hf:moonshotai/Kimi-K3",
            constructor_name: hf_moonshotai_kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        HfNvidiaNvidiaNemotron3Super120bA12bNvfp4 {
            model_name: "hf:nvidia/NVIDIA-Nemotron-3-Super-120B-A12B-NVFP4",
            constructor_name: hf_nvidia_nvidia_nemotron_3_super_120b_a12b_nvfp4,
            display_name: "Nemotron 3 Super 120B A12B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        HfOpenaiGptOss120b {
            model_name: "hf:openai/gpt-oss-120b",
            constructor_name: hf_openai_gpt_oss_120b,
            display_name: "GPT OSS 120B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        HfZaiOrgGlm47Flash {
            model_name: "hf:zai-org/GLM-4.7-Flash",
            constructor_name: hf_zai_org_glm_4_7_flash,
            display_name: "GLM-4.7-Flash",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        HfZaiOrgGlm53 {
            model_name: "hf:zai-org/GLM-5.3",
            constructor_name: hf_zai_org_glm_5_3,
            display_name: "GLM-5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        HfZaiOrgGlm53Flash {
            model_name: "hf:zai-org/GLM-5.3-Flash",
            constructor_name: hf_zai_org_glm_5_3_flash,
            display_name: "GLM-5.3-Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
