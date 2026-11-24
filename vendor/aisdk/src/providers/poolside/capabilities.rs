//! Capabilities for poolside models.
//!
//! This module defines model types and their capabilities for poolside providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::poolside::Poolside;

model_capabilities! {
    provider: Poolside,
    models: {
        PoolsideLagunaM1 {
            model_name: "poolside/laguna-m.1",
            constructor_name: poolside_laguna_m_1,
            display_name: "Laguna M.1",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        PoolsideLagunaS21 {
            model_name: "poolside/laguna-s-2.1",
            constructor_name: poolside_laguna_s_2_1,
            display_name: "Laguna S 2.1",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        PoolsideLagunaXs21 {
            model_name: "poolside/laguna-xs-2.1",
            constructor_name: poolside_laguna_xs_2_1,
            display_name: "Laguna XS 2.1",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
