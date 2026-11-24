//! Capabilities for lowrouter models.
//!
//! This module defines model types and their capabilities for lowrouter providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::lowrouter::Lowrouter;

model_capabilities! {
    provider: Lowrouter,
    models: {
        AutoMistralaiCodestral2508 {
            model_name: "auto/mistralai/codestral-2508",
            constructor_name: auto_mistralai_codestral_2508,
            display_name: "Codestral 25.08",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoMistralaiMinistral14b2512 {
            model_name: "auto/mistralai/ministral-14b-2512",
            constructor_name: auto_mistralai_ministral_14b_2512,
            display_name: "Ministral 3 14B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoMistralaiMinistral3b2512 {
            model_name: "auto/mistralai/ministral-3b-2512",
            constructor_name: auto_mistralai_ministral_3b_2512,
            display_name: "Ministral 3 3B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoMistralaiMinistral8b2512 {
            model_name: "auto/mistralai/ministral-8b-2512",
            constructor_name: auto_mistralai_ministral_8b_2512,
            display_name: "Ministral 3 8B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoMistralaiMistralLarge2512 {
            model_name: "auto/mistralai/mistral-large-2512",
            constructor_name: auto_mistralai_mistral_large_2512,
            display_name: "Mistral Large 3",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoMistralaiMistralLarge4 {
            model_name: "auto/mistralai/mistral-large-4",
            constructor_name: auto_mistralai_mistral_large_4,
            display_name: "Mistral Large 4",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoMistralaiMistralMedium35 {
            model_name: "auto/mistralai/mistral-medium-3.5",
            constructor_name: auto_mistralai_mistral_medium_3_5,
            display_name: "Mistral Medium 3.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoMistralaiMistralSmall2603 {
            model_name: "auto/mistralai/mistral-small-2603",
            constructor_name: auto_mistralai_mistral_small_2603,
            display_name: "Mistral Small 4",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt41 {
            model_name: "auto/openai/gpt-4.1",
            constructor_name: auto_openai_gpt_4_1,
            display_name: "GPT-4.1",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt41Mini {
            model_name: "auto/openai/gpt-4.1-mini",
            constructor_name: auto_openai_gpt_4_1_mini,
            display_name: "GPT-4.1 mini",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt4o {
            model_name: "auto/openai/gpt-4o",
            constructor_name: auto_openai_gpt_4o,
            display_name: "GPT-4o",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt4oMini {
            model_name: "auto/openai/gpt-4o-mini",
            constructor_name: auto_openai_gpt_4o_mini,
            display_name: "GPT-4o mini",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt5 {
            model_name: "auto/openai/gpt-5",
            constructor_name: auto_openai_gpt_5,
            display_name: "GPT-5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt5Mini {
            model_name: "auto/openai/gpt-5-mini",
            constructor_name: auto_openai_gpt_5_mini,
            display_name: "GPT-5 Mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt5Nano {
            model_name: "auto/openai/gpt-5-nano",
            constructor_name: auto_openai_gpt_5_nano,
            display_name: "GPT-5 Nano",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt52 {
            model_name: "auto/openai/gpt-5.2",
            constructor_name: auto_openai_gpt_5_2,
            display_name: "GPT-5.2",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt54 {
            model_name: "auto/openai/gpt-5.4",
            constructor_name: auto_openai_gpt_5_4,
            display_name: "GPT-5.4",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt54Mini {
            model_name: "auto/openai/gpt-5.4-mini",
            constructor_name: auto_openai_gpt_5_4_mini,
            display_name: "GPT-5.4 mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt55 {
            model_name: "auto/openai/gpt-5.5",
            constructor_name: auto_openai_gpt_5_5,
            display_name: "GPT-5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt56Luna {
            model_name: "auto/openai/gpt-5.6-luna",
            constructor_name: auto_openai_gpt_5_6_luna,
            display_name: "GPT-5.6 Luna",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt56Sol {
            model_name: "auto/openai/gpt-5.6-sol",
            constructor_name: auto_openai_gpt_5_6_sol,
            display_name: "GPT-5.6 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt56Terra {
            model_name: "auto/openai/gpt-5.6-terra",
            constructor_name: auto_openai_gpt_5_6_terra,
            display_name: "GPT-5.6 Terra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt6Astra {
            model_name: "auto/openai/gpt-6-astra",
            constructor_name: auto_openai_gpt_6_astra,
            display_name: "GPT-6 Astra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt6Luna {
            model_name: "auto/openai/gpt-6-luna",
            constructor_name: auto_openai_gpt_6_luna,
            display_name: "GPT-6 Luna",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt6Sol {
            model_name: "auto/openai/gpt-6-sol",
            constructor_name: auto_openai_gpt_6_sol,
            display_name: "GPT-6 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGpt61Sol {
            model_name: "auto/openai/gpt-6.1-sol",
            constructor_name: auto_openai_gpt_6_1_sol,
            display_name: "GPT-6.1 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGptOss120b {
            model_name: "auto/openai/gpt-oss-120b",
            constructor_name: auto_openai_gpt_oss_120b,
            display_name: "GPT OSS 120B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGptOss20b {
            model_name: "auto/openai/gpt-oss-20b",
            constructor_name: auto_openai_gpt_oss_20b,
            display_name: "GPT OSS 20B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGptOssSafeguard120b {
            model_name: "auto/openai/gpt-oss-safeguard-120b",
            constructor_name: auto_openai_gpt_oss_safeguard_120b,
            display_name: "GPT OSS Safeguard 120B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiGptOssSafeguard20b {
            model_name: "auto/openai/gpt-oss-safeguard-20b",
            constructor_name: auto_openai_gpt_oss_safeguard_20b,
            display_name: "GPT OSS Safeguard 20B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AutoOpenaiO3 {
            model_name: "auto/openai/o3",
            constructor_name: auto_openai_o3,
            display_name: "o3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
