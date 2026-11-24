//! Capabilities for trustedrouter models.
//!
//! This module defines model types and their capabilities for trustedrouter providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::trustedrouter::Trustedrouter;

model_capabilities! {
    provider: Trustedrouter,
    models: {
        TrustedrouterAuto {
            model_name: "trustedrouter/auto",
            constructor_name: trustedrouter_auto,
            display_name: "Auto",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TrustedrouterCheap {
            model_name: "trustedrouter/cheap",
            constructor_name: trustedrouter_cheap,
            display_name: "Cheap",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TrustedrouterE2e {
            model_name: "trustedrouter/e2e",
            constructor_name: trustedrouter_e2e,
            display_name: "End-to-End Encrypted",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TrustedrouterFast {
            model_name: "trustedrouter/fast",
            constructor_name: trustedrouter_fast,
            display_name: "Fast",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TrustedrouterSynth {
            model_name: "trustedrouter/synth",
            constructor_name: trustedrouter_synth,
            display_name: "Synth",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TrustedrouterSynthCode {
            model_name: "trustedrouter/synth-code",
            constructor_name: trustedrouter_synth_code,
            display_name: "Synth Code",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TrustedrouterZdr {
            model_name: "trustedrouter/zdr",
            constructor_name: trustedrouter_zdr,
            display_name: "Zero Data Retention",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
