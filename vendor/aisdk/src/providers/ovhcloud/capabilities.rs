//! Capabilities for ovhcloud models.
//!
//! This module defines model types and their capabilities for ovhcloud providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::ovhcloud::Ovhcloud;

model_capabilities! {
    provider: Ovhcloud,
    models: {
        GptOss120b {
            model_name: "gpt-oss-120b",
            constructor_name: gpt_oss_120b,
            display_name: "gpt-oss-120b",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GptOss20b {
            model_name: "gpt-oss-20b",
            constructor_name: gpt_oss_20b,
            display_name: "gpt-oss-20b",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaLlama3370bInstruct {
            model_name: "meta-llama-3_3-70b-instruct",
            constructor_name: meta_llama_3_3_70b_instruct,
            display_name: "Meta-Llama-3_3-70B-Instruct",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralNemoInstruct2407 {
            model_name: "mistral-nemo-instruct-2407",
            constructor_name: mistral_nemo_instruct_2407,
            display_name: "Mistral-Nemo-Instruct-2407",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralSmall3224bInstruct2506 {
            model_name: "mistral-small-3.2-24b-instruct-2506",
            constructor_name: mistral_small_3_2_24b_instruct_2506,
            display_name: "Mistral-Small-3.2-24B-Instruct-2506",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen25Vl72bInstruct {
            model_name: "qwen2.5-vl-72b-instruct",
            constructor_name: qwen2_5_vl_72b_instruct,
            display_name: "Qwen2.5-VL-72B-Instruct",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        Qwen35397bA17b {
            model_name: "qwen3.5-397b-a17b",
            constructor_name: qwen3_5_397b_a17b,
            display_name: "Qwen3.5-397B-A17B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen359b {
            model_name: "qwen3.5-9b",
            constructor_name: qwen3_5_9b,
            display_name: "Qwen3.5-9B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3627b {
            model_name: "qwen3.6-27b",
            constructor_name: qwen3_6_27b,
            display_name: "Qwen3.6-27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3827b {
            model_name: "qwen3.8-27b",
            constructor_name: qwen3_8_27b,
            display_name: "Qwen3.8-27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3guardGen06b {
            model_name: "qwen3guard-gen-0.6b",
            constructor_name: qwen3guard_gen_0_6b,
            display_name: "Qwen3Guard-Gen-0.6B",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        Qwen3guardGen8b {
            model_name: "qwen3guard-gen-8b",
            constructor_name: qwen3guard_gen_8b,
            display_name: "Qwen3Guard-Gen-8B",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
    }
}
