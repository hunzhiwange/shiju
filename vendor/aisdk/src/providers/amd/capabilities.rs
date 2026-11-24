//! Capabilities for amd models.
//!
//! This module defines model types and their capabilities for amd providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::amd::Amd;

model_capabilities! {
    provider: Amd,
    models: {
        DeepseekV4Flash {
            model_name: "DeepSeek-V4-Flash",
            constructor_name: deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash 0731",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV41Flash {
            model_name: "DeepSeek-V4.1-Flash",
            constructor_name: deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53Flash {
            model_name: "GLM-5.3-Flash",
            constructor_name: glm_5_3_flash,
            display_name: "GLM-5.3-Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Gemma431bIt {
            model_name: "Gemma-4-31B-IT",
            constructor_name: gemma_4_31b_it,
            display_name: "Gemma 4 31B IT",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MimoV26Flash {
            model_name: "MiMo-V2.6-Flash",
            constructor_name: mimo_v2_6_flash,
            display_name: "MiMo-V2.6-Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinicpmV471b {
            model_name: "MiniCPM-V-4.7-1B",
            constructor_name: minicpm_v_4_7_1b,
            display_name: "MiniCPM-V 4.7 1B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinicpmV4735bA3b {
            model_name: "MiniCPM-V-4.7-35B-A3B",
            constructor_name: minicpm_v_4_7_35b_a3b,
            display_name: "MiniCPM-V 4.7 35B-A3B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Minicpm52b {
            model_name: "MiniCPM5-2B",
            constructor_name: minicpm5_2b,
            display_name: "MiniCPM5-2B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3827b {
            model_name: "Qwen3.8-27B",
            constructor_name: qwen3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen38FlashNext {
            model_name: "Qwen3.8-Flash-Next",
            constructor_name: qwen3_8_flash_next,
            display_name: "Qwen3.8 Flash Next",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
