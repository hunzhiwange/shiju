//! Capabilities for sakana models.
//!
//! This module defines model types and their capabilities for sakana providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::sakana::Sakana;

model_capabilities! {
    provider: Sakana,
    models: {
        Fugu {
            model_name: "fugu",
            constructor_name: fugu,
            display_name: "Fugu",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FuguUltra {
            model_name: "fugu-ultra",
            constructor_name: fugu_ultra,
            display_name: "Fugu Ultra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FuguUltra20260615 {
            model_name: "fugu-ultra-20260615",
            constructor_name: fugu_ultra_20260615,
            display_name: "Fugu Ultra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        SakanaNamazu {
            model_name: "sakana-namazu",
            constructor_name: sakana_namazu,
            display_name: "Sakana Namazu",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
