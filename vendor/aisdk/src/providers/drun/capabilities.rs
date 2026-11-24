//! Capabilities for drun models.
//!
//! This module defines model types and their capabilities for drun providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::drun::Drun;

model_capabilities! {
    provider: Drun,
    models: {
        PublicDeepseekR1 {
            model_name: "public/deepseek-r1",
            constructor_name: public_deepseek_r1,
            display_name: "DeepSeek R1",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        PublicDeepseekV3 {
            model_name: "public/deepseek-v3",
            constructor_name: public_deepseek_v3,
            display_name: "DeepSeek V3",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        PublicMinimaxM25 {
            model_name: "public/minimax-m25",
            constructor_name: public_minimax_m25,
            display_name: "MiniMax M2.5",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
