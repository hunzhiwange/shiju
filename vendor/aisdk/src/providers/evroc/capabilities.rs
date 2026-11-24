//! Capabilities for evroc models.
//!
//! This module defines model types and their capabilities for evroc providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::evroc::Evroc;

model_capabilities! {
    provider: Evroc,
    models: {
        KblabKbWhisperLarge {
            model_name: "KBLab/kb-whisper-large",
            constructor_name: kblab_kb_whisper_large,
            display_name: "KB Whisper",
            capabilities: [AudioInputSupport, ImageInputSupport, TextOutputSupport]
        },
        QwenQwen3Embedding8b {
            model_name: "Qwen/Qwen3-Embedding-8B",
            constructor_name: qwen_qwen3_embedding_8b,
            display_name: "Qwen3 Embedding 8B",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        QwenQwen3Reranker4b {
            model_name: "Qwen/Qwen3-Reranker-4B",
            constructor_name: qwen_qwen3_reranker_4b,
            display_name: "Qwen3 Reranker 4B",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        QwenQwen3635bA3b {
            model_name: "Qwen/Qwen3.6-35B-A3B",
            constructor_name: qwen_qwen3_6_35b_a3b,
            display_name: "Qwen3.6 35B-A3B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen3827b {
            model_name: "Qwen/Qwen3.8-27B",
            constructor_name: qwen_qwen3_8_27b,
            display_name: "Qwen3.8-27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        EvrocRoc {
            model_name: "evroc/roc",
            constructor_name: evroc_roc,
            display_name: "roc",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemma426bA4bIt {
            model_name: "google/gemma-4-26B-A4B-it",
            constructor_name: google_gemma_4_26b_a4b_it,
            display_name: "Gemma 4 26B A4B IT",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        IntfloatMultilingualE5LargeInstruct {
            model_name: "intfloat/multilingual-e5-large-instruct",
            constructor_name: intfloat_multilingual_e5_large_instruct,
            display_name: "E5 Multi-Lingual Large Embeddings 0.6B",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        MistralaiMistralMedium35128b {
            model_name: "mistralai/Mistral-Medium-3.5-128B",
            constructor_name: mistralai_mistral_medium_3_5_128b,
            display_name: "Mistral Medium 3.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiVoxtralSmall24b2507 {
            model_name: "mistralai/Voxtral-Small-24B-2507",
            constructor_name: mistralai_voxtral_small_24b_2507,
            display_name: "Voxtral Small 24B",
            capabilities: [AudioInputSupport, ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        MoonshotaiKimiK26 {
            model_name: "moonshotai/Kimi-K2.6",
            constructor_name: moonshotai_kimi_k2_6,
            display_name: "Kimi K2.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NvidiaLlama3370bInstructFp8 {
            model_name: "nvidia/Llama-3.3-70B-Instruct-FP8",
            constructor_name: nvidia_llama_3_3_70b_instruct_fp8,
            display_name: "Llama-3.3-70B-Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOss120b {
            model_name: "openai/gpt-oss-120b",
            constructor_name: openai_gpt_oss_120b,
            display_name: "GPT OSS 120B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiWhisperLargeV3 {
            model_name: "openai/whisper-large-v3",
            constructor_name: openai_whisper_large_v3,
            display_name: "Whisper 3 Large",
            capabilities: [AudioInputSupport, ImageInputSupport, TextOutputSupport]
        },
        OpenaiWhisperLargeV3Turbo {
            model_name: "openai/whisper-large-v3-turbo",
            constructor_name: openai_whisper_large_v3_turbo,
            display_name: "Whisper Large v3 Turbo",
            capabilities: [AudioInputSupport, ImageInputSupport, TextOutputSupport]
        },
        ZaiOrgGlm52 {
            model_name: "zai-org/GLM-5.2",
            constructor_name: zai_org_glm_5_2,
            display_name: "GLM-5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
