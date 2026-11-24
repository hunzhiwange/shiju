//! Capabilities for regolo_ai models.
//!
//! This module defines model types and their capabilities for regolo_ai providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::regolo_ai::RegoloAi;

model_capabilities! {
    provider: RegoloAi,
    models: {
        Apertus70b {
            model_name: "apertus-70b",
            constructor_name: apertus_70b,
            display_name: "Apertus 70B",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BrickComplexityPro {
            model_name: "brick-complexity-pro",
            constructor_name: brick_complexity_pro,
            display_name: "Brick Complexity Pro",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BrickV1Beta {
            model_name: "brick-v1-beta",
            constructor_name: brick_v1_beta,
            display_name: "Brick v1 Beta",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekOcr2 {
            model_name: "deepseek-ocr-2",
            constructor_name: deepseek_ocr_2,
            display_name: "DeepSeek OCR 2",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        FasterWhisperLargeV3 {
            model_name: "faster-whisper-large-v3",
            constructor_name: faster_whisper_large_v3,
            display_name: "Faster Whisper Large v3",
            capabilities: [AudioInputSupport, ImageInputSupport, TextOutputSupport]
        },
        Gemma431b {
            model_name: "gemma4-31b",
            constructor_name: gemma4_31b,
            display_name: "Gemma 4 31B IT",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm52 {
            model_name: "glm5.2",
            constructor_name: glm5_2,
            display_name: "GLM-5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GptOss120b {
            model_name: "gpt-oss-120b",
            constructor_name: gpt_oss_120b,
            display_name: "GPT-OSS-120B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GptOss20b {
            model_name: "gpt-oss-20b",
            constructor_name: gpt_oss_20b,
            display_name: "GPT-OSS-20B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralSmall4119b {
            model_name: "mistral-small-4-119b",
            constructor_name: mistral_small_4_119b,
            display_name: "Mistral Small 4 119B",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenImage {
            model_name: "qwen-image",
            constructor_name: qwen_image,
            display_name: "Qwen-Image",
            capabilities: [ImageOutputSupport, TextInputSupport]
        },
        Qwen3Embedding8b {
            model_name: "qwen3-embedding-8b",
            constructor_name: qwen3_embedding_8b,
            display_name: "Qwen3-Embedding-8B",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        Qwen3Reranker4b {
            model_name: "qwen3-reranker-4b",
            constructor_name: qwen3_reranker_4b,
            display_name: "Qwen3-Reranker-4B",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        Qwen35122b {
            model_name: "qwen3.5-122b",
            constructor_name: qwen3_5_122b,
            display_name: "Qwen3.5-122B",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen359b {
            model_name: "qwen3.5-9b",
            constructor_name: qwen3_5_9b,
            display_name: "Qwen3.5-9B",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3827b {
            model_name: "qwen3.8-27b",
            constructor_name: qwen3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
