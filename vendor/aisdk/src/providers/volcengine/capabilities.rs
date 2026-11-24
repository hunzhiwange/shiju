//! Capabilities for volcengine models.
//!
//! This module defines model types and their capabilities for volcengine providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::volcengine::Volcengine;

model_capabilities! {
    provider: Volcengine,
    models: {
        DeepseekV4FlashGa260731 {
            model_name: "deepseek-v4-flash-ga-260731",
            constructor_name: deepseek_v4_flash_ga_260731,
            display_name: "DeepSeek V4 Flash 0731",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV4ProGa260813 {
            model_name: "deepseek-v4-pro-ga-260813",
            constructor_name: deepseek_v4_pro_ga_260813,
            display_name: "DeepSeek V4 Pro 0813",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DoubaoSeed20Lite260428 {
            model_name: "doubao-seed-2-0-lite-260428",
            constructor_name: doubao_seed_2_0_lite_260428,
            display_name: "Seed 2.0 Lite",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DoubaoSeed20Mini260428 {
            model_name: "doubao-seed-2-0-mini-260428",
            constructor_name: doubao_seed_2_0_mini_260428,
            display_name: "Seed 2.0 Mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DoubaoSeed21Pro260628 {
            model_name: "doubao-seed-2-1-pro-260628",
            constructor_name: doubao_seed_2_1_pro_260628,
            display_name: "Seed 2.1 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DoubaoSeed21Turbo260628 {
            model_name: "doubao-seed-2-1-turbo-260628",
            constructor_name: doubao_seed_2_1_turbo_260628,
            display_name: "Seed 2.1 Turbo",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DoubaoSeedCharacter260628 {
            model_name: "doubao-seed-character-260628",
            constructor_name: doubao_seed_character_260628,
            display_name: "Seed Character",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DoubaoSeedEvolving {
            model_name: "doubao-seed-evolving",
            constructor_name: doubao_seed_evolving,
            display_name: "Seed Evolving",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Glm52260617 {
            model_name: "glm-5-2-260617",
            constructor_name: glm_5_2_260617,
            display_name: "GLM-5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53Flash260828 {
            model_name: "glm-5-3-flash-260828",
            constructor_name: glm_5_3_flash_260828,
            display_name: "GLM-5.3-Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
