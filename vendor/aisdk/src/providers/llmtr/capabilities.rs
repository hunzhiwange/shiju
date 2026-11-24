//! Capabilities for llmtr models.
//!
//! This module defines model types and their capabilities for llmtr providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::llmtr::Llmtr;

model_capabilities! {
    provider: Llmtr,
    models: {
        Gemma4 {
            model_name: "gemma-4",
            constructor_name: gemma_4,
            display_name: "Gemma 4",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Magibu11bV8 {
            model_name: "magibu-11b-v8",
            constructor_name: magibu_11b_v8,
            display_name: "Magibu 11B v8",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        MetaMuseSpark12Contributor {
            model_name: "meta/muse-spark-1.2-contributor",
            constructor_name: meta_muse_spark_1_2_contributor,
            display_name: "Muse Spark 1.2 Contributor",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MistralVoxtralSmallLatest {
            model_name: "mistral/voxtral-small-latest",
            constructor_name: mistral_voxtral_small_latest,
            display_name: "Voxtral Small (latest)",
            capabilities: [AudioInputSupport, ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MuseGlimmer30bTr {
            model_name: "muse-glimmer-30b-tr",
            constructor_name: muse_glimmer_30b_tr,
            display_name: "Muse Glimmer 30B (TR)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        PerplexitySonarDeepResearch {
            model_name: "perplexity/sonar-deep-research",
            constructor_name: perplexity_sonar_deep_research,
            display_name: "Sonar Deep Research",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        PoolsideLagunaXs21 {
            model_name: "poolside/laguna-xs-2.1",
            constructor_name: poolside_laguna_xs_2_1,
            display_name: "Laguna XS 2.1",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        PublicaiApertus70bInstruct {
            model_name: "publicai/apertus-70b-instruct",
            constructor_name: publicai_apertus_70b_instruct,
            display_name: "Apertus 70B Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        PublicaiApertus8bInstruct {
            model_name: "publicai/apertus-8b-instruct",
            constructor_name: publicai_apertus_8b_instruct,
            display_name: "Apertus 8B Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwenFlash {
            model_name: "qwen/qwen-flash",
            constructor_name: qwen_qwen_flash,
            display_name: "Qwen Flash",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwenPlus {
            model_name: "qwen/qwen-plus",
            constructor_name: qwen_qwen_plus,
            display_name: "Qwen Plus",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3CoderFlash {
            model_name: "qwen/qwen3-coder-flash",
            constructor_name: qwen_qwen3_coder_flash,
            display_name: "Qwen3 Coder Flash",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3VlPlus {
            model_name: "qwen/qwen3-vl-plus",
            constructor_name: qwen_qwen3_vl_plus,
            display_name: "Qwen3-VL Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen35397bA17b {
            model_name: "qwen/qwen3.5-397b-a17b",
            constructor_name: qwen_qwen3_5_397b_a17b,
            display_name: "Qwen3.5 397B-A17B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen35Plus {
            model_name: "qwen/qwen3.5-plus",
            constructor_name: qwen_qwen3_5_plus,
            display_name: "Qwen3.5 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen36Flash {
            model_name: "qwen/qwen3.6-flash",
            constructor_name: qwen_qwen3_6_flash,
            display_name: "Qwen3.6 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen36Plus {
            model_name: "qwen/qwen3.6-plus",
            constructor_name: qwen_qwen3_6_plus,
            display_name: "Qwen3.6 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen37Plus {
            model_name: "qwen/qwen3.7-plus",
            constructor_name: qwen_qwen3_7_plus,
            display_name: "Qwen3.7 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen3635b {
            model_name: "qwen3-6-35b",
            constructor_name: qwen3_6_35b,
            display_name: "Qwen3.6 35B-A3B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, VideoInputSupport]
        },
        SakanaFuguUltra {
            model_name: "sakana/fugu-ultra",
            constructor_name: sakana_fugu_ultra,
            display_name: "Fugu Ultra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ThinkingmachinesInkling {
            model_name: "thinkingmachines/inkling",
            constructor_name: thinkingmachines_inkling,
            display_name: "Inkling",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ThinkingmachinesInklingSmall {
            model_name: "thinkingmachines/inkling-small",
            constructor_name: thinkingmachines_inkling_small,
            display_name: "Inkling Small",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TrendyolAsure12b {
            model_name: "trendyol-asure-12b",
            constructor_name: trendyol_asure_12b,
            display_name: "Trendyol Asure 12B",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        UpstageSolarPro2 {
            model_name: "upstage/solar-pro2",
            constructor_name: upstage_solar_pro2,
            display_name: "Solar Pro 2",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UpstageSolarPro3 {
            model_name: "upstage/solar-pro3",
            constructor_name: upstage_solar_pro3,
            display_name: "Solar Pro 3",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UpstageSolarPro4 {
            model_name: "upstage/solar-pro4",
            constructor_name: upstage_solar_pro4,
            display_name: "Solar Pro 4",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
