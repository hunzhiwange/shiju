//! Capabilities for groq models.
//!
//! This module defines model types and their capabilities for groq providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::groq::Groq;

model_capabilities! {
    provider: Groq,
    models: {
        Allam27b {
            model_name: "allam-2-7b",
            constructor_name: allam_2_7b,
            display_name: "ALLaM-2-7b",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        CanopylabsOrpheusArabicSaudi {
            model_name: "canopylabs/orpheus-arabic-saudi",
            constructor_name: canopylabs_orpheus_arabic_saudi,
            display_name: "Canopy Labs Orpheus Arabic Saudi",
            capabilities: [AudioOutputSupport, TextInputSupport]
        },
        CanopylabsOrpheusV1English {
            model_name: "canopylabs/orpheus-v1-english",
            constructor_name: canopylabs_orpheus_v1_english,
            display_name: "Canopy Labs Orpheus V1 English",
            capabilities: [AudioOutputSupport, TextInputSupport]
        },
        MetaLlamaLlamaPromptGuard222m {
            model_name: "meta-llama/llama-prompt-guard-2-22m",
            constructor_name: meta_llama_llama_prompt_guard_2_22m,
            display_name: "Llama Prompt Guard 2 22M",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        MetaLlamaLlamaPromptGuard286m {
            model_name: "meta-llama/llama-prompt-guard-2-86m",
            constructor_name: meta_llama_llama_prompt_guard_2_86m,
            display_name: "Prompt Guard 2 86M",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        OpenaiGptOss120b {
            model_name: "openai/gpt-oss-120b",
            constructor_name: openai_gpt_oss_120b,
            display_name: "GPT OSS 120B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOss20b {
            model_name: "openai/gpt-oss-20b",
            constructor_name: openai_gpt_oss_20b,
            display_name: "GPT OSS 20B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOssSafeguard20b {
            model_name: "openai/gpt-oss-safeguard-20b",
            constructor_name: openai_gpt_oss_safeguard_20b,
            display_name: "Safety GPT OSS 20B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3827b {
            model_name: "qwen/qwen3.8-27b",
            constructor_name: qwen_qwen3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        WhisperLargeV3 {
            model_name: "whisper-large-v3",
            constructor_name: whisper_large_v3,
            display_name: "Whisper",
            capabilities: [AudioInputSupport, ImageInputSupport, TextOutputSupport]
        },
        WhisperLargeV3Turbo {
            model_name: "whisper-large-v3-turbo",
            constructor_name: whisper_large_v3_turbo,
            display_name: "Whisper Large V3 Turbo",
            capabilities: [AudioInputSupport, ImageInputSupport, TextOutputSupport]
        },
    }
}
