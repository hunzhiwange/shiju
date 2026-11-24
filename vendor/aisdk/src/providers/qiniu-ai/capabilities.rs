//! Capabilities for qiniu_ai models.
//!
//! This module defines model types and their capabilities for qiniu_ai providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::qiniu_ai::QiniuAi;

model_capabilities! {
    provider: QiniuAi,
    models: {
        DoubaoSeed20Lite {
            model_name: "doubao-seed-2.0-lite",
            constructor_name: doubao_seed_2_0_lite,
            display_name: "Doubao Seed 2.0 Lite",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DoubaoSeed20Mini {
            model_name: "doubao-seed-2.0-mini",
            constructor_name: doubao_seed_2_0_mini,
            display_name: "Doubao Seed 2.0 Mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        KlingV26 {
            model_name: "kling-v2-6",
            constructor_name: kling_v2_6,
            display_name: "Kling-V2 6",
            capabilities: [ImageInputSupport, TextInputSupport, VideoInputSupport, VideoOutputSupport]
        },
        MinimaxMinimaxM25 {
            model_name: "minimax/minimax-m2.5",
            constructor_name: minimax_minimax_m2_5,
            display_name: "Minimax/Minimax-M2.5",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen35397bA17b {
            model_name: "qwen3.5-397b-a17b",
            constructor_name: qwen3_5_397b_a17b,
            display_name: "Qwen3.5 397B A17B",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZAiGlm5 {
            model_name: "z-ai/glm-5",
            constructor_name: z_ai_glm_5,
            display_name: "Z-Ai/GLM 5",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
