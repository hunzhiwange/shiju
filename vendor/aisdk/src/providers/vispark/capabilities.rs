//! Capabilities for vispark models.
//!
//! This module defines model types and their capabilities for vispark providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::vispark::Vispark;

model_capabilities! {
    provider: Vispark,
    models: {
        VisparkVisionLarge {
            model_name: "vispark/vision-large",
            constructor_name: vispark_vision_large,
            display_name: "Vision Large",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VisparkVisionMedium {
            model_name: "vispark/vision-medium",
            constructor_name: vispark_vision_medium,
            display_name: "Vision Medium",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VisparkVisionSmall {
            model_name: "vispark/vision-small",
            constructor_name: vispark_vision_small,
            display_name: "Vision Small",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
