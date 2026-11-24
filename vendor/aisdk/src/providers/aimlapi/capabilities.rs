//! Capabilities for aimlapi models.
//!
//! This module defines model types and their capabilities for aimlapi providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::aimlapi::Aimlapi;

model_capabilities! {
    provider: Aimlapi,
    models: {
        AionLabsAionRpLlama318b {
            model_name: "aion-labs/aion-rp-llama-3.1-8b",
            constructor_name: aion_labs_aion_rp_llama_3_1_8b,
            display_name: "Aion-RP 1.0 (8B)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        AlibabaQwenMax {
            model_name: "alibaba/qwen-max",
            constructor_name: alibaba_qwen_max,
            display_name: "Qwen Max",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaQwenPlus {
            model_name: "alibaba/qwen-plus",
            constructor_name: alibaba_qwen_plus,
            display_name: "Qwen Plus",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaQwen3Coder480bA35bInstruct {
            model_name: "alibaba/qwen3-coder-480b-a35b-instruct",
            constructor_name: alibaba_qwen3_coder_480b_a35b_instruct,
            display_name: "Qwen3 Coder",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaQwen3Next80bA3bInstruct {
            model_name: "alibaba/qwen3-next-80b-a3b-instruct",
            constructor_name: alibaba_qwen3_next_80b_a3b_instruct,
            display_name: "Qwen3 Next 80B A3B Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaQwen3824tA95b {
            model_name: "alibaba/qwen3.8-2.4t-a95b",
            constructor_name: alibaba_qwen3_8_2_4t_a95b,
            display_name: "Qwen3.8 2.4T A95B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaQwen3827b {
            model_name: "alibaba/qwen3.8-27b",
            constructor_name: alibaba_qwen3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaQwen38Flash {
            model_name: "alibaba/qwen3.8-flash",
            constructor_name: alibaba_qwen3_8_flash,
            display_name: "Qwen3.8 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaQwen38Max {
            model_name: "alibaba/qwen3.8-max",
            constructor_name: alibaba_qwen3_8_max,
            display_name: "Qwen3.8 Max",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaQwen38MaxPrime {
            model_name: "alibaba/qwen3.8-max-prime",
            constructor_name: alibaba_qwen3_8_max_prime,
            display_name: "Qwen3.8 Max Prime",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AmazonNovaLiteV1 {
            model_name: "amazon/nova-lite-v1",
            constructor_name: amazon_nova_lite_v1,
            display_name: "Nova Lite 1.0",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonNovaMicroV1 {
            model_name: "amazon/nova-micro-v1",
            constructor_name: amazon_nova_micro_v1,
            display_name: "Nova Micro 1.0",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonNovaProV1 {
            model_name: "amazon/nova-pro-v1",
            constructor_name: amazon_nova_pro_v1,
            display_name: "Nova Pro 1.0",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthraciteOrgMagnumV472b {
            model_name: "anthracite-org/magnum-v4-72b",
            constructor_name: anthracite_org_magnum_v4_72b,
            display_name: "Magnum v4 72B",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        AnthropicClaudeFable5 {
            model_name: "anthropic/claude-fable-5",
            constructor_name: anthropic_claude_fable_5,
            display_name: "Claude Fable 5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeFable51 {
            model_name: "anthropic/claude-fable-5.1",
            constructor_name: anthropic_claude_fable_5_1,
            display_name: "Claude Fable 5.1",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeHaiku45 {
            model_name: "anthropic/claude-haiku-4.5",
            constructor_name: anthropic_claude_haiku_4_5,
            display_name: "Claude 4.5 Haiku",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeHaiku55 {
            model_name: "anthropic/claude-haiku-5.5",
            constructor_name: anthropic_claude_haiku_5_5,
            display_name: "Claude Haiku 5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus45 {
            model_name: "anthropic/claude-opus-4.5",
            constructor_name: anthropic_claude_opus_4_5,
            display_name: "Claude 4.5 Opus",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus47 {
            model_name: "anthropic/claude-opus-4.7",
            constructor_name: anthropic_claude_opus_4_7,
            display_name: "Claude 4.7 Opus",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus48 {
            model_name: "anthropic/claude-opus-4.8",
            constructor_name: anthropic_claude_opus_4_8,
            display_name: "Claude 4.8 Opus",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus5 {
            model_name: "anthropic/claude-opus-5",
            constructor_name: anthropic_claude_opus_5,
            display_name: "Claude Opus 5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus55 {
            model_name: "anthropic/claude-opus-5.5",
            constructor_name: anthropic_claude_opus_5_5,
            display_name: "Claude Opus 5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet45 {
            model_name: "anthropic/claude-sonnet-4.5",
            constructor_name: anthropic_claude_sonnet_4_5,
            display_name: "Claude 4.5 Sonnet",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet46 {
            model_name: "anthropic/claude-sonnet-4.6",
            constructor_name: anthropic_claude_sonnet_4_6,
            display_name: "Claude 4.6 Sonnet",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet5 {
            model_name: "anthropic/claude-sonnet-5",
            constructor_name: anthropic_claude_sonnet_5,
            display_name: "Claude Sonnet 5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet55 {
            model_name: "anthropic/claude-sonnet-5.5",
            constructor_name: anthropic_claude_sonnet_5_5,
            display_name: "Claude Sonnet 5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceSeedSeed16Flash {
            model_name: "bytedance-seed/seed-1.6-flash",
            constructor_name: bytedance_seed_seed_1_6_flash,
            display_name: "Seed 1.6 Flash",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BytedanceSeedSeed20Code {
            model_name: "bytedance-seed/seed-2.0-code",
            constructor_name: bytedance_seed_seed_2_0_code,
            display_name: "Seed-2.0-Code",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BytedanceSeedSeed20Lite {
            model_name: "bytedance-seed/seed-2.0-lite",
            constructor_name: bytedance_seed_seed_2_0_lite,
            display_name: "Seed-2.0-Lite",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BytedanceSeedSeed20Mini {
            model_name: "bytedance-seed/seed-2.0-mini",
            constructor_name: bytedance_seed_seed_2_0_mini,
            display_name: "Seed-2.0-Mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        CognitivecomputationsDolphinMistral24bVeniceEdition {
            model_name: "cognitivecomputations/dolphin-mistral-24b-venice-edition",
            constructor_name: cognitivecomputations_dolphin_mistral_24b_venice_edition,
            display_name: "Venice Uncensored",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        CohereCommandA {
            model_name: "cohere/command-a",
            constructor_name: cohere_command_a,
            display_name: "Command A",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CohereCommandR082024 {
            model_name: "cohere/command-r-08-2024",
            constructor_name: cohere_command_r_08_2024,
            display_name: "Command R (08-2024)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CohereCommandRPlus082024 {
            model_name: "cohere/command-r-plus-08-2024",
            constructor_name: cohere_command_r_plus_08_2024,
            display_name: "Command R+ (08-2024)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CohereCommandR7b122024 {
            model_name: "cohere/command-r7b-12-2024",
            constructor_name: cohere_command_r7b_12_2024,
            display_name: "Command R7B (12-2024)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekChat {
            model_name: "deepseek/deepseek-chat",
            constructor_name: deepseek_deepseek_chat,
            display_name: "DeepSeek V4 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekChatV30324 {
            model_name: "deepseek/deepseek-chat-v3-0324",
            constructor_name: deepseek_deepseek_chat_v3_0324,
            display_name: "DeepSeek V3",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV4Flash {
            model_name: "deepseek/deepseek-v4-flash",
            constructor_name: deepseek_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV4FlashVisionExp {
            model_name: "deepseek/deepseek-v4-flash-vision-exp",
            constructor_name: deepseek_deepseek_v4_flash_vision_exp,
            display_name: "DeepSeek V4 Flash Vision Exp",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV4Pro {
            model_name: "deepseek/deepseek-v4-pro",
            constructor_name: deepseek_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV4Pro0813 {
            model_name: "deepseek/deepseek-v4-pro-0813",
            constructor_name: deepseek_deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV41Flash {
            model_name: "deepseek/deepseek-v4.1-flash",
            constructor_name: deepseek_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemini38Flash {
            model_name: "google/gemini-3-8-flash",
            constructor_name: google_gemini_3_8_flash,
            display_name: "Gemini 3.8 Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini31FlashLitePreview {
            model_name: "google/gemini-3.1-flash-lite-preview",
            constructor_name: google_gemini_3_1_flash_lite_preview,
            display_name: "Gemini 3.1 Flash Lite Preview",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini31ProPreview {
            model_name: "google/gemini-3.1-pro-preview",
            constructor_name: google_gemini_3_1_pro_preview,
            display_name: "Gemini 3.1 Pro Preview",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini31ProPreviewCustomtools {
            model_name: "google/gemini-3.1-pro-preview-customtools",
            constructor_name: google_gemini_3_1_pro_preview_customtools,
            display_name: "Gemini 3.1 Pro Preview Custom Tools",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini36Flash {
            model_name: "google/gemini-3.6-flash",
            constructor_name: google_gemini_3_6_flash,
            display_name: "Gemini 3.6 Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini37Flash {
            model_name: "google/gemini-3.7-flash",
            constructor_name: google_gemini_3_7_flash,
            display_name: "Gemini 3.7 Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGeminiFlashLatest {
            model_name: "google/gemini-flash-latest",
            constructor_name: google_gemini_flash_latest,
            display_name: "Gemini Flash Latest",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemma227bIt {
            model_name: "google/gemma-2-27b-it",
            constructor_name: google_gemma_2_27b_it,
            display_name: "Gemma 2 27B",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        GoogleGemma312bIt {
            model_name: "google/gemma-3-12b-it",
            constructor_name: google_gemma_3_12b_it,
            display_name: "Gemma 3 12B",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemma327bIt {
            model_name: "google/gemma-3-27b-it",
            constructor_name: google_gemma_3_27b_it,
            display_name: "Gemma 3 27B",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemma34bIt {
            model_name: "google/gemma-3-4b-it",
            constructor_name: google_gemma_3_4b_it,
            display_name: "Gemma 3 4B",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemma426bA4bIt {
            model_name: "google/gemma-4-26b-a4b-it",
            constructor_name: google_gemma_4_26b_a4b_it,
            display_name: "Gemma 4 26B A4B IT",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemma431bIt {
            model_name: "google/gemma-4-31b-it",
            constructor_name: google_gemma_4_31b_it,
            display_name: "Gemma 4 31B Instruct",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GrypheMythomaxL213b {
            model_name: "gryphe/mythomax-l2-13b",
            constructor_name: gryphe_mythomax_l2_13b,
            display_name: "MythoMax 13B",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        IbmGraniteGranite40HMicro {
            model_name: "ibm-granite/granite-4.0-h-micro",
            constructor_name: ibm_granite_granite_4_0_h_micro,
            display_name: "Granite 4.0 Micro",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        InceptionMercury2 {
            model_name: "inception/mercury-2",
            constructor_name: inception_mercury_2,
            display_name: "Mercury 2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        InceptionMercury25 {
            model_name: "inception/mercury-2.5",
            constructor_name: inception_mercury_2_5,
            display_name: "Mercury 2.5",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        InferenceNetSchematronV2Small {
            model_name: "inference-net/schematron-v2-small",
            constructor_name: inference_net_schematron_v2_small,
            display_name: "Schematron V2 Small",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        InferenceNetSchematronV2Turbo {
            model_name: "inference-net/schematron-v2-turbo",
            constructor_name: inference_net_schematron_v2_turbo,
            display_name: "Schematron V2 Turbo",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        MancerWeaver {
            model_name: "mancer/weaver",
            constructor_name: mancer_weaver,
            display_name: "Weaver (alpha)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        MetaLlamaLlama3370bInstructTurbo {
            model_name: "meta-llama/Llama-3.3-70B-Instruct-Turbo",
            constructor_name: meta_llama_llama_3_3_70b_instruct_turbo,
            display_name: "Llama 3.3 70B Instruct Turbo",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaMuseGlimmer30b {
            model_name: "meta/muse-glimmer-30b",
            constructor_name: meta_muse_glimmer_30b,
            display_name: "Muse Glimmer 30B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaMuseSpark11 {
            model_name: "meta/muse-spark-1.1",
            constructor_name: meta_muse_spark_1_1,
            display_name: "Muse Spark 1.1",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MetaMuseSpark12 {
            model_name: "meta/muse-spark-1.2",
            constructor_name: meta_muse_spark_1_2,
            display_name: "Muse Spark 1.2",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MetaMuseSpark13 {
            model_name: "meta/muse-spark-1.3",
            constructor_name: meta_muse_spark_1_3,
            display_name: "Muse Spark 1.3",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MistralaiCodestral2508 {
            model_name: "mistralai/codestral-2508",
            constructor_name: mistralai_codestral_2508,
            display_name: "Codestral 2508",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiDevstral2512 {
            model_name: "mistralai/devstral-2512",
            constructor_name: mistralai_devstral_2512,
            display_name: "Devstral 2 2512",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMinistral14b2512 {
            model_name: "mistralai/ministral-14b-2512",
            constructor_name: mistralai_ministral_14b_2512,
            display_name: "Ministral 3 14B 2512",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMinistral3b2512 {
            model_name: "mistralai/ministral-3b-2512",
            constructor_name: mistralai_ministral_3b_2512,
            display_name: "Ministral 3 3B 2512",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMinistral8b2512 {
            model_name: "mistralai/ministral-8b-2512",
            constructor_name: mistralai_ministral_8b_2512,
            display_name: "Ministral 3 8B 2512",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMistralLarge {
            model_name: "mistralai/mistral-large",
            constructor_name: mistralai_mistral_large,
            display_name: "Large",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMistralLarge2407 {
            model_name: "mistralai/mistral-large-2407",
            constructor_name: mistralai_mistral_large_2407,
            display_name: "Large 2407",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMistralMedium3 {
            model_name: "mistralai/mistral-medium-3",
            constructor_name: mistralai_mistral_medium_3,
            display_name: "Medium 3",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMistralMedium35 {
            model_name: "mistralai/mistral-medium-3-5",
            constructor_name: mistralai_mistral_medium_3_5,
            display_name: "Medium 3.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMistralMedium31 {
            model_name: "mistralai/mistral-medium-3.1",
            constructor_name: mistralai_mistral_medium_3_1,
            display_name: "Medium 3.1",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMistralNemo {
            model_name: "mistralai/mistral-nemo",
            constructor_name: mistralai_mistral_nemo,
            display_name: "Mistral NeMo",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMistralSaba {
            model_name: "mistralai/mistral-saba",
            constructor_name: mistralai_mistral_saba,
            display_name: "Saba",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMistralSmall24bInstruct2501 {
            model_name: "mistralai/mistral-small-24b-instruct-2501",
            constructor_name: mistralai_mistral_small_24b_instruct_2501,
            display_name: "Small 3",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        MistralaiMistralSmall2603 {
            model_name: "mistralai/mistral-small-2603",
            constructor_name: mistralai_mistral_small_2603,
            display_name: "Small 4",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMistralSmall3124bInstruct {
            model_name: "mistralai/mistral-small-3.1-24b-instruct",
            constructor_name: mistralai_mistral_small_3_1_24b_instruct,
            display_name: "Small 3.1 24B",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        MistralaiMistralSmall3224bInstruct {
            model_name: "mistralai/mistral-small-3.2-24b-instruct",
            constructor_name: mistralai_mistral_small_3_2_24b_instruct,
            display_name: "Small 3.2 24B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiMixtral8x22bInstruct {
            model_name: "mistralai/mixtral-8x22b-instruct",
            constructor_name: mistralai_mixtral_8x22b_instruct,
            display_name: "Mixtral 8x22B Instruct",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralaiVoxtralSmall24b2507 {
            model_name: "mistralai/voxtral-small-24b-2507",
            constructor_name: mistralai_voxtral_small_24b_2507,
            display_name: "Voxtral Small 24B 2507",
            capabilities: [AudioInputSupport, ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotKimiK3 {
            model_name: "moonshot/kimi-k3",
            constructor_name: moonshot_kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MoonshotaiKimiK2 {
            model_name: "moonshotai/kimi-k2",
            constructor_name: moonshotai_kimi_k2,
            display_name: "Kimi K2 0711",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK20905 {
            model_name: "moonshotai/kimi-k2-0905",
            constructor_name: moonshotai_kimi_k2_0905,
            display_name: "Kimi K2 0905",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK2Thinking {
            model_name: "moonshotai/kimi-k2-thinking",
            constructor_name: moonshotai_kimi_k2_thinking,
            display_name: "Kimi K2 Thinking",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MorphMorphV3Fast {
            model_name: "morph/morph-v3-fast",
            constructor_name: morph_morph_v3_fast,
            display_name: "V3 Fast",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        MorphMorphV3Large {
            model_name: "morph/morph-v3-large",
            constructor_name: morph_morph_v3_large,
            display_name: "V3 Large",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        NousresearchHermes3Llama31405b {
            model_name: "nousresearch/hermes-3-llama-3.1-405b",
            constructor_name: nousresearch_hermes_3_llama_3_1_405b,
            display_name: "Hermes 3 405B Instruct",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        NousresearchHermes3Llama3170b {
            model_name: "nousresearch/hermes-3-llama-3.1-70b",
            constructor_name: nousresearch_hermes_3_llama_3_1_70b,
            display_name: "Hermes 3 70B Instruct",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        OpenaiGpt35Turbo {
            model_name: "openai/gpt-3.5-turbo",
            constructor_name: openai_gpt_3_5_turbo,
            display_name: "GPT-3.5 Turbo",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        OpenaiGpt4 {
            model_name: "openai/gpt-4",
            constructor_name: openai_gpt_4,
            display_name: "GPT-4",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4Turbo {
            model_name: "openai/gpt-4-turbo",
            constructor_name: openai_gpt_4_turbo,
            display_name: "GPT-4 Turbo",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt41 {
            model_name: "openai/gpt-4.1",
            constructor_name: openai_gpt_4_1,
            display_name: "GPT-4.1",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt41Mini {
            model_name: "openai/gpt-4.1-mini",
            constructor_name: openai_gpt_4_1_mini,
            display_name: "GPT-4.1 Mini",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt41Nano {
            model_name: "openai/gpt-4.1-nano",
            constructor_name: openai_gpt_4_1_nano,
            display_name: "GPT-4.1 Nano",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4o {
            model_name: "openai/gpt-4o",
            constructor_name: openai_gpt_4o,
            display_name: "GPT-4o",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4o20240513 {
            model_name: "openai/gpt-4o-2024-05-13",
            constructor_name: openai_gpt_4o_2024_05_13,
            display_name: "GPT-4o",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4o20240806 {
            model_name: "openai/gpt-4o-2024-08-06",
            constructor_name: openai_gpt_4o_2024_08_06,
            display_name: "GPT-4o",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4o20241120 {
            model_name: "openai/gpt-4o-2024-11-20",
            constructor_name: openai_gpt_4o_2024_11_20,
            display_name: "GPT-4o",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4oMini {
            model_name: "openai/gpt-4o-mini",
            constructor_name: openai_gpt_4o_mini,
            display_name: "GPT-4o Mini",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5 {
            model_name: "openai/gpt-5",
            constructor_name: openai_gpt_5,
            display_name: "GPT-5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt51 {
            model_name: "openai/gpt-5-1",
            constructor_name: openai_gpt_5_1,
            display_name: "GPT-5.1",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt52 {
            model_name: "openai/gpt-5-2",
            constructor_name: openai_gpt_5_2,
            display_name: "GPT-5.2",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54 {
            model_name: "openai/gpt-5-4",
            constructor_name: openai_gpt_5_4,
            display_name: "GPT-5.4",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt55 {
            model_name: "openai/gpt-5-5",
            constructor_name: openai_gpt_5_5,
            display_name: "GPT-5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5Mini {
            model_name: "openai/gpt-5-mini",
            constructor_name: openai_gpt_5_mini,
            display_name: "GPT-5 Mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5Nano {
            model_name: "openai/gpt-5-nano",
            constructor_name: openai_gpt_5_nano,
            display_name: "GPT-5 Nano",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5Pro {
            model_name: "openai/gpt-5-pro",
            constructor_name: openai_gpt_5_pro,
            display_name: "GPT-5 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt51CodexMax {
            model_name: "openai/gpt-5.1-codex-max",
            constructor_name: openai_gpt_5_1_codex_max,
            display_name: "GPT-5.1-Codex-Max",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt52Pro {
            model_name: "openai/gpt-5.2-pro",
            constructor_name: openai_gpt_5_2_pro,
            display_name: "GPT-5.2 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt53Codex {
            model_name: "openai/gpt-5.3-codex",
            constructor_name: openai_gpt_5_3_codex,
            display_name: "GPT-5.3 Codex",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Image2 {
            model_name: "openai/gpt-5.4-image-2",
            constructor_name: openai_gpt_5_4_image_2,
            display_name: "GPT-5.4 Image 2",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        OpenaiGpt54Mini {
            model_name: "openai/gpt-5.4-mini",
            constructor_name: openai_gpt_5_4_mini,
            display_name: "GPT-5.4 Mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Nano {
            model_name: "openai/gpt-5.4-nano",
            constructor_name: openai_gpt_5_4_nano,
            display_name: "GPT-5.4 Nano",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Pro {
            model_name: "openai/gpt-5.4-pro",
            constructor_name: openai_gpt_5_4_pro,
            display_name: "GPT-5.4 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt55Pro {
            model_name: "openai/gpt-5.5-pro",
            constructor_name: openai_gpt_5_5_pro,
            display_name: "GPT-5.5 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Luna {
            model_name: "openai/gpt-5.6-luna",
            constructor_name: openai_gpt_5_6_luna,
            display_name: "GPT-5.6 Luna",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56LunaPro {
            model_name: "openai/gpt-5.6-luna-pro",
            constructor_name: openai_gpt_5_6_luna_pro,
            display_name: "GPT-5.6 Luna Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Sol {
            model_name: "openai/gpt-5.6-sol",
            constructor_name: openai_gpt_5_6_sol,
            display_name: "GPT-5.6 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56SolPro {
            model_name: "openai/gpt-5.6-sol-pro",
            constructor_name: openai_gpt_5_6_sol_pro,
            display_name: "GPT-5.6 Sol Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Terra {
            model_name: "openai/gpt-5.6-terra",
            constructor_name: openai_gpt_5_6_terra,
            display_name: "GPT-5.6 Terra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56TerraPro {
            model_name: "openai/gpt-5.6-terra-pro",
            constructor_name: openai_gpt_5_6_terra_pro,
            display_name: "GPT-5.6 Terra Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Astra {
            model_name: "openai/gpt-6-astra",
            constructor_name: openai_gpt_6_astra,
            display_name: "GPT-6 Astra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Luna {
            model_name: "openai/gpt-6-luna",
            constructor_name: openai_gpt_6_luna,
            display_name: "GPT-6 Luna",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Sol {
            model_name: "openai/gpt-6-sol",
            constructor_name: openai_gpt_6_sol,
            display_name: "GPT-6 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt61Sol {
            model_name: "openai/gpt-6.1-sol",
            constructor_name: openai_gpt_6_1_sol,
            display_name: "GPT-6.1 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
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
            display_name: "gpt-oss-safeguard-20b",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiO1 {
            model_name: "openai/o1",
            constructor_name: openai_o1,
            display_name: "o1",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiO1Pro {
            model_name: "openai/o1-pro",
            constructor_name: openai_o1_pro,
            display_name: "o1-pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiO3Mini {
            model_name: "openai/o3-mini",
            constructor_name: openai_o3_mini,
            display_name: "o3 Mini",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiO3Pro {
            model_name: "openai/o3-pro",
            constructor_name: openai_o3_pro,
            display_name: "o3 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        PerplexitySonar {
            model_name: "perplexity/sonar",
            constructor_name: perplexity_sonar,
            display_name: "Sonar",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        PerplexitySonarPro {
            model_name: "perplexity/sonar-pro",
            constructor_name: perplexity_sonar_pro,
            display_name: "Sonar Pro",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        RekaaiRekaEdge {
            model_name: "rekaai/reka-edge",
            constructor_name: rekaai_reka_edge,
            display_name: "Reka Edge",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        RelaceRelaceApply3 {
            model_name: "relace/relace-apply-3",
            constructor_name: relace_relace_apply_3,
            display_name: "Apply 3",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        RelaceRelaceSearch {
            model_name: "relace/relace-search",
            constructor_name: relace_relace_search,
            display_name: "Search",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        SakanaFuguMax {
            model_name: "sakana/fugu-max",
            constructor_name: sakana_fugu_max,
            display_name: "Fugu Max",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        SakanaFuguUltra {
            model_name: "sakana/fugu-ultra",
            constructor_name: sakana_fugu_ultra,
            display_name: "Fugu Ultra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        SakanaFuguUltraV2 {
            model_name: "sakana/fugu-ultra-v2",
            constructor_name: sakana_fugu_ultra_v2,
            display_name: "Fugu Ultra v2",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Sao10kL3Lunaris8b {
            model_name: "sao10k/l3-lunaris-8b",
            constructor_name: sao10k_l3_lunaris_8b,
            display_name: "Llama 3 8B Lunaris",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        Sao10kL31Euryale70b {
            model_name: "sao10k/l3.1-euryale-70b",
            constructor_name: sao10k_l3_1_euryale_70b,
            display_name: "Llama 3.1 Euryale 70B v2.2",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Sao10kL33Euryale70b {
            model_name: "sao10k/l3.3-euryale-70b",
            constructor_name: sao10k_l3_3_euryale_70b,
            display_name: "Llama 3.3 Euryale 70B",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        StepfunStep5Preview {
            model_name: "stepfun/step-5-preview",
            constructor_name: stepfun_step_5_preview,
            display_name: "Step 5 Preview",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        TencentHyMt218b {
            model_name: "tencent/hy-mt2-1.8b",
            constructor_name: tencent_hy_mt2_1_8b,
            display_name: "Hy-MT2-1.8B",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        TencentHyMt230bA3b {
            model_name: "tencent/hy-mt2-30b-a3b",
            constructor_name: tencent_hy_mt2_30b_a3b,
            display_name: "Hy-MT2-30B-A3B",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        TencentHyMt27b {
            model_name: "tencent/hy-mt2-7b",
            constructor_name: tencent_hy_mt2_7b,
            display_name: "Hy-MT2-7B",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        TencentHy3 {
            model_name: "tencent/hy3",
            constructor_name: tencent_hy3,
            display_name: "Hy3",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentHy4Preview {
            model_name: "tencent/hy4-preview",
            constructor_name: tencent_hy4_preview,
            display_name: "Hy4 preview",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ThedrummerCydonia24bV41 {
            model_name: "thedrummer/cydonia-24b-v4.1",
            constructor_name: thedrummer_cydonia_24b_v4_1,
            display_name: "Cydonia 24B V4.1",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        ThedrummerSkyfall36bV2 {
            model_name: "thedrummer/skyfall-36b-v2",
            constructor_name: thedrummer_skyfall_36b_v2,
            display_name: "Skyfall 36B V2",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        ThedrummerUnslopnemo12b {
            model_name: "thedrummer/unslopnemo-12b",
            constructor_name: thedrummer_unslopnemo_12b,
            display_name: "UnslopNemo 12B",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        ThinkingmachinesInkling {
            model_name: "thinkingmachines/inkling",
            constructor_name: thinkingmachines_inkling,
            display_name: "Inkling",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ThinkingmachinesInklingSmall {
            model_name: "thinkingmachines/inkling-small",
            constructor_name: thinkingmachines_inkling_small,
            display_name: "Inkling Small",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UnbiasedPareto {
            model_name: "unbiased/pareto",
            constructor_name: unbiased_pareto,
            display_name: "Pareto",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Undi95RemmSlerpL213b {
            model_name: "undi95/remm-slerp-l2-13b",
            constructor_name: undi95_remm_slerp_l2_13b,
            display_name: "ReMM SLERP 13B",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        UpstageSolarPro3 {
            model_name: "upstage/solar-pro-3",
            constructor_name: upstage_solar_pro_3,
            display_name: "Solar Pro 3",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UpstageSolarPro4 {
            model_name: "upstage/solar-pro4",
            constructor_name: upstage_solar_pro4,
            display_name: "Solar Pro 4",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XAiGrok4200309NonReasoning {
            model_name: "x-ai/grok-4-20-0309-non-reasoning",
            constructor_name: x_ai_grok_4_20_0309_non_reasoning,
            display_name: "Grok 4.20 Non-Reasoning",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XAiGrok4FastNonReasoning {
            model_name: "x-ai/grok-4-fast-non-reasoning",
            constructor_name: x_ai_grok_4_fast_non_reasoning,
            display_name: "Grok 4 Fast Non-Reasoning",
            capabilities: [AudioInputSupport, ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        XiaomiMimoV25 {
            model_name: "xiaomi/mimo-v2.5",
            constructor_name: xiaomi_mimo_v2_5,
            display_name: "MiMo V2.5",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        XiaomiMimoV25Pro {
            model_name: "xiaomi/mimo-v2.5-pro",
            constructor_name: xiaomi_mimo_v2_5_pro,
            display_name: "MiMo V2.5 Pro",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZAiGlm45v {
            model_name: "z-ai/glm-4.5v",
            constructor_name: z_ai_glm_4_5v,
            display_name: "GLM 4.5V",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ZAiGlm46v {
            model_name: "z-ai/glm-4.6v",
            constructor_name: z_ai_glm_4_6v,
            display_name: "GLM 4.6V",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ZAiGlm47Flash {
            model_name: "z-ai/glm-4.7-flash",
            constructor_name: z_ai_glm_4_7_flash,
            display_name: "GLM 4.7 Flash",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZAiGlm5Turbo {
            model_name: "z-ai/glm-5-turbo",
            constructor_name: z_ai_glm_5_turbo,
            display_name: "GLM 5 Turbo",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZAiGlm53Flash {
            model_name: "z-ai/glm-5.3-flash",
            constructor_name: z_ai_glm_5_3_flash,
            display_name: "GLM 5.3 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ZAiGlm5vTurbo {
            model_name: "z-ai/glm-5v-turbo",
            constructor_name: z_ai_glm_5v_turbo,
            display_name: "GLM 5V Turbo",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ZhipuGlm52 {
            model_name: "zhipu/glm-5.2",
            constructor_name: zhipu_glm_5_2,
            display_name: "GLM 5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZhipuGlm53 {
            model_name: "zhipu/glm-5.3",
            constructor_name: zhipu_glm_5_3,
            display_name: "GLM 5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
