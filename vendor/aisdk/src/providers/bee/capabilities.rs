//! Capabilities for bee models.
//!
//! This module defines model types and their capabilities for bee providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::bee::Bee;

model_capabilities! {
    provider: Bee,
    models: {
        BeeBrood {
            model_name: "bee-brood",
            constructor_name: bee_brood,
            display_name: "Bee Brood 1.0",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BeeBuzz {
            model_name: "bee-buzz",
            constructor_name: bee_buzz,
            display_name: "Bee Buzz 1.0",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BeeCell {
            model_name: "bee-cell",
            constructor_name: bee_cell,
            display_name: "Bee Cell 1.0",
            capabilities: [AudioInputSupport, ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BeeComb {
            model_name: "bee-comb",
            constructor_name: bee_comb,
            display_name: "Bee Comb 2.0",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BeeHive {
            model_name: "bee-hive",
            constructor_name: bee_hive,
            display_name: "Bee Hive 1.0",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BeeSwarm {
            model_name: "bee-swarm",
            constructor_name: bee_swarm,
            display_name: "Bee Swarm 1.0",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
    }
}
