//! Capabilities for infomaniak models.
//!
//! This module defines model types and their capabilities for infomaniak providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::infomaniak::Infomaniak;

model_capabilities! {
    provider: Infomaniak,
    models: {
        QwenQwen35397bA17bFp8 {
            model_name: "Qwen/Qwen3.5-397B-A17B-FP8",
            constructor_name: qwen_qwen3_5_397b_a17b_fp8,
            display_name: "Qwen3.5 397B-A17B FP8",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BgeMultilingualGemma2 {
            model_name: "bge_multilingual_gemma2",
            constructor_name: bge_multilingual_gemma2,
            display_name: "BGE Multilingual Gemma2",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        GoogleGemma431bIt {
            model_name: "google/gemma-4-31B-it",
            constructor_name: google_gemma_4_31b_it,
            display_name: "Gemma 4 31B IT",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MiniLmL12V2 {
            model_name: "mini_lm_l12_v2",
            constructor_name: mini_lm_l12_v2,
            display_name: "All-MiniLM-L12-v2",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        MistralaiMinistral314bInstruct2512 {
            model_name: "mistralai/Ministral-3-14B-Instruct-2512",
            constructor_name: mistralai_ministral_3_14b_instruct_2512,
            display_name: "Ministral 3 14B Instruct",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMistralSmall4119b2603 {
            model_name: "mistralai/Mistral-Small-4-119B-2603",
            constructor_name: mistralai_mistral_small_4_119b_2603,
            display_name: "Mistral Small 4",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK26 {
            model_name: "moonshotai/Kimi-K2.6",
            constructor_name: moonshotai_kimi_k2_6,
            display_name: "Kimi K2.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        SwissAiApertusV1570b {
            model_name: "swiss-ai/Apertus-v1.5-70B",
            constructor_name: swiss_ai_apertus_v1_5_70b,
            display_name: "Apertus v1.5 70B",
            capabilities: [AudioInputSupport, ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
