//! Capabilities for empiriolabs models.
//!
//! This module defines model types and their capabilities for empiriolabs providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::empiriolabs::Empiriolabs;

model_capabilities! {
    provider: Empiriolabs,
    models: {
        DeepseekV41Flash1017 {
            model_name: "deepseek-v4-1-flash-1017",
            constructor_name: deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV4Flash0731 {
            model_name: "deepseek-v4-flash-0731",
            constructor_name: deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV4Pro {
            model_name: "deepseek-v4-pro",
            constructor_name: deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV4Pro0813 {
            model_name: "deepseek-v4-pro-0813",
            constructor_name: deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FuguMax {
            model_name: "fugu-max",
            constructor_name: fugu_max,
            display_name: "Fugu Max",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FuguUltraV10 {
            model_name: "fugu-ultra-v1-0",
            constructor_name: fugu_ultra_v1_0,
            display_name: "Fugu Ultra v1.0",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FuguUltraV11 {
            model_name: "fugu-ultra-v1-1",
            constructor_name: fugu_ultra_v1_1,
            display_name: "Fugu Ultra v1.1",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FuguUltraV20 {
            model_name: "fugu-ultra-v2-0",
            constructor_name: fugu_ultra_v2_0,
            display_name: "Fugu Ultra v2.0",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Gemma327b {
            model_name: "gemma-3-27b",
            constructor_name: gemma_3_27b,
            display_name: "Gemma 3 27B",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        Gemma426bA4b {
            model_name: "gemma-4-26b-a4b",
            constructor_name: gemma_4_26b_a4b,
            display_name: "Gemma 4 26B-A4B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Glm45Flash {
            model_name: "glm-4-5-flash",
            constructor_name: glm_4_5_flash,
            display_name: "GLM 4.5 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm46vFlash {
            model_name: "glm-4-6v-flash",
            constructor_name: glm_4_6v_flash,
            display_name: "GLM 4.6V Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Glm47Flash {
            model_name: "glm-4-7-flash",
            constructor_name: glm_4_7_flash,
            display_name: "GLM 4.7 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm51 {
            model_name: "glm-5-1",
            constructor_name: glm_5_1,
            display_name: "GLM 5.1",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm52 {
            model_name: "glm-5-2",
            constructor_name: glm_5_2,
            display_name: "GLM 5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53 {
            model_name: "glm-5-3",
            constructor_name: glm_5_3,
            display_name: "GLM 5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53Flash {
            model_name: "glm-5-3-flash",
            constructor_name: glm_5_3_flash,
            display_name: "GLM 5.3 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        KimiK26 {
            model_name: "kimi-k2-6",
            constructor_name: kimi_k2_6,
            display_name: "Kimi K2.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        KimiK27Code {
            model_name: "kimi-k2-7-code",
            constructor_name: kimi_k2_7_code,
            display_name: "Kimi K2.7 Code",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        KimiK27CodeHighspeed {
            model_name: "kimi-k2-7-code-highspeed",
            constructor_name: kimi_k2_7_code_highspeed,
            display_name: "Kimi K2.7 Code Highspeed",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        KimiK3 {
            model_name: "kimi-k3",
            constructor_name: kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MimoV26Flash {
            model_name: "mimo-v2-6-flash",
            constructor_name: mimo_v2_6_flash,
            display_name: "MiMo V2.6 Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MimoV26Pro {
            model_name: "mimo-v2-6-pro",
            constructor_name: mimo_v2_6_pro,
            display_name: "MiMo V2.6 Pro",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MimoV26ProUltraspeed {
            model_name: "mimo-v2-6-pro-ultraspeed",
            constructor_name: mimo_v2_6_pro_ultraspeed,
            display_name: "MiMo V2.6 Pro UltraSpeed",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MinimaxM3 {
            model_name: "minimax-m3",
            constructor_name: minimax_m3,
            display_name: "MiniMax M3",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MinimaxM31Flash {
            model_name: "minimax-m3-1-flash",
            constructor_name: minimax_m3_1_flash,
            display_name: "MiniMax M3.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MistralSmall4 {
            model_name: "mistral-small-4",
            constructor_name: mistral_small_4,
            display_name: "Mistral Small 4",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MuseGlimmer30b {
            model_name: "muse-glimmer-30b",
            constructor_name: muse_glimmer_30b,
            display_name: "Muse Glimmer 30B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MuseSpark11 {
            model_name: "muse-spark-1-1",
            constructor_name: muse_spark_1_1,
            display_name: "Muse Spark 1.1",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MuseSpark12 {
            model_name: "muse-spark-1-2",
            constructor_name: muse_spark_1_2,
            display_name: "Muse Spark 1.2",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MuseSpark13 {
            model_name: "muse-spark-1-3",
            constructor_name: muse_spark_1_3,
            display_name: "Muse Spark 1.3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen35122bA10b {
            model_name: "qwen3-5-122b-a10b",
            constructor_name: qwen3_5_122b_a10b,
            display_name: "Qwen3.5 122B-A10B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen3527b {
            model_name: "qwen3-5-27b",
            constructor_name: qwen3_5_27b,
            display_name: "Qwen3.5 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen3535bA3b {
            model_name: "qwen3-5-35b-a3b",
            constructor_name: qwen3_5_35b_a3b,
            display_name: "Qwen3.5 35B-A3B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen354b {
            model_name: "qwen3-5-4b",
            constructor_name: qwen3_5_4b,
            display_name: "Qwen3.5 4B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen359b {
            model_name: "qwen3-5-9b",
            constructor_name: qwen3_5_9b,
            display_name: "Qwen3.5 9B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen35Flash {
            model_name: "qwen3-5-flash",
            constructor_name: qwen3_5_flash,
            display_name: "Qwen3.5 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen3627b {
            model_name: "qwen3-6-27b",
            constructor_name: qwen3_6_27b,
            display_name: "Qwen3.6 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen3635bA3b {
            model_name: "qwen3-6-35b-a3b",
            constructor_name: qwen3_6_35b_a3b,
            display_name: "Qwen3.6 35B A3B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen36Flash {
            model_name: "qwen3-6-flash",
            constructor_name: qwen3_6_flash,
            display_name: "Qwen3.6 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen36Plus {
            model_name: "qwen3-6-plus",
            constructor_name: qwen3_6_plus,
            display_name: "Qwen3.6 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen37Flash {
            model_name: "qwen3-7-flash",
            constructor_name: qwen3_7_flash,
            display_name: "Qwen3.7 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen37Max {
            model_name: "qwen3-7-max",
            constructor_name: qwen3_7_max,
            display_name: "Qwen3.7 Max",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen37Plus {
            model_name: "qwen3-7-plus",
            constructor_name: qwen3_7_plus,
            display_name: "Qwen3.7 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen3827b {
            model_name: "qwen3-8-27b",
            constructor_name: qwen3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen38Flash {
            model_name: "qwen3-8-flash",
            constructor_name: qwen3_8_flash,
            display_name: "Qwen3.8 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen38Max {
            model_name: "qwen3-8-max",
            constructor_name: qwen3_8_max,
            display_name: "Qwen3.8 Max",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen38Max0902 {
            model_name: "qwen3-8-max-0902",
            constructor_name: qwen3_8_max_0902,
            display_name: "Qwen3.8 Max 0902",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen38OmniFlash {
            model_name: "qwen3-8-omni-flash",
            constructor_name: qwen3_8_omni_flash,
            display_name: "Qwen3.8 Omni Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Seed20Code {
            model_name: "seed-2-0-code",
            constructor_name: seed_2_0_code,
            display_name: "Seed 2.0 Code",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Seed20Lite {
            model_name: "seed-2-0-lite",
            constructor_name: seed_2_0_lite,
            display_name: "Seed 2.0 Lite",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Seed20Mini {
            model_name: "seed-2-0-mini",
            constructor_name: seed_2_0_mini,
            display_name: "Seed 2.0 Mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Seed20Pro {
            model_name: "seed-2-0-pro",
            constructor_name: seed_2_0_pro,
            display_name: "Seed 2.0 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Seed21Turbo {
            model_name: "seed-2-1-turbo",
            constructor_name: seed_2_1_turbo,
            display_name: "Seed 2.1 Turbo",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Step35Flash {
            model_name: "step-3-5-flash",
            constructor_name: step_3_5_flash,
            display_name: "Step 3.5 Flash",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Step35Flash2603 {
            model_name: "step-3-5-flash-2603",
            constructor_name: step_3_5_flash_2603,
            display_name: "Step 3.5 Flash 2603",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Step37Flash {
            model_name: "step-3-7-flash",
            constructor_name: step_3_7_flash,
            display_name: "Step 3.7 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Step5Preview {
            model_name: "step-5-preview",
            constructor_name: step_5_preview,
            display_name: "Step 5 Preview",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
