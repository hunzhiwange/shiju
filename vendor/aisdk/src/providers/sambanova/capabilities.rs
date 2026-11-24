//! Capabilities for sambanova models.
//!
//! This module defines model types and their capabilities for sambanova providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::sambanova::Sambanova;

model_capabilities! {
    provider: Sambanova,
    models: {
        DeepseekV31 {
            model_name: "DeepSeek-V3.1",
            constructor_name: deepseek_v3_1,
            display_name: "DeepSeek-V3.1",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV32 {
            model_name: "DeepSeek-V3.2",
            constructor_name: deepseek_v3_2,
            display_name: "DeepSeek V3.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaLlama3370bInstruct {
            model_name: "Meta-Llama-3.3-70B-Instruct",
            constructor_name: meta_llama_3_3_70b_instruct,
            display_name: "Llama-3.3-70B-Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxM3 {
            model_name: "MiniMax-M3",
            constructor_name: minimax_m3,
            display_name: "MiniMax-M3",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Gemma431bIt {
            model_name: "gemma-4-31B-it",
            constructor_name: gemma_4_31b_it,
            display_name: "Gemma 4 31B IT",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GptOss120b {
            model_name: "gpt-oss-120b",
            constructor_name: gpt_oss_120b,
            display_name: "GPT OSS 120B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
