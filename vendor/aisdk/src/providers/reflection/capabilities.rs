//! Capabilities for reflection models.
//!
//! This module defines model types and their capabilities for reflection providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::reflection::Reflection;

model_capabilities! {
    provider: Reflection,
    models: {
        Beam10501bA23b {
            model_name: "Beam-1.0-501B-A23B",
            constructor_name: beam_1_0_501b_a23b,
            display_name: "Beam 1.0 501B A23B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
