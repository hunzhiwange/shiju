//! Capabilities for coralbricks models.
//!
//! This module defines model types and their capabilities for coralbricks providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::coralbricks::Coralbricks;

model_capabilities! {
    provider: Coralbricks,
    models: {
        DeepseekV41FlashFast {
            model_name: "deepseek-v4.1-flash-fast",
            constructor_name: deepseek_v4_1_flash_fast,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
