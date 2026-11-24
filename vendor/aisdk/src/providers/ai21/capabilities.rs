//! Capabilities for ai21 models.
//!
//! This module defines model types and their capabilities for ai21 providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::ai21::Ai21;

model_capabilities! {
    provider: Ai21,
    models: {
        JambaLarge {
            model_name: "jamba-large",
            constructor_name: jamba_large,
            display_name: "Jamba Large",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        JambaMini {
            model_name: "jamba-mini",
            constructor_name: jamba_mini,
            display_name: "Jamba Mini",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
