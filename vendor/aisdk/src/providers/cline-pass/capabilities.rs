//! Capabilities for cline_pass models.
//!
//! This module defines model types and their capabilities for cline_pass providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::cline_pass::ClinePass;

model_capabilities! {
    provider: ClinePass,
    models: {
        ClinePassDeepseekV4Pro {
            model_name: "cline-pass/deepseek-v4-pro",
            constructor_name: cline_pass_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ClinePassDeepseekV41Flash {
            model_name: "cline-pass/deepseek-v4.1-flash",
            constructor_name: cline_pass_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ClinePassGlm53 {
            model_name: "cline-pass/glm-5.3",
            constructor_name: cline_pass_glm_5_3,
            display_name: "GLM-5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ClinePassGlm53Flash {
            model_name: "cline-pass/glm-5.3-flash",
            constructor_name: cline_pass_glm_5_3_flash,
            display_name: "cline-pass/glm-5.3-flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ClinePassKimiK3 {
            model_name: "cline-pass/kimi-k3",
            constructor_name: cline_pass_kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ClinePassMimoV25 {
            model_name: "cline-pass/mimo-v2.5",
            constructor_name: cline_pass_mimo_v2_5,
            display_name: "MiMo-V2.5",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ClinePassMimoV25Pro {
            model_name: "cline-pass/mimo-v2.5-pro",
            constructor_name: cline_pass_mimo_v2_5_pro,
            display_name: "MiMo-V2.5-Pro",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ClinePassMinimaxM3 {
            model_name: "cline-pass/minimax-m3",
            constructor_name: cline_pass_minimax_m3,
            display_name: "MiniMax-M3",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ClinePassMuseSpark13Contributor {
            model_name: "cline-pass/muse-spark-1.3-contributor",
            constructor_name: cline_pass_muse_spark_1_3_contributor,
            display_name: "Muse Spark 1.3 Contributor",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ClinePassQwen37Max {
            model_name: "cline-pass/qwen3.7-max",
            constructor_name: cline_pass_qwen3_7_max,
            display_name: "Qwen3.7 Max",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ClinePassQwen37Plus {
            model_name: "cline-pass/qwen3.7-plus",
            constructor_name: cline_pass_qwen3_7_plus,
            display_name: "Qwen3.7 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ClinePassQwen38Max {
            model_name: "cline-pass/qwen3.8-max",
            constructor_name: cline_pass_qwen3_8_max,
            display_name: "Qwen3.8 Max",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
