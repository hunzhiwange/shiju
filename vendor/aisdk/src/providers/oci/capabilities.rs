//! Capabilities for oci models.
//!
//! This module defines model types and their capabilities for oci providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::oci::Oci;

model_capabilities! {
    provider: Oci,
    models: {
        MetaLlama4Maverick17b128eInstructFp8 {
            model_name: "meta.llama-4-maverick-17b-128e-instruct-fp8",
            constructor_name: meta_llama_4_maverick_17b_128e_instruct_fp8,
            display_name: "Llama 4 Maverick 17B Instruct",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        MetaLlama4Scout17b16eInstruct {
            model_name: "meta.llama-4-scout-17b-16e-instruct",
            constructor_name: meta_llama_4_scout_17b_16e_instruct,
            display_name: "Llama 4 Scout 17B Instruct",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        OpenaiGptOss120b {
            model_name: "openai.gpt-oss-120b",
            constructor_name: openai_gpt_oss_120b,
            display_name: "GPT OSS 120B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOss20b {
            model_name: "openai.gpt-oss-20b",
            constructor_name: openai_gpt_oss_20b,
            display_name: "GPT OSS 20B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok420NonReasoning {
            model_name: "xai.grok-4.20-non-reasoning",
            constructor_name: xai_grok_4_20_non_reasoning,
            display_name: "Grok 4.20 (Non-Reasoning)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok420Reasoning {
            model_name: "xai.grok-4.20-reasoning",
            constructor_name: xai_grok_4_20_reasoning,
            display_name: "Grok 4.20 (Reasoning)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok43 {
            model_name: "xai.grok-4.3",
            constructor_name: xai_grok_4_3,
            display_name: "Grok 4.3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok46 {
            model_name: "xai.grok-4.6",
            constructor_name: xai_grok_4_6,
            display_name: "Grok 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
