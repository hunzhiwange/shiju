//! Capabilities for tinfoil models.
//!
//! This module defines model types and their capabilities for tinfoil providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::tinfoil::Tinfoil;

model_capabilities! {
    provider: Tinfoil,
    models: {
        DeepseekV41Flash {
            model_name: "deepseek-v4-1-flash",
            constructor_name: deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Gemma431b {
            model_name: "gemma4-31b",
            constructor_name: gemma4_31b,
            display_name: "Gemma 4 31B IT",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53 {
            model_name: "glm-5-3",
            constructor_name: glm_5_3,
            display_name: "GLM-5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GptOss120b {
            model_name: "gpt-oss-120b",
            constructor_name: gpt_oss_120b,
            display_name: "gpt-oss-120b",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GptOssSafeguard120b {
            model_name: "gpt-oss-safeguard-120b",
            constructor_name: gpt_oss_safeguard_120b,
            display_name: "gpt-oss-safeguard-120b",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        KimiK3 {
            model_name: "kimi-k3",
            constructor_name: kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Llama3370b {
            model_name: "llama3-3-70b",
            constructor_name: llama3_3_70b,
            display_name: "Llama-3.3-70B-Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NomicEmbedText {
            model_name: "nomic-embed-text",
            constructor_name: nomic_embed_text,
            display_name: "Nomic Embed Text v1.5",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
    }
}
