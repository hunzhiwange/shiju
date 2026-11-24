//! Capabilities for agnes models.
//!
//! This module defines model types and their capabilities for agnes providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::agnes::Agnes;

model_capabilities! {
    provider: Agnes,
    models: {
        Agnes25Flash {
            model_name: "agnes-2.5-flash",
            constructor_name: agnes_2_5_flash,
            display_name: "Agnes 2.5 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Agnes25ProAlpha {
            model_name: "agnes-2.5-pro-alpha",
            constructor_name: agnes_2_5_pro_alpha,
            display_name: "Agnes 2.5 Pro Alpha",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Agnes30Flash {
            model_name: "agnes-3.0-flash",
            constructor_name: agnes_3_0_flash,
            display_name: "Agnes 3.0 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
