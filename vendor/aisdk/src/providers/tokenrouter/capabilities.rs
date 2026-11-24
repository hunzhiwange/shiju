//! Capabilities for tokenrouter models.
//!
//! This module defines model types and their capabilities for tokenrouter providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::tokenrouter::Tokenrouter;

model_capabilities! {
    provider: Tokenrouter,
    models: {
        ZAiGlm53Free {
            model_name: "z-ai/glm-5.3-free",
            constructor_name: z_ai_glm_5_3_free,
            display_name: "GLM-5.3 (free)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
