//! Capabilities for llmtech models.
//!
//! This module defines model types and their capabilities for llmtech providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::llmtech::Llmtech;

model_capabilities! {
    provider: Llmtech,
    models: {
        NvidiaQwen3827bNvfp4 {
            model_name: "nvidia/Qwen3.8-27B-NVFP4",
            constructor_name: nvidia_qwen3_8_27b_nvfp4,
            display_name: "Qwen3.8 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
