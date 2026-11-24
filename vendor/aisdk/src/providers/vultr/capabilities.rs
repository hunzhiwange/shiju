//! Capabilities for vultr models.
//!
//! This module defines model types and their capabilities for vultr providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::vultr::Vultr;

model_capabilities! {
    provider: Vultr,
    models: {
        DeepseekV41Flash {
            model_name: "deepseek-v4.1-flash",
            constructor_name: deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm52 {
            model_name: "glm-5.2",
            constructor_name: glm_5_2,
            display_name: "GLM-5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53 {
            model_name: "glm-5.3",
            constructor_name: glm_5_3,
            display_name: "GLM-5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53Flash {
            model_name: "glm-5.3-flash",
            constructor_name: glm_5_3_flash,
            display_name: "GLM-5.3-Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Glm5XMenthol {
            model_name: "glm-5.x-menthol",
            constructor_name: glm_5_x_menthol,
            display_name: "GLM-5",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MimoV26FlashRl {
            model_name: "mimo-v2.6-flash-rl",
            constructor_name: mimo_v2_6_flash_rl,
            display_name: "MiMo-V2.6-Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MimoV26ProRl {
            model_name: "mimo-v2.6-pro-rl",
            constructor_name: mimo_v2_6_pro_rl,
            display_name: "MiMo-V2.6-Pro",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MinimaxM3 {
            model_name: "minimax-m3",
            constructor_name: minimax_m3,
            display_name: "MiniMax-M3",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MuseGlimmer30b {
            model_name: "muse-glimmer-30b",
            constructor_name: muse_glimmer_30b,
            display_name: "Muse Glimmer 30B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Nemotron3NanoOmni30bA3bReasoning {
            model_name: "nemotron-3-nano-omni-30b-a3b-reasoning",
            constructor_name: nemotron_3_nano_omni_30b_a3b_reasoning,
            display_name: "Nemotron 3 Nano Omni 30B A3B Reasoning",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Nemotron35ContentSafety {
            model_name: "nemotron-3.5-content-safety",
            constructor_name: nemotron_3_5_content_safety,
            display_name: "Nemotron 3.5 Content Safety",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        Qwen3827b {
            model_name: "qwen3.8-27b",
            constructor_name: qwen3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen38FlashNext {
            model_name: "qwen3.8-flash-next",
            constructor_name: qwen3_8_flash_next,
            display_name: "Qwen3.8 Flash Next",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
