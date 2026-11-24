//! Capabilities for pendra models.
//!
//! This module defines model types and their capabilities for pendra providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::pendra::Pendra;

model_capabilities! {
    provider: Pendra,
    models: {
        DeepseekV4Flash {
            model_name: "deepseek-v4-flash",
            constructor_name: deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm47Flash {
            model_name: "glm-4.7-flash",
            constructor_name: glm_4_7_flash,
            display_name: "GLM-4.7-Flash",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GptOss120b {
            model_name: "gpt-oss:120b",
            constructor_name: gpt_oss_120b,
            display_name: "GPT OSS 120B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Llama3370b {
            model_name: "llama3.3:70b",
            constructor_name: llama3_3_70b,
            display_name: "Llama-3.3-70B-Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3Coder30b {
            model_name: "qwen3-coder:30b",
            constructor_name: qwen3_coder_30b,
            display_name: "Qwen3-Coder 30B-A3B Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3627b {
            model_name: "qwen3.6:27b",
            constructor_name: qwen3_6_27b,
            display_name: "Qwen3.6 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
