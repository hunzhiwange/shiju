//! Capabilities for fireworks_ai models.
//!
//! This module defines model types and their capabilities for fireworks_ai providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::fireworks_ai::FireworksAi;

model_capabilities! {
    provider: FireworksAi,
    models: {
        AccountsFireworksModelsDeepseekV4p1Flash {
            model_name: "accounts/fireworks/models/deepseek-v4p1-flash",
            constructor_name: accounts_fireworks_models_deepseek_v4p1_flash,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksModelsDeepseekV4p1Flash1017 {
            model_name: "accounts/fireworks/models/deepseek-v4p1-flash-1017",
            constructor_name: accounts_fireworks_models_deepseek_v4p1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksModelsEmber1 {
            model_name: "accounts/fireworks/models/ember-1",
            constructor_name: accounts_fireworks_models_ember_1,
            display_name: "Ember-1",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksModelsGlm5p3 {
            model_name: "accounts/fireworks/models/glm-5p3",
            constructor_name: accounts_fireworks_models_glm_5p3,
            display_name: "GLM 5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksModelsGlm5p3Flash {
            model_name: "accounts/fireworks/models/glm-5p3-flash",
            constructor_name: accounts_fireworks_models_glm_5p3_flash,
            display_name: "GLM 5.3 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksModelsKimiK3 {
            model_name: "accounts/fireworks/models/kimi-k3",
            constructor_name: accounts_fireworks_models_kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksModelsMinimaxM3 {
            model_name: "accounts/fireworks/models/minimax-m3",
            constructor_name: accounts_fireworks_models_minimax_m3,
            display_name: "MiniMax-M3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksModelsMinimaxM3p1Flash {
            model_name: "accounts/fireworks/models/minimax-m3p1-flash",
            constructor_name: accounts_fireworks_models_minimax_m3p1_flash,
            display_name: "MiniMax-M3.1-Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksModelsMuseSparkOpen {
            model_name: "accounts/fireworks/models/muse-spark-open",
            constructor_name: accounts_fireworks_models_muse_spark_open,
            display_name: "Muse Spark Open",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksModelsQwen3p82p4tA95b {
            model_name: "accounts/fireworks/models/qwen3p8-2p4t-a95b",
            constructor_name: accounts_fireworks_models_qwen3p8_2p4t_a95b,
            display_name: "Qwen3.8 2.4T A95B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksModelsQwen3p8Max {
            model_name: "accounts/fireworks/models/qwen3p8-max",
            constructor_name: accounts_fireworks_models_qwen3p8_max,
            display_name: "Qwen3.8 Max",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksRoutersDeepseekFlashLatest {
            model_name: "accounts/fireworks/routers/deepseek-flash-latest",
            constructor_name: accounts_fireworks_routers_deepseek_flash_latest,
            display_name: "DeepSeek Flash Latest",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksRoutersDeepseekV4p1Flash1017Spot {
            model_name: "accounts/fireworks/routers/deepseek-v4p1-flash-1017-spot",
            constructor_name: accounts_fireworks_routers_deepseek_v4p1_flash_1017_spot,
            display_name: "DeepSeek V4.1 Flash 1017 Spot",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksRoutersDeepseekV4p1FlashSpot {
            model_name: "accounts/fireworks/routers/deepseek-v4p1-flash-spot",
            constructor_name: accounts_fireworks_routers_deepseek_v4p1_flash_spot,
            display_name: "DeepSeek V4.1 Flash Spot",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksRoutersGlm5p3Fast {
            model_name: "accounts/fireworks/routers/glm-5p3-fast",
            constructor_name: accounts_fireworks_routers_glm_5p3_fast,
            display_name: "GLM 5.3 Fast",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksRoutersGlm5p3Spot {
            model_name: "accounts/fireworks/routers/glm-5p3-spot",
            constructor_name: accounts_fireworks_routers_glm_5p3_spot,
            display_name: "GLM 5.3 Spot",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksRoutersGlmFastLatest {
            model_name: "accounts/fireworks/routers/glm-fast-latest",
            constructor_name: accounts_fireworks_routers_glm_fast_latest,
            display_name: "GLM 5.3 Fast (Latest)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksRoutersGlmFlashLatest {
            model_name: "accounts/fireworks/routers/glm-flash-latest",
            constructor_name: accounts_fireworks_routers_glm_flash_latest,
            display_name: "GLM Flash Latest (GLM 5.3 Flash)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksRoutersGlmLatest {
            model_name: "accounts/fireworks/routers/glm-latest",
            constructor_name: accounts_fireworks_routers_glm_latest,
            display_name: "GLM Latest",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksRoutersKimiFastLatest {
            model_name: "accounts/fireworks/routers/kimi-fast-latest",
            constructor_name: accounts_fireworks_routers_kimi_fast_latest,
            display_name: "Kimi Fast Latest",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksRoutersKimiK3Fast {
            model_name: "accounts/fireworks/routers/kimi-k3-fast",
            constructor_name: accounts_fireworks_routers_kimi_k3_fast,
            display_name: "Kimi K3 Fast",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksRoutersKimiK3Spot {
            model_name: "accounts/fireworks/routers/kimi-k3-spot",
            constructor_name: accounts_fireworks_routers_kimi_k3_spot,
            display_name: "Kimi K3 Spot",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksRoutersKimiLatest {
            model_name: "accounts/fireworks/routers/kimi-latest",
            constructor_name: accounts_fireworks_routers_kimi_latest,
            display_name: "Kimi Latest",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksRoutersMinimaxLatest {
            model_name: "accounts/fireworks/routers/minimax-latest",
            constructor_name: accounts_fireworks_routers_minimax_latest,
            display_name: "MiniMax Latest",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AccountsFireworksRoutersQwenMaxLatest {
            model_name: "accounts/fireworks/routers/qwen-max-latest",
            constructor_name: accounts_fireworks_routers_qwen_max_latest,
            display_name: "Qwen Max Latest (Qwen3.8 Max)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
