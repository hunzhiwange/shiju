//! Capabilities for mixlayer models.
//!
//! This module defines model types and their capabilities for mixlayer providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::mixlayer::Mixlayer;

model_capabilities! {
    provider: Mixlayer,
    models: {
        QwenQwen35122bA10b {
            model_name: "qwen/qwen3.5-122b-a10b",
            constructor_name: qwen_qwen3_5_122b_a10b,
            display_name: "Qwen3.5 122B A10B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3527b {
            model_name: "qwen/qwen3.5-27b",
            constructor_name: qwen_qwen3_5_27b,
            display_name: "Qwen3.5 27B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3535bA3b {
            model_name: "qwen/qwen3.5-35b-a3b",
            constructor_name: qwen_qwen3_5_35b_a3b,
            display_name: "Qwen3.5 35B A3B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen35397bA17b {
            model_name: "qwen/qwen3.5-397b-a17b",
            constructor_name: qwen_qwen3_5_397b_a17b,
            display_name: "Qwen3.5 397B A17B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen359b {
            model_name: "qwen/qwen3.5-9b",
            constructor_name: qwen_qwen3_5_9b,
            display_name: "Qwen3.5 9B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
