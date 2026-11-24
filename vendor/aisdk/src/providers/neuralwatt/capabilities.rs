//! Capabilities for neuralwatt models.
//!
//! This module defines model types and their capabilities for neuralwatt providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::neuralwatt::Neuralwatt;

model_capabilities! {
    provider: Neuralwatt,
    models: {
        DeepseekV4Flash {
            model_name: "deepseek-v4-flash",
            constructor_name: deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV4FlashFlex {
            model_name: "deepseek-v4-flash-flex",
            constructor_name: deepseek_v4_flash_flex,
            display_name: "DeepSeek V4 Flash Flex",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV4FlashSpeed {
            model_name: "deepseek-v4-flash-speed",
            constructor_name: deepseek_v4_flash_speed,
            display_name: "DeepSeek V4 Flash (Speed)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV41Flash {
            model_name: "deepseek-v4.1-flash",
            constructor_name: deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV41FlashFlex {
            model_name: "deepseek-v4.1-flash-flex",
            constructor_name: deepseek_v4_1_flash_flex,
            display_name: "DeepSeek V4.1 Flash Flex",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Gemma431b {
            model_name: "gemma-4-31b",
            constructor_name: gemma_4_31b,
            display_name: "Gemma 4 31B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53 {
            model_name: "glm-5.3",
            constructor_name: glm_5_3,
            display_name: "GLM 5.3",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53Flash {
            model_name: "glm-5.3-flash",
            constructor_name: glm_5_3_flash,
            display_name: "GLM-5.3 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53FlashFlex {
            model_name: "glm-5.3-flash-flex",
            constructor_name: glm_5_3_flash_flex,
            display_name: "GLM-5.3 Flash Flex",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53Flex {
            model_name: "glm-5.3-flex",
            constructor_name: glm_5_3_flex,
            display_name: "GLM 5.3 Flex",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        KimiK3 {
            model_name: "kimi-k3",
            constructor_name: kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        KimiK3Fast {
            model_name: "kimi-k3-fast",
            constructor_name: kimi_k3_fast,
            display_name: "Kimi K3 Fast",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        KimiK3Flex {
            model_name: "kimi-k3-flex",
            constructor_name: kimi_k3_flex,
            display_name: "Kimi K3 Flex",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MimoV26Pro {
            model_name: "mimo-v2.6-pro",
            constructor_name: mimo_v2_6_pro,
            display_name: "MiMo-V2.6-Pro",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MimoV26ProFlex {
            model_name: "mimo-v2.6-pro-flex",
            constructor_name: mimo_v2_6_pro_flex,
            display_name: "MiMo-V2.6-Pro Flex",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3827b {
            model_name: "qwen-3.8-27b",
            constructor_name: qwen_3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3827bFlex {
            model_name: "qwen-3.8-27b-flex",
            constructor_name: qwen_3_8_27b_flex,
            display_name: "Qwen3.8 27B Flex",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3635b {
            model_name: "qwen3.6-35b",
            constructor_name: qwen3_6_35b,
            display_name: "Qwen3.6 35B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3635bFast {
            model_name: "qwen3.6-35b-fast",
            constructor_name: qwen3_6_35b_fast,
            display_name: "Qwen3.6 35B Fast",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3635bFlex {
            model_name: "qwen3.6-35b-flex",
            constructor_name: qwen3_6_35b_flex,
            display_name: "Qwen3.6 35B Flex",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
