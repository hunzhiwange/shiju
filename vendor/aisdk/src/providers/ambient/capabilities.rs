//! Capabilities for ambient models.
//!
//! This module defines model types and their capabilities for ambient providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::ambient::Ambient;

model_capabilities! {
    provider: Ambient,
    models: {
        AmbientLarge {
            model_name: "ambient/large",
            constructor_name: ambient_large,
            display_name: "Ambient Large",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmbientSmall {
            model_name: "ambient/small",
            constructor_name: ambient_small,
            display_name: "Ambient Small",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM3 {
            model_name: "minimax/minimax-m3",
            constructor_name: minimax_minimax_m3,
            display_name: "MiniMax-M3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen330bA3bInstruct2507 {
            model_name: "qwen/qwen3-30b-a3b-instruct-2507",
            constructor_name: qwen_qwen3_30b_a3b_instruct_2507,
            display_name: "Qwen3 30B-A3B Instruct 2507",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3627b {
            model_name: "qwen/qwen3.6-27b",
            constructor_name: qwen_qwen3_6_27b,
            display_name: "Qwen3.6 27B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3635bA3b {
            model_name: "qwen/qwen3.6-35b-a3b",
            constructor_name: qwen_qwen3_6_35b_a3b,
            display_name: "Qwen3.6 35B-A3B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3827b {
            model_name: "qwen/qwen3.8-27b",
            constructor_name: qwen_qwen3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UnslothQwen3827bGguf {
            model_name: "unsloth/qwen3.8-27b-gguf",
            constructor_name: unsloth_qwen3_8_27b_gguf,
            display_name: "qwen3.8-27b-4q",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZAiGlm52 {
            model_name: "z-ai/glm-5.2",
            constructor_name: z_ai_glm_5_2,
            display_name: "GLM-5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
