//! Capabilities for ainetcafe models.
//!
//! This module defines model types and their capabilities for ainetcafe providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::ainetcafe::Ainetcafe;

model_capabilities! {
    provider: Ainetcafe,
    models: {
        KimiK3 {
            model_name: "Kimi-K3",
            constructor_name: kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
