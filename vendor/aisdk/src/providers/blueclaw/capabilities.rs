//! Capabilities for blueclaw models.
//!
//! This module defines model types and their capabilities for blueclaw providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::blueclaw::Blueclaw;

model_capabilities! {
    provider: Blueclaw,
    models: {
        Qwen3827b {
            model_name: "Qwen3.8-27B",
            constructor_name: qwen3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
