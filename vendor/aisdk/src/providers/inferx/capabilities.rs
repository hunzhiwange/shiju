//! Capabilities for inferx models.
//!
//! This module defines model types and their capabilities for inferx providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::inferx::Inferx;

model_capabilities! {
    provider: Inferx,
    models: {
        AgentsA1 {
            model_name: "Agents-A1",
            constructor_name: agents_a1,
            display_name: "Agents-A1",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Devstral2123bInstruct2512Int4Autoround {
            model_name: "Devstral-2-123B-Instruct-2512-int4-AutoRound",
            constructor_name: devstral_2_123b_instruct_2512_int4_autoround,
            display_name: "Devstral-2-123B-Instruct-2512-int4-AutoRound",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Ornith1035bFp8 {
            model_name: "Ornith-1.0-35B-FP8",
            constructor_name: ornith_1_0_35b_fp8,
            display_name: "Ornith-1.0-35B-FP8",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3CoderNextFp8 {
            model_name: "Qwen3-Coder-Next-FP8",
            constructor_name: qwen3_coder_next_fp8,
            display_name: "Qwen3 Coder Next FP8",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3CoderNextFp8NoThinking {
            model_name: "Qwen3-Coder-Next-FP8-no-thinking",
            constructor_name: qwen3_coder_next_fp8_no_thinking,
            display_name: "Qwen3-Coder-Next-FP8-no-thinking",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3Embedding8b {
            model_name: "Qwen3-Embedding-8B",
            constructor_name: qwen3_embedding_8b,
            display_name: "Qwen3-Embedding-8B",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        Qwen3627bFp8 {
            model_name: "Qwen3.6-27B-FP8",
            constructor_name: qwen3_6_27b_fp8,
            display_name: "Qwen3.6 27B FP8",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen3635bA3bFp8 {
            model_name: "Qwen3.6-35B-A3B-FP8",
            constructor_name: qwen3_6_35b_a3b_fp8,
            display_name: "Qwen3.6 35B A3B FP8",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen3635bA3bFp8NoThinking {
            model_name: "Qwen3.6-35B-A3B-fp8-no-thinking",
            constructor_name: qwen3_6_35b_a3b_fp8_no_thinking,
            display_name: "Qwen3.6-35B-A3B-fp8-no-thinking",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DeepseekV4Flash {
            model_name: "deepseek-v4-flash",
            constructor_name: deepseek_v4_flash,
            display_name: "deepseek-v4-flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Gemma431bItFp8 {
            model_name: "gemma-4-31B-it-fp8",
            constructor_name: gemma_4_31b_it_fp8,
            display_name: "Gemma 4 31B IT FP8",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MimoV25 {
            model_name: "mimo-v25",
            constructor_name: mimo_v25,
            display_name: "mimo-v25",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
