//! Capabilities for zenifra models.
//!
//! This module defines model types and their capabilities for zenifra providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::zenifra::Zenifra;

model_capabilities! {
    provider: Zenifra,
    models: {
        AlibabaQwen3635bA3b {
            model_name: "alibaba/qwen3.6-35b-a3b",
            constructor_name: alibaba_qwen3_6_35b_a3b,
            display_name: "Qwen3.6 35B-A3B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
