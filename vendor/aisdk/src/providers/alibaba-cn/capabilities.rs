//! Capabilities for alibaba_cn models.
//!
//! This module defines model types and their capabilities for alibaba_cn providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::alibaba_cn::AlibabaCn;

model_capabilities! {
    provider: AlibabaCn,
    models: {
        MinimaxM25 {
            model_name: "MiniMax-M2.5",
            constructor_name: minimax_m2_5,
            display_name: "MiniMax-M2.5",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM27 {
            model_name: "MiniMax/MiniMax-M2.7",
            constructor_name: minimax_minimax_m2_7,
            display_name: "MiniMax-M2.7",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekR1DistillLlama70b {
            model_name: "deepseek-r1-distill-llama-70b",
            constructor_name: deepseek_r1_distill_llama_70b,
            display_name: "DeepSeek R1 Distill Llama 70B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        DeepseekR1DistillQwen15b {
            model_name: "deepseek-r1-distill-qwen-1-5b",
            constructor_name: deepseek_r1_distill_qwen_1_5b,
            display_name: "DeepSeek R1 Distill Qwen 1.5B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        DeepseekV4Pro {
            model_name: "deepseek-v4-pro",
            constructor_name: deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV41Flash {
            model_name: "deepseek-v4.1-flash",
            constructor_name: deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
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
        Glm53 {
            model_name: "glm-5.3",
            constructor_name: glm_5_3,
            display_name: "GLM-5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        KimiK25 {
            model_name: "kimi-k2.5",
            constructor_name: kimi_k2_5,
            display_name: "Moonshot Kimi K2.5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        KimiK26 {
            model_name: "kimi-k2.6",
            constructor_name: kimi_k2_6,
            display_name: "Moonshot Kimi K2.6",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        KimiK3 {
            model_name: "kimi-k3",
            constructor_name: kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        KimiKimiK25 {
            model_name: "kimi/kimi-k2.5",
            constructor_name: kimi_kimi_k2_5,
            display_name: "kimi/kimi-k2.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenDeepResearch {
            model_name: "qwen-deep-research",
            constructor_name: qwen_deep_research,
            display_name: "Qwen Deep Research",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenDocTurbo {
            model_name: "qwen-doc-turbo",
            constructor_name: qwen_doc_turbo,
            display_name: "Qwen Doc Turbo",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenFlash {
            model_name: "qwen-flash",
            constructor_name: qwen_flash,
            display_name: "Qwen Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenLong {
            model_name: "qwen-long",
            constructor_name: qwen_long,
            display_name: "Qwen Long",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        QwenMax {
            model_name: "qwen-max",
            constructor_name: qwen_max,
            display_name: "Qwen Max",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenMtPlus {
            model_name: "qwen-mt-plus",
            constructor_name: qwen_mt_plus,
            display_name: "Qwen-MT Plus",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        QwenPlus {
            model_name: "qwen-plus",
            constructor_name: qwen_plus,
            display_name: "Qwen Plus",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenPlusCharacter {
            model_name: "qwen-plus-character",
            constructor_name: qwen_plus_character,
            display_name: "Qwen Plus Character",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        QwenVlOcr {
            model_name: "qwen-vl-ocr",
            constructor_name: qwen_vl_ocr,
            display_name: "Qwen-VL OCR",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        Qwen3AsrFlash {
            model_name: "qwen3-asr-flash",
            constructor_name: qwen3_asr_flash,
            display_name: "Qwen3-ASR Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, TextOutputSupport]
        },
        Qwen3CoderFlash {
            model_name: "qwen3-coder-flash",
            constructor_name: qwen3_coder_flash,
            display_name: "Qwen3 Coder Flash",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3OmniFlash {
            model_name: "qwen3-omni-flash",
            constructor_name: qwen3_omni_flash,
            display_name: "Qwen3-Omni Flash",
            capabilities: [AudioInputSupport, AudioOutputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen3VlPlus {
            model_name: "qwen3-vl-plus",
            constructor_name: qwen3_vl_plus,
            display_name: "Qwen3-VL Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen35397bA17b {
            model_name: "qwen3.5-397b-a17b",
            constructor_name: qwen3_5_397b_a17b,
            display_name: "Qwen3.5 397B-A17B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen35Flash {
            model_name: "qwen3.5-flash",
            constructor_name: qwen3_5_flash,
            display_name: "Qwen3.5 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen35Plus {
            model_name: "qwen3.5-plus",
            constructor_name: qwen3_5_plus,
            display_name: "Qwen3.5 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen36Flash {
            model_name: "qwen3.6-flash",
            constructor_name: qwen3_6_flash,
            display_name: "Qwen3.6 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen36Plus {
            model_name: "qwen3.6-plus",
            constructor_name: qwen3_6_plus,
            display_name: "Qwen3.6 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen37Flash {
            model_name: "qwen3.7-flash",
            constructor_name: qwen3_7_flash,
            display_name: "Qwen3.7 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen37Max {
            model_name: "qwen3.7-max",
            constructor_name: qwen3_7_max,
            display_name: "Qwen3.7 Max",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen37Plus {
            model_name: "qwen3.7-plus",
            constructor_name: qwen3_7_plus,
            display_name: "Qwen3.7 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen38Flash {
            model_name: "qwen3.8-flash",
            constructor_name: qwen3_8_flash,
            display_name: "Qwen3.8 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen38Max {
            model_name: "qwen3.8-max",
            constructor_name: qwen3_8_max,
            display_name: "Qwen3.8 Max",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen38OmniFlash {
            model_name: "qwen3.8-omni-flash",
            constructor_name: qwen3_8_omni_flash,
            display_name: "Qwen3.8 Omni Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        SiliconflowDeepseekR10528 {
            model_name: "siliconflow/deepseek-r1-0528",
            constructor_name: siliconflow_deepseek_r1_0528,
            display_name: "siliconflow/deepseek-r1-0528",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        SiliconflowDeepseekV30324 {
            model_name: "siliconflow/deepseek-v3-0324",
            constructor_name: siliconflow_deepseek_v3_0324,
            display_name: "siliconflow/deepseek-v3-0324",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        SiliconflowDeepseekV31Terminus {
            model_name: "siliconflow/deepseek-v3.1-terminus",
            constructor_name: siliconflow_deepseek_v3_1_terminus,
            display_name: "siliconflow/deepseek-v3.1-terminus",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        SiliconflowDeepseekV32 {
            model_name: "siliconflow/deepseek-v3.2",
            constructor_name: siliconflow_deepseek_v3_2,
            display_name: "siliconflow/deepseek-v3.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TongyiIntentDetectV3 {
            model_name: "tongyi-intent-detect-v3",
            constructor_name: tongyi_intent_detect_v3,
            display_name: "Tongyi Intent Detect V3",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
    }
}
