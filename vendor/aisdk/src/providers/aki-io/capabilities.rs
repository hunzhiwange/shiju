//! Capabilities for aki_io models.
//!
//! This module defines model types and their capabilities for aki_io providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::aki_io::AkiIo;

model_capabilities! {
    provider: AkiIo,
    models: {
        DeepseekV41Flash552b {
            model_name: "deepseek-v4.1-flash-552b",
            constructor_name: deepseek_v4_1_flash_552b,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Gemma426b {
            model_name: "gemma4-26b",
            constructor_name: gemma4_26b,
            display_name: "Gemma 4 26B A4B IT",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53754b {
            model_name: "glm5.3-754b",
            constructor_name: glm5_3_754b,
            display_name: "GLM-5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GptOss120b {
            model_name: "gpt-oss-120b",
            constructor_name: gpt_oss_120b,
            display_name: "GPT OSS 120B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Mistral4119b {
            model_name: "mistral4-119b",
            constructor_name: mistral4_119b,
            display_name: "Mistral Small 4",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3827b {
            model_name: "qwen3.8-27b",
            constructor_name: qwen3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
