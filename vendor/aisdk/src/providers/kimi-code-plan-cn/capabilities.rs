//! Capabilities for kimi_code_plan_cn models.
//!
//! This module defines model types and their capabilities for kimi_code_plan_cn providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::kimi_code_plan_cn::KimiCodePlanCn;

model_capabilities! {
    provider: KimiCodePlanCn,
    models: {
        K3 {
            model_name: "k3",
            constructor_name: k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        K3256k {
            model_name: "k3-256k",
            constructor_name: k3_256k,
            display_name: "Kimi K3-256K",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        KimiForCoding {
            model_name: "kimi-for-coding",
            constructor_name: kimi_for_coding,
            display_name: "Kimi K2.8 Preview",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        KimiForCodingHighspeed {
            model_name: "kimi-for-coding-highspeed",
            constructor_name: kimi_for_coding_highspeed,
            display_name: "Kimi For Coding HighSpeed",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
