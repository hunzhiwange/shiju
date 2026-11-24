//! Capabilities for cloudflare_workers_ai models.
//!
//! This module defines model types and their capabilities for cloudflare_workers_ai providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::cloudflare_workers_ai::CloudflareWorkersAi;

model_capabilities! {
    provider: CloudflareWorkersAi,
    models: {
        CfAisingaporeGemmaSeaLionV427bIt {
            model_name: "@cf/aisingapore/gemma-sea-lion-v4-27b-it",
            constructor_name: cf_aisingapore_gemma_sea_lion_v4_27b_it,
            display_name: "Gemma Sea Lion V4 27B It",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        CfCloudflareClairic {
            model_name: "@cf/cloudflare/clairic",
            constructor_name: cf_cloudflare_clairic,
            display_name: "Clairic",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        CfCloudflareClef {
            model_name: "@cf/cloudflare/clef",
            constructor_name: cf_cloudflare_clef,
            display_name: "Clef",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        CfCloudflareClefFlash {
            model_name: "@cf/cloudflare/clef-flash",
            constructor_name: cf_cloudflare_clef_flash,
            display_name: "Clef Flash",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        CfCloudflareClefOmni {
            model_name: "@cf/cloudflare/clef-omni",
            constructor_name: cf_cloudflare_clef_omni,
            display_name: "Clef Omni",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        CfDeepseekAiDeepseekR1DistillQwen32b {
            model_name: "@cf/deepseek-ai/deepseek-r1-distill-qwen-32b",
            constructor_name: cf_deepseek_ai_deepseek_r1_distill_qwen_32b,
            display_name: "Deepseek R1 Distill Qwen 32B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        CfDeepseekAiDeepseekV4Flash0731 {
            model_name: "@cf/deepseek-ai/deepseek-v4-flash-0731",
            constructor_name: cf_deepseek_ai_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfDeepseekAiDeepseekV4Pro0813 {
            model_name: "@cf/deepseek-ai/deepseek-v4-pro-0813",
            constructor_name: cf_deepseek_ai_deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfDeepseekAiDeepseekV41Flash1017 {
            model_name: "@cf/deepseek-ai/deepseek-v4.1-flash-1017",
            constructor_name: cf_deepseek_ai_deepseek_v4_1_flash_1017,
            display_name: "Deepseek V4.1 Flash 1017",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfGoogleGemma426bA4bIt {
            model_name: "@cf/google/gemma-4-26b-a4b-it",
            constructor_name: cf_google_gemma_4_26b_a4b_it,
            display_name: "Gemma 4 26B A4B IT",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfIbmGraniteGranite40HMicro {
            model_name: "@cf/ibm-granite/granite-4.0-h-micro",
            constructor_name: cf_ibm_granite_granite_4_0_h_micro,
            display_name: "Granite 4.0 H Micro",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfMetaLlama318bInstructFp8 {
            model_name: "@cf/meta/llama-3.1-8b-instruct-fp8",
            constructor_name: cf_meta_llama_3_1_8b_instruct_fp8,
            display_name: "Llama 3.1 8B Instruct fp8",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        CfMetaLlama3211bVisionInstruct {
            model_name: "@cf/meta/llama-3.2-11b-vision-instruct",
            constructor_name: cf_meta_llama_3_2_11b_vision_instruct,
            display_name: "Llama 3.2 11B Vision Instruct",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        CfMetaLlama321bInstruct {
            model_name: "@cf/meta/llama-3.2-1b-instruct",
            constructor_name: cf_meta_llama_3_2_1b_instruct,
            display_name: "Llama 3.2 1B Instruct",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        CfMetaLlama323bInstruct {
            model_name: "@cf/meta/llama-3.2-3b-instruct",
            constructor_name: cf_meta_llama_3_2_3b_instruct,
            display_name: "Llama 3.2 3B Instruct",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        CfMetaLlama3370bInstructFp8Fast {
            model_name: "@cf/meta/llama-3.3-70b-instruct-fp8-fast",
            constructor_name: cf_meta_llama_3_3_70b_instruct_fp8_fast,
            display_name: "Llama 3.3 70B Instruct fp8 Fast",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfMetaLlama4Scout17b16eInstruct {
            model_name: "@cf/meta/llama-4-scout-17b-16e-instruct",
            constructor_name: cf_meta_llama_4_scout_17b_16e_instruct,
            display_name: "Llama 4 Scout 17B 16E Instruct",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfMetaLlamaGuard38b {
            model_name: "@cf/meta/llama-guard-3-8b",
            constructor_name: cf_meta_llama_guard_3_8b,
            display_name: "Llama Guard 3 8B",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        CfMistralaiMistralSmall3124bInstruct {
            model_name: "@cf/mistralai/mistral-small-3.1-24b-instruct",
            constructor_name: cf_mistralai_mistral_small_3_1_24b_instruct,
            display_name: "Mistral Small 3.1 24B Instruct",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfMoonshotaiKimiK26 {
            model_name: "@cf/moonshotai/kimi-k2.6",
            constructor_name: cf_moonshotai_kimi_k2_6,
            display_name: "Kimi K2.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfMoonshotaiKimiK27Code {
            model_name: "@cf/moonshotai/kimi-k2.7-code",
            constructor_name: cf_moonshotai_kimi_k2_7_code,
            display_name: "Kimi K2.7 Code",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfNvidiaNemotron3120bA12b {
            model_name: "@cf/nvidia/nemotron-3-120b-a12b",
            constructor_name: cf_nvidia_nemotron_3_120b_a12b,
            display_name: "Nemotron 3 Super 120B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfOpenaiGptOss120b {
            model_name: "@cf/openai/gpt-oss-120b",
            constructor_name: cf_openai_gpt_oss_120b,
            display_name: "GPT OSS 120B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfOpenaiGptOss20b {
            model_name: "@cf/openai/gpt-oss-20b",
            constructor_name: cf_openai_gpt_oss_20b,
            display_name: "GPT OSS 20B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfQwenQwen25Coder32bInstruct {
            model_name: "@cf/qwen/qwen2.5-coder-32b-instruct",
            constructor_name: cf_qwen_qwen2_5_coder_32b_instruct,
            display_name: "Qwen2.5 Coder 32B Instruct",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        CfQwenQwen330bA3bFp8 {
            model_name: "@cf/qwen/qwen3-30b-a3b-fp8",
            constructor_name: cf_qwen_qwen3_30b_a3b_fp8,
            display_name: "Qwen3 30B A3b fp8",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfQwenQwen3827b {
            model_name: "@cf/qwen/qwen3.8-27b",
            constructor_name: cf_qwen_qwen3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfQwenQwq32b {
            model_name: "@cf/qwen/qwq-32b",
            constructor_name: cf_qwen_qwq_32b,
            display_name: "Qwq 32B",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        CfZaiOrgGlm47Flash {
            model_name: "@cf/zai-org/glm-4.7-flash",
            constructor_name: cf_zai_org_glm_4_7_flash,
            display_name: "GLM-4.7-Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfZaiOrgGlm52 {
            model_name: "@cf/zai-org/glm-5.2",
            constructor_name: cf_zai_org_glm_5_2,
            display_name: "Glm 5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfZaiOrgGlm53 {
            model_name: "@cf/zai-org/glm-5.3",
            constructor_name: cf_zai_org_glm_5_3,
            display_name: "Glm 5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CfZaiOrgGlm53Flash {
            model_name: "@cf/zai-org/glm-5.3-flash",
            constructor_name: cf_zai_org_glm_5_3_flash,
            display_name: "Glm 5.3 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
