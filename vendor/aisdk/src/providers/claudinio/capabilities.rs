//! Capabilities for claudinio models.
//!
//! This module defines model types and their capabilities for claudinio providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::claudinio::Claudinio;

model_capabilities! {
    provider: Claudinio,
    models: {
        ClaudinioModel {
            model_name: "claudinio",
            constructor_name: claudinio_model,
            display_name: "Claudinio",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Claudius {
            model_name: "claudius",
            constructor_name: claudius,
            display_name: "Claudius",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
