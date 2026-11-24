//! Capabilities for crusoe models.
//!
//! This module defines model types and their capabilities for crusoe providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::crusoe::Crusoe;

model_capabilities! {
    provider: Crusoe,
    models: {
        GoogleGemma431bIt {
            model_name: "google/gemma-4-31b-it",
            constructor_name: google_gemma_4_31b_it,
            display_name: "Gemma 4 31B IT",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NvidiaNvidiaNemotron3Nano30bA3b {
            model_name: "nvidia/NVIDIA-Nemotron-3-Nano-30B-A3B",
            constructor_name: nvidia_nvidia_nemotron_3_nano_30b_a3b,
            display_name: "Nemotron 3 Nano 30B A3B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NvidiaNvidiaNemotron3Super120bA12b {
            model_name: "nvidia/NVIDIA-Nemotron-3-Super-120B-A12B",
            constructor_name: nvidia_nvidia_nemotron_3_super_120b_a12b,
            display_name: "Nemotron 3 Super 120B A12B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NvidiaNemotron3NanoOmniReasoning30bA3b {
            model_name: "nvidia/Nemotron-3-Nano-Omni-Reasoning-30B-A3B",
            constructor_name: nvidia_nemotron_3_nano_omni_reasoning_30b_a3b,
            display_name: "Nemotron 3 Nano Omni 30B A3B Reasoning",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        OpenaiGptOss120b {
            model_name: "openai/gpt-oss-120b",
            constructor_name: openai_gpt_oss_120b,
            display_name: "GPT OSS 120B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
