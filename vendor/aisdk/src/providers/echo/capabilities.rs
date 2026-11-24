//! Capabilities for echo models.
//!
//! This module defines model types and their capabilities for echo providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::echo::Echo;

model_capabilities! {
    provider: Echo,
    models: {
        EchoModel {
            model_name: "echo",
            constructor_name: echo_model,
            display_name: "Echo",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
