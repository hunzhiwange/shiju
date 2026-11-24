//! Capabilities for longcat models.
//!
//! This module defines model types and their capabilities for longcat providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::longcat::Longcat;

model_capabilities! {
    provider: Longcat,
    models: {
        Longcat20 {
            model_name: "LongCat-2.0",
            constructor_name: longcat_2_0,
            display_name: "LongCat-2.0",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Longcat25Preview {
            model_name: "LongCat-2.5-Preview",
            constructor_name: longcat_2_5_preview,
            display_name: "LongCat-2.5-Preview",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
