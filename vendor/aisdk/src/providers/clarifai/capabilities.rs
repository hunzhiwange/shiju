//! Capabilities for clarifai models.
//!
//! This module defines model types and their capabilities for clarifai providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::clarifai::Clarifai;

model_capabilities! {
    provider: Clarifai,
    models: {
        ArceeAiAfmModelsTrinityMini {
            model_name: "arcee_ai/AFM/models/trinity-mini",
            constructor_name: arcee_ai_afm_models_trinity_mini,
            display_name: "Trinity Mini",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ClarifaiMainModelsMmPoly8b {
            model_name: "clarifai/main/models/mm-poly-8b",
            constructor_name: clarifai_main_models_mm_poly_8b,
            display_name: "MM Poly 8B",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, VideoInputSupport]
        },
        DeepseekAiDeepseekOcrModelsDeepseekOcr {
            model_name: "deepseek-ai/deepseek-ocr/models/DeepSeek-OCR",
            constructor_name: deepseek_ai_deepseek_ocr_models_deepseek_ocr,
            display_name: "DeepSeek OCR",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        MinimaxaiChatCompletionModelsMinimaxM25HighThroughput {
            model_name: "minimaxai/chat-completion/models/MiniMax-M2_5-high-throughput",
            constructor_name: minimaxai_chat_completion_models_minimax_m2_5_high_throughput,
            display_name: "MiniMax-M2.5 High Throughput",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiCompletionModelsMinistral314bReasoning2512 {
            model_name: "mistralai/completion/models/Ministral-3-14B-Reasoning-2512",
            constructor_name: mistralai_completion_models_ministral_3_14b_reasoning_2512,
            display_name: "Ministral 3 14B Reasoning 2512",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiCompletionModelsMinistral33bReasoning2512 {
            model_name: "mistralai/completion/models/Ministral-3-3B-Reasoning-2512",
            constructor_name: mistralai_completion_models_ministral_3_3b_reasoning_2512,
            display_name: "Ministral 3 3B Reasoning 2512",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiChatCompletionModelsKimiK26 {
            model_name: "moonshotai/chat-completion/models/Kimi-K2_6",
            constructor_name: moonshotai_chat_completion_models_kimi_k2_6,
            display_name: "Kimi K2.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiChatCompletionModelsGptOss120bHighThroughput {
            model_name: "openai/chat-completion/models/gpt-oss-120b-high-throughput",
            constructor_name: openai_chat_completion_models_gpt_oss_120b_high_throughput,
            display_name: "GPT OSS 120B High Throughput",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiChatCompletionModelsGptOss20b {
            model_name: "openai/chat-completion/models/gpt-oss-20b",
            constructor_name: openai_chat_completion_models_gpt_oss_20b,
            display_name: "GPT OSS 20B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwencoderModelsQwen3Coder30bA3bInstruct {
            model_name: "qwen/qwenCoder/models/Qwen3-Coder-30B-A3B-Instruct",
            constructor_name: qwen_qwencoder_models_qwen3_coder_30b_a3b_instruct,
            display_name: "Qwen3 Coder 30B A3B Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwenlmModelsQwen330bA3bInstruct2507 {
            model_name: "qwen/qwenLM/models/Qwen3-30B-A3B-Instruct-2507",
            constructor_name: qwen_qwenlm_models_qwen3_30b_a3b_instruct_2507,
            display_name: "Qwen3 30B A3B Instruct 2507",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwenlmModelsQwen330bA3bThinking2507 {
            model_name: "qwen/qwenLM/models/Qwen3-30B-A3B-Thinking-2507",
            constructor_name: qwen_qwenlm_models_qwen3_30b_a3b_thinking_2507,
            display_name: "Qwen3 30B A3B Thinking 2507",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
