//! Capabilities for zeldoc models.
//!
//! This module defines model types and their capabilities for zeldoc providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::zeldoc::Zeldoc;

model_capabilities! {
    provider: Zeldoc,
    models: {
        Zdev {
            model_name: "zdev",
            constructor_name: zdev,
            display_name: "ZDev",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
