//! Capabilities for abliteration_ai models.
//!
//! This module defines model types and their capabilities for abliteration_ai providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::abliteration_ai::AbliterationAi;

model_capabilities! {
    provider: AbliterationAi,
    models: {
        AbliteratedModel {
            model_name: "abliterated-model",
            constructor_name: abliterated_model,
            display_name: "Abliterated Model",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AbliteratedModelLarge {
            model_name: "abliterated-model-large",
            constructor_name: abliterated_model_large,
            display_name: "Abliterated Model Large",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AbliteratedModelLargeV2 {
            model_name: "abliterated-model-large-v2",
            constructor_name: abliterated_model_large_v2,
            display_name: "Abliterated Model Large V2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
