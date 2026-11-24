//! Capabilities for liquid models.
//!
//! This module defines model types and their capabilities for liquid providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::liquid::Liquid;

model_capabilities! {
    provider: Liquid,
    models: {
        LiquidAuto {
            model_name: "liquid.auto",
            constructor_name: liquid_auto,
            display_name: "Liquid Auto",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
