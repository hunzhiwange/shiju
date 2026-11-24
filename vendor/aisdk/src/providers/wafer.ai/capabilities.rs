//! Capabilities for wafer_ai models.
//!
//! This module defines model types and their capabilities for wafer_ai providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::wafer_ai::WaferAi;

model_capabilities! {
    provider: WaferAi,
    models: {
        Glm52 {
            model_name: "GLM-5.2",
            constructor_name: glm_5_2,
            display_name: "GLM-5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
