//! Capabilities for mistral models.
//!
//! This module defines model types and their capabilities for mistral providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::mistral::Mistral;

model_capabilities! {
    provider: Mistral,
    models: {
        Codestral2508 {
            model_name: "codestral-2508",
            constructor_name: codestral_2508,
            display_name: "Codestral 25.08",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CodestralLatest {
            model_name: "codestral-latest",
            constructor_name: codestral_latest,
            display_name: "Codestral (latest)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Ministral14b2512 {
            model_name: "ministral-14b-2512",
            constructor_name: ministral_14b_2512,
            display_name: "Ministral 3 14B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Ministral3b2512 {
            model_name: "ministral-3b-2512",
            constructor_name: ministral_3b_2512,
            display_name: "Ministral 3 3B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Ministral3bLatest {
            model_name: "ministral-3b-latest",
            constructor_name: ministral_3b_latest,
            display_name: "Ministral 3B (latest)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Ministral8b2512 {
            model_name: "ministral-8b-2512",
            constructor_name: ministral_8b_2512,
            display_name: "Ministral 3 8B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Ministral8bLatest {
            model_name: "ministral-8b-latest",
            constructor_name: ministral_8b_latest,
            display_name: "Ministral 8B (latest)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralEmbed {
            model_name: "mistral-embed",
            constructor_name: mistral_embed,
            display_name: "Mistral Embed",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        MistralLarge2512 {
            model_name: "mistral-large-2512",
            constructor_name: mistral_large_2512,
            display_name: "Mistral Large 3",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralLarge4 {
            model_name: "mistral-large-4",
            constructor_name: mistral_large_4,
            display_name: "Mistral Large 4",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralLargeLatest {
            model_name: "mistral-large-latest",
            constructor_name: mistral_large_latest,
            display_name: "Mistral Large (latest)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMedium2604 {
            model_name: "mistral-medium-2604",
            constructor_name: mistral_medium_2604,
            display_name: "Mistral Medium 3.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMediumLatest {
            model_name: "mistral-medium-latest",
            constructor_name: mistral_medium_latest,
            display_name: "Mistral Medium (latest)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralSmall2603 {
            model_name: "mistral-small-2603",
            constructor_name: mistral_small_2603,
            display_name: "Mistral Small 4",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralSmallLatest {
            model_name: "mistral-small-latest",
            constructor_name: mistral_small_latest,
            display_name: "Mistral Small (latest)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        VoxtralMiniLatest {
            model_name: "voxtral-mini-latest",
            constructor_name: voxtral_mini_latest,
            display_name: "Voxtral Mini (latest)",
            capabilities: [AudioInputSupport, ImageInputSupport, TextOutputSupport]
        },
        VoxtralMiniTtsLatest {
            model_name: "voxtral-mini-tts-latest",
            constructor_name: voxtral_mini_tts_latest,
            display_name: "Voxtral Mini TTS (latest)",
            capabilities: [AudioOutputSupport, TextInputSupport]
        },
        VoxtralSmall2507 {
            model_name: "voxtral-small-2507",
            constructor_name: voxtral_small_2507,
            display_name: "Voxtral Small",
            capabilities: [AudioInputSupport, ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        VoxtralSmallLatest {
            model_name: "voxtral-small-latest",
            constructor_name: voxtral_small_latest,
            display_name: "Voxtral Small (latest)",
            capabilities: [AudioInputSupport, ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm53 {
            model_name: "zai-glm-5-3",
            constructor_name: zai_glm_5_3,
            display_name: "GLM-5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm53Flash {
            model_name: "zai-glm-5-3-flash",
            constructor_name: zai_glm_5_3_flash,
            display_name: "GLM-5.3-Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
