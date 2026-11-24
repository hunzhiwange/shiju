//! Capabilities for inco models.
//!
//! This module defines model types and their capabilities for inco providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::inco::Inco;

model_capabilities! {
    provider: Inco,
    models: {
        DeepseekV41FlashFast {
            model_name: "deepseek-v4.1-flash:fast",
            constructor_name: deepseek_v4_1_flash_fast,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53 {
            model_name: "glm-5.3",
            constructor_name: glm_5_3,
            display_name: "GLM-5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53FlashFast {
            model_name: "glm-5.3-flash:fast",
            constructor_name: glm_5_3_flash_fast,
            display_name: "GLM-5.3-Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Glm53Fast {
            model_name: "glm-5.3:fast",
            constructor_name: glm_5_3_fast,
            display_name: "GLM-5.3 Fast",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        KimiK3Fast {
            model_name: "kimi-k3:fast",
            constructor_name: kimi_k3_fast,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MinimaxM3 {
            model_name: "minimax-m3",
            constructor_name: minimax_m3,
            display_name: "MiniMax-M3",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MinimaxM3Fast {
            model_name: "minimax-m3:fast",
            constructor_name: minimax_m3_fast,
            display_name: "MiniMax M3 Fast",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
