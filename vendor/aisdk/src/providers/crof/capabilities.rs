//! Capabilities for crof models.
//!
//! This module defines model types and their capabilities for crof providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::crof::Crof;

model_capabilities! {
    provider: Crof,
    models: {
        DeepseekV32 {
            model_name: "deepseek-v3.2",
            constructor_name: deepseek_v3_2,
            display_name: "DeepSeek V3.2",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV4Flash {
            model_name: "deepseek-v4-flash",
            constructor_name: deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV4Pro {
            model_name: "deepseek-v4-pro",
            constructor_name: deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV4ProLightning {
            model_name: "deepseek-v4-pro-lightning",
            constructor_name: deepseek_v4_pro_lightning,
            display_name: "DeepSeek V4 Pro Lightning",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Gemma431bIt {
            model_name: "gemma-4-31b-it",
            constructor_name: gemma_4_31b_it,
            display_name: "Gemma 4 31B IT",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm47 {
            model_name: "glm-4.7",
            constructor_name: glm_4_7,
            display_name: "GLM-4.7",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm47Flash {
            model_name: "glm-4.7-flash",
            constructor_name: glm_4_7_flash,
            display_name: "GLM-4.7-Flash",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm5 {
            model_name: "glm-5",
            constructor_name: glm_5,
            display_name: "GLM-5",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm51 {
            model_name: "glm-5.1",
            constructor_name: glm_5_1,
            display_name: "GLM-5.1",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm52 {
            model_name: "glm-5.2",
            constructor_name: glm_5_2,
            display_name: "GLM-5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Greg1Mini {
            model_name: "greg-1-mini",
            constructor_name: greg_1_mini,
            display_name: "Greg 1 Mini",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        Greg2Super {
            model_name: "greg-2-super",
            constructor_name: greg_2_super,
            display_name: "Greg 2 Super",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        Greg2Ultra {
            model_name: "greg-2-ultra",
            constructor_name: greg_2_ultra,
            display_name: "Greg 2 Ultra",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        GregRp {
            model_name: "greg-rp",
            constructor_name: greg_rp,
            display_name: "Greg (Roleplay)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        KimiK25 {
            model_name: "kimi-k2.5",
            constructor_name: kimi_k2_5,
            display_name: "Kimi K2.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        KimiK25Lightning {
            model_name: "kimi-k2.5-lightning",
            constructor_name: kimi_k2_5_lightning,
            display_name: "Kimi K2.5 (Lightning)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        KimiK26 {
            model_name: "kimi-k2.6",
            constructor_name: kimi_k2_6,
            display_name: "Kimi K2.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        KimiK27Code {
            model_name: "kimi-k2.7-code",
            constructor_name: kimi_k2_7_code,
            display_name: "Kimi K2.7 Code",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        KimiK3 {
            model_name: "kimi-k3",
            constructor_name: kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        KimiK3Eco {
            model_name: "kimi-k3-eco",
            constructor_name: kimi_k3_eco,
            display_name: "Kimi K3 Eco",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MimoV25Pro {
            model_name: "mimo-v2.5-pro",
            constructor_name: mimo_v2_5_pro,
            display_name: "MiMo-V2.5-Pro",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxM25 {
            model_name: "minimax-m2.5",
            constructor_name: minimax_m2_5,
            display_name: "MiniMax-M2.5",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen35397bA17b {
            model_name: "qwen3.5-397b-a17b",
            constructor_name: qwen3_5_397b_a17b,
            display_name: "Qwen3.5 397B-A17B",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen359b {
            model_name: "qwen3.5-9b",
            constructor_name: qwen3_5_9b,
            display_name: "Qwen3.5 9B",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen3627b {
            model_name: "qwen3.6-27b",
            constructor_name: qwen3_6_27b,
            display_name: "Qwen3.6 27B",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
