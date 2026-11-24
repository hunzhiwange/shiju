//! Capabilities for edenai models.
//!
//! This module defines model types and their capabilities for edenai providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::edenai::Edenai;

model_capabilities! {
    provider: Edenai,
    models: {
        AmazonAmazonNovaLiteV10 {
            model_name: "amazon/amazon.nova-lite-v1:0",
            constructor_name: amazon_amazon_nova_lite_v1_0,
            display_name: "Nova Lite",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AmazonAmazonNovaLiteV10Us {
            model_name: "amazon/amazon.nova-lite-v1:0@us",
            constructor_name: amazon_amazon_nova_lite_v1_0_us,
            display_name: "Nova Lite (US)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AmazonAmazonNovaMicroV10 {
            model_name: "amazon/amazon.nova-micro-v1:0",
            constructor_name: amazon_amazon_nova_micro_v1_0,
            display_name: "Nova Micro",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonAmazonNovaMicroV10Us {
            model_name: "amazon/amazon.nova-micro-v1:0@us",
            constructor_name: amazon_amazon_nova_micro_v1_0_us,
            display_name: "Nova Micro (US)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonAmazonNovaProV10 {
            model_name: "amazon/amazon.nova-pro-v1:0",
            constructor_name: amazon_amazon_nova_pro_v1_0,
            display_name: "Nova Pro",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AmazonAmazonNovaProV10Us {
            model_name: "amazon/amazon.nova-pro-v1:0@us",
            constructor_name: amazon_amazon_nova_pro_v1_0_us,
            display_name: "Nova Pro (US)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AmazonGoogleGemma312bIt {
            model_name: "amazon/google.gemma-3-12b-it",
            constructor_name: amazon_google_gemma_3_12b_it,
            display_name: "Gemma 3 12B IT (Amazon Bedrock)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonGoogleGemma312bItUs {
            model_name: "amazon/google.gemma-3-12b-it@us",
            constructor_name: amazon_google_gemma_3_12b_it_us,
            display_name: "Gemma 3 12B IT (Amazon Bedrock, US)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonGoogleGemma327bIt {
            model_name: "amazon/google.gemma-3-27b-it",
            constructor_name: amazon_google_gemma_3_27b_it,
            display_name: "Gemma 3 27B IT (Amazon Bedrock)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonGoogleGemma327bItUs {
            model_name: "amazon/google.gemma-3-27b-it@us",
            constructor_name: amazon_google_gemma_3_27b_it_us,
            display_name: "Gemma 3 27B IT (Amazon Bedrock, US)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonGoogleGemma34bIt {
            model_name: "amazon/google.gemma-3-4b-it",
            constructor_name: amazon_google_gemma_3_4b_it,
            display_name: "Gemma 3 4B IT (Amazon Bedrock)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonGoogleGemma34bItUs {
            model_name: "amazon/google.gemma-3-4b-it@us",
            constructor_name: amazon_google_gemma_3_4b_it_us,
            display_name: "Gemma 3 4B IT (Amazon Bedrock, US)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonMistralPixtralLarge2502V10 {
            model_name: "amazon/mistral.pixtral-large-2502-v1:0",
            constructor_name: amazon_mistral_pixtral_large_2502_v1_0,
            display_name: "Pixtral Large (25.02) (Amazon Bedrock)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonMistralPixtralLarge2502V10Us {
            model_name: "amazon/mistral.pixtral-large-2502-v1:0@us",
            constructor_name: amazon_mistral_pixtral_large_2502_v1_0_us,
            display_name: "Pixtral Large (25.02) (Amazon Bedrock, US)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonMistralVoxtralMini3b2507 {
            model_name: "amazon/mistral.voxtral-mini-3b-2507",
            constructor_name: amazon_mistral_voxtral_mini_3b_2507,
            display_name: "Voxtral Mini 3B 2507 (Amazon Bedrock)",
            capabilities: [AudioInputSupport, ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        AmazonMistralVoxtralMini3b2507Us {
            model_name: "amazon/mistral.voxtral-mini-3b-2507@us",
            constructor_name: amazon_mistral_voxtral_mini_3b_2507_us,
            display_name: "Voxtral Mini 3B 2507 (Amazon Bedrock, US)",
            capabilities: [AudioInputSupport, ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        AmazonMistralVoxtralSmall24b2507 {
            model_name: "amazon/mistral.voxtral-small-24b-2507",
            constructor_name: amazon_mistral_voxtral_small_24b_2507,
            display_name: "Voxtral Small 24B 2507 (Amazon Bedrock)",
            capabilities: [AudioInputSupport, ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        AmazonMistralVoxtralSmall24b2507Us {
            model_name: "amazon/mistral.voxtral-small-24b-2507@us",
            constructor_name: amazon_mistral_voxtral_small_24b_2507_us,
            display_name: "Voxtral Small 24B 2507 (Amazon Bedrock, US)",
            capabilities: [AudioInputSupport, ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        AmazonMoonshotKimiK2Thinking {
            model_name: "amazon/moonshot.kimi-k2-thinking",
            constructor_name: amazon_moonshot_kimi_k2_thinking,
            display_name: "Kimi K2 Thinking (Amazon Bedrock)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonMoonshotaiKimiK25 {
            model_name: "amazon/moonshotai.kimi-k2.5",
            constructor_name: amazon_moonshotai_kimi_k2_5,
            display_name: "Kimi K2.5 (Amazon Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonOpenaiGptOssSafeguard20b {
            model_name: "amazon/openai.gpt-oss-safeguard-20b",
            constructor_name: amazon_openai_gpt_oss_safeguard_20b,
            display_name: "GPT OSS Safeguard 20B (Amazon Bedrock)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonOpenaiGptOssSafeguard20bUs {
            model_name: "amazon/openai.gpt-oss-safeguard-20b@us",
            constructor_name: amazon_openai_gpt_oss_safeguard_20b_us,
            display_name: "GPT OSS Safeguard 20B (Amazon Bedrock, US)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonQwenQwen3CoderNext {
            model_name: "amazon/qwen.qwen3-coder-next",
            constructor_name: amazon_qwen_qwen3_coder_next,
            display_name: "Qwen3 Coder Next (Amazon Bedrock)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonZaiGlm47Flash {
            model_name: "amazon/zai.glm-4.7-flash",
            constructor_name: amazon_zai_glm_4_7_flash,
            display_name: "GLM-4.7-Flash (Amazon Bedrock)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonZaiGlm47FlashUs {
            model_name: "amazon/zai.glm-4.7-flash@us",
            constructor_name: amazon_zai_glm_4_7_flash_us,
            display_name: "GLM-4.7-Flash (Amazon Bedrock, US)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeFable5 {
            model_name: "anthropic/claude-fable-5",
            constructor_name: anthropic_claude_fable_5,
            display_name: "Claude Fable 5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeFable51 {
            model_name: "anthropic/claude-fable-5-1",
            constructor_name: anthropic_claude_fable_5_1,
            display_name: "Claude Fable 5.1",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeFableLatest {
            model_name: "anthropic/claude-fable-latest",
            constructor_name: anthropic_claude_fable_latest,
            display_name: "Claude Fable Latest (Claude Fable 5.1)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeHaiku55 {
            model_name: "anthropic/claude-haiku-5-5",
            constructor_name: anthropic_claude_haiku_5_5,
            display_name: "Claude Haiku 5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeHaikuLatest {
            model_name: "anthropic/claude-haiku-latest",
            constructor_name: anthropic_claude_haiku_latest,
            display_name: "Claude Haiku Latest (Claude Haiku 5.5)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus45 {
            model_name: "anthropic/claude-opus-4-5",
            constructor_name: anthropic_claude_opus_4_5,
            display_name: "Claude Opus 4.5 (latest)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus4520251101 {
            model_name: "anthropic/claude-opus-4-5-20251101",
            constructor_name: anthropic_claude_opus_4_5_20251101,
            display_name: "Claude Opus 4.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus46 {
            model_name: "anthropic/claude-opus-4-6",
            constructor_name: anthropic_claude_opus_4_6,
            display_name: "Claude Opus 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus47 {
            model_name: "anthropic/claude-opus-4-7",
            constructor_name: anthropic_claude_opus_4_7,
            display_name: "Claude Opus 4.7",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus48 {
            model_name: "anthropic/claude-opus-4-8",
            constructor_name: anthropic_claude_opus_4_8,
            display_name: "Claude Opus 4.8",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus5 {
            model_name: "anthropic/claude-opus-5",
            constructor_name: anthropic_claude_opus_5,
            display_name: "Claude Opus 5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus55 {
            model_name: "anthropic/claude-opus-5-5",
            constructor_name: anthropic_claude_opus_5_5,
            display_name: "Claude Opus 5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpusLatest {
            model_name: "anthropic/claude-opus-latest",
            constructor_name: anthropic_claude_opus_latest,
            display_name: "Claude Opus Latest (Claude Opus 5.5)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet46 {
            model_name: "anthropic/claude-sonnet-4-6",
            constructor_name: anthropic_claude_sonnet_4_6,
            display_name: "Claude Sonnet 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet5 {
            model_name: "anthropic/claude-sonnet-5",
            constructor_name: anthropic_claude_sonnet_5,
            display_name: "Claude Sonnet 5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet55 {
            model_name: "anthropic/claude-sonnet-5-5",
            constructor_name: anthropic_claude_sonnet_5_5,
            display_name: "Claude Sonnet 5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnetLatest {
            model_name: "anthropic/claude-sonnet-latest",
            constructor_name: anthropic_claude_sonnet_latest,
            display_name: "Claude Sonnet Latest (Claude Sonnet 5.5)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt51Codex {
            model_name: "azure/gpt-5.1-codex",
            constructor_name: azure_gpt_5_1_codex,
            display_name: "GPT-5.1 Codex (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt51CodexMax {
            model_name: "azure/gpt-5.1-codex-max",
            constructor_name: azure_gpt_5_1_codex_max,
            display_name: "GPT-5.1 Codex Max (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt51CodexMini {
            model_name: "azure/gpt-5.1-codex-mini",
            constructor_name: azure_gpt_5_1_codex_mini,
            display_name: "GPT-5.1 Codex mini (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt52Codex {
            model_name: "azure/gpt-5.2-codex",
            constructor_name: azure_gpt_5_2_codex,
            display_name: "GPT-5.2 Codex (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CerebrasGptOss120b {
            model_name: "cerebras/gpt-oss-120b",
            constructor_name: cerebras_gpt_oss_120b,
            display_name: "GPT OSS 120B (Cerebras)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CloudflareCfAisingaporeGemmaSeaLionV427bIt {
            model_name: "cloudflare/@cf/aisingapore/gemma-sea-lion-v4-27b-it",
            constructor_name: cloudflare_cf_aisingapore_gemma_sea_lion_v4_27b_it,
            display_name: "Gemma-SEA-LION-v4-27B-IT (Cloudflare)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        CloudflareCfDeepseekAiDeepseekV4Flash0731 {
            model_name: "cloudflare/@cf/deepseek-ai/deepseek-v4-flash-0731",
            constructor_name: cloudflare_cf_deepseek_ai_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731 (Cloudflare)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CloudflareCfDeepseekAiDeepseekV4Pro0813 {
            model_name: "cloudflare/@cf/deepseek-ai/deepseek-v4-pro-0813",
            constructor_name: cloudflare_cf_deepseek_ai_deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813 (Cloudflare)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CloudflareCfDeepseekAiDeepseekV41Flash1017 {
            model_name: "cloudflare/@cf/deepseek-ai/deepseek-v4.1-flash-1017",
            constructor_name: cloudflare_cf_deepseek_ai_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (Cloudflare)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CloudflareCfMetaLlama321bInstruct {
            model_name: "cloudflare/@cf/meta/llama-3.2-1b-instruct",
            constructor_name: cloudflare_cf_meta_llama_3_2_1b_instruct,
            display_name: "Llama 3.2 1B Instruct (Cloudflare)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        CloudflareCfMetaLlama323bInstruct {
            model_name: "cloudflare/@cf/meta/llama-3.2-3b-instruct",
            constructor_name: cloudflare_cf_meta_llama_3_2_3b_instruct,
            display_name: "Llama 3.2 3B Instruct (Cloudflare)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        CloudflareCfMetaLlamaGuard38b {
            model_name: "cloudflare/@cf/meta/llama-guard-3-8b",
            constructor_name: cloudflare_cf_meta_llama_guard_3_8b,
            display_name: "Llama-Guard-3-8B (Cloudflare)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        CloudflareCfMistralaiMistralSmall3124bInstruct {
            model_name: "cloudflare/@cf/mistralai/mistral-small-3.1-24b-instruct",
            constructor_name: cloudflare_cf_mistralai_mistral_small_3_1_24b_instruct,
            display_name: "Mistral Small 3.1 24B (Cloudflare)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CloudflareCfOpenaiGptOss120b {
            model_name: "cloudflare/@cf/openai/gpt-oss-120b",
            constructor_name: cloudflare_cf_openai_gpt_oss_120b,
            display_name: "GPT OSS 120B (Cloudflare)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CloudflareCfOpenaiGptOss20b {
            model_name: "cloudflare/@cf/openai/gpt-oss-20b",
            constructor_name: cloudflare_cf_openai_gpt_oss_20b,
            display_name: "GPT OSS 20B (Cloudflare)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CloudflareCfQwenQwen25Coder32bInstruct {
            model_name: "cloudflare/@cf/qwen/qwen2.5-coder-32b-instruct",
            constructor_name: cloudflare_cf_qwen_qwen2_5_coder_32b_instruct,
            display_name: "Qwen2.5-Coder-32B-Instruct (Cloudflare)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        CloudflareCfZaiOrgGlm47Flash {
            model_name: "cloudflare/@cf/zai-org/glm-4.7-flash",
            constructor_name: cloudflare_cf_zai_org_glm_4_7_flash,
            display_name: "GLM-4.7-Flash (Cloudflare)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CohereC4aiAyaExpanse32b {
            model_name: "cohere/c4ai-aya-expanse-32b",
            constructor_name: cohere_c4ai_aya_expanse_32b,
            display_name: "Aya Expanse 32B",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        CohereCommandA032025 {
            model_name: "cohere/command-a-03-2025",
            constructor_name: cohere_command_a_03_2025,
            display_name: "Command A",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CohereCommandR082024 {
            model_name: "cohere/command-r-08-2024",
            constructor_name: cohere_command_r_08_2024,
            display_name: "Command R",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CohereCommandRPlus082024 {
            model_name: "cohere/command-r-plus-08-2024",
            constructor_name: cohere_command_r_plus_08_2024,
            display_name: "Command R+",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CohereCommandR7b122024 {
            model_name: "cohere/command-r7b-12-2024",
            constructor_name: cohere_command_r7b_12_2024,
            display_name: "Command R7B",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksDatabricksDeepseekV4Flash0731 {
            model_name: "databricks/databricks-deepseek-v4-flash-0731",
            constructor_name: databricks_databricks_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731 (Databricks)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksDatabricksDeepseekV4Pro0813 {
            model_name: "databricks/databricks-deepseek-v4-pro-0813",
            constructor_name: databricks_databricks_deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813 (Databricks)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksDatabricksGptOss120b {
            model_name: "databricks/databricks-gpt-oss-120b",
            constructor_name: databricks_databricks_gpt_oss_120b,
            display_name: "GPT OSS 120B (Databricks)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksDatabricksGptOss120bEu {
            model_name: "databricks/databricks-gpt-oss-120b@eu",
            constructor_name: databricks_databricks_gpt_oss_120b_eu,
            display_name: "GPT OSS 120B (Databricks, EU)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksDatabricksGptOss20b {
            model_name: "databricks/databricks-gpt-oss-20b",
            constructor_name: databricks_databricks_gpt_oss_20b,
            display_name: "GPT OSS 20B (Databricks)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksDatabricksGptOss20bEu {
            model_name: "databricks/databricks-gpt-oss-20b@eu",
            constructor_name: databricks_databricks_gpt_oss_20b_eu,
            display_name: "GPT OSS 20B (Databricks, EU)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksDatabricksInkling {
            model_name: "databricks/databricks-inkling",
            constructor_name: databricks_databricks_inkling,
            display_name: "Inkling (Databricks)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksDatabricksQwen3Next80bA3bInstruct {
            model_name: "databricks/databricks-qwen3-next-80b-a3b-instruct",
            constructor_name: databricks_databricks_qwen3_next_80b_a3b_instruct,
            display_name: "Qwen3-Next 80B-A3B Instruct (Databricks)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksDatabricksQwen3Next80bA3bInstructEu {
            model_name: "databricks/databricks-qwen3-next-80b-a3b-instruct@eu",
            constructor_name: databricks_databricks_qwen3_next_80b_a3b_instruct_eu,
            display_name: "Qwen3-Next 80B-A3B Instruct (Databricks, EU)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraBytedanceSeed20Code {
            model_name: "deepinfra/ByteDance/Seed-2.0-code",
            constructor_name: deepinfra_bytedance_seed_2_0_code,
            display_name: "Seed 2.0 Code (Deep Infra)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraBytedanceSeed20Mini {
            model_name: "deepinfra/ByteDance/Seed-2.0-mini",
            constructor_name: deepinfra_bytedance_seed_2_0_mini,
            display_name: "Seed 2.0 Mini (Deep Infra)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraGrypheMythomaxL213b {
            model_name: "deepinfra/Gryphe/MythoMax-L2-13b",
            constructor_name: deepinfra_gryphe_mythomax_l2_13b,
            display_name: "MythoMax L2 13B (Deep Infra)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraNousresearchHermes3Llama31405b {
            model_name: "deepinfra/NousResearch/Hermes-3-Llama-3.1-405B",
            constructor_name: deepinfra_nousresearch_hermes_3_llama_3_1_405b,
            display_name: "Hermes 3 405B Instruct (Deep Infra)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraNousresearchHermes3Llama3170b {
            model_name: "deepinfra/NousResearch/Hermes-3-Llama-3.1-70B",
            constructor_name: deepinfra_nousresearch_hermes_3_llama_3_1_70b,
            display_name: "Hermes 3 70B Instruct (Deep Infra)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        DeepinfraQwenQwen257bInstruct {
            model_name: "deepinfra/Qwen/Qwen2.5-7B-Instruct",
            constructor_name: deepinfra_qwen_qwen2_5_7b_instruct,
            display_name: "Qwen2.5 7B Instruct (Deep Infra)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        DeepinfraQwenQwen3235bA22bInstruct2507 {
            model_name: "deepinfra/Qwen/Qwen3-235B-A22B-Instruct-2507",
            constructor_name: deepinfra_qwen_qwen3_235b_a22b_instruct_2507,
            display_name: "Qwen3 235B-A22B Instruct 2507 (Deep Infra)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraQwenQwen3235bA22bThinking2507 {
            model_name: "deepinfra/Qwen/Qwen3-235B-A22B-Thinking-2507",
            constructor_name: deepinfra_qwen_qwen3_235b_a22b_thinking_2507,
            display_name: "Qwen3 235B-A22B Thinking 2507 (Deep Infra)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraQwenQwen3Coder480bA35bInstruct {
            model_name: "deepinfra/Qwen/Qwen3-Coder-480B-A35B-Instruct",
            constructor_name: deepinfra_qwen_qwen3_coder_480b_a35b_instruct,
            display_name: "Qwen3-Coder 480B-A35B Instruct (Deep Infra)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraQwenQwen3Max {
            model_name: "deepinfra/Qwen/Qwen3-Max",
            constructor_name: deepinfra_qwen_qwen3_max,
            display_name: "Qwen3 Max (Deep Infra)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraQwenQwen3Next80bA3bInstruct {
            model_name: "deepinfra/Qwen/Qwen3-Next-80B-A3B-Instruct",
            constructor_name: deepinfra_qwen_qwen3_next_80b_a3b_instruct,
            display_name: "Qwen3-Next 80B-A3B Instruct (Deep Infra)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraQwenQwen3Vl235bA22bInstruct {
            model_name: "deepinfra/Qwen/Qwen3-VL-235B-A22B-Instruct",
            constructor_name: deepinfra_qwen_qwen3_vl_235b_a22b_instruct,
            display_name: "Qwen3 VL 235B A22B Instruct (Deep Infra)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraQwenQwen3Vl30bA3bInstruct {
            model_name: "deepinfra/Qwen/Qwen3-VL-30B-A3B-Instruct",
            constructor_name: deepinfra_qwen_qwen3_vl_30b_a3b_instruct,
            display_name: "Qwen3 VL 30B A3B Instruct (Deep Infra)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraDeepseekAiDeepseekR1 {
            model_name: "deepinfra/deepseek-ai/DeepSeek-R1",
            constructor_name: deepinfra_deepseek_ai_deepseek_r1,
            display_name: "DeepSeek-R1 (Deep Infra)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraDeepseekAiDeepseekR10528 {
            model_name: "deepinfra/deepseek-ai/DeepSeek-R1-0528",
            constructor_name: deepinfra_deepseek_ai_deepseek_r1_0528,
            display_name: "DeepSeek R1 0528 (Deep Infra)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraDeepseekAiDeepseekV3 {
            model_name: "deepinfra/deepseek-ai/DeepSeek-V3",
            constructor_name: deepinfra_deepseek_ai_deepseek_v3,
            display_name: "DeepSeek-V3 (Deep Infra)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraDeepseekAiDeepseekV30324 {
            model_name: "deepinfra/deepseek-ai/DeepSeek-V3-0324",
            constructor_name: deepinfra_deepseek_ai_deepseek_v3_0324,
            display_name: "DeepSeek V3 0324 (Deep Infra)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraDeepseekAiDeepseekV4Flash0731 {
            model_name: "deepinfra/deepseek-ai/DeepSeek-V4-Flash-0731",
            constructor_name: deepinfra_deepseek_ai_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731 (Deep Infra)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraDeepseekAiDeepseekV4Pro0813 {
            model_name: "deepinfra/deepseek-ai/DeepSeek-V4-Pro-0813",
            constructor_name: deepinfra_deepseek_ai_deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813 (Deep Infra)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraDeepseekAiDeepseekV41Flash {
            model_name: "deepinfra/deepseek-ai/DeepSeek-V4.1-Flash",
            constructor_name: deepinfra_deepseek_ai_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (Deep Infra)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraDeepseekAiDeepseekV41Flash1017 {
            model_name: "deepinfra/deepseek-ai/DeepSeek-V4.1-Flash-1017",
            constructor_name: deepinfra_deepseek_ai_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (Deep Infra)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraGoogleGemma312bIt {
            model_name: "deepinfra/google/gemma-3-12b-it",
            constructor_name: deepinfra_google_gemma_3_12b_it,
            display_name: "Gemma 3 12B IT (Deep Infra)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraGoogleGemma327bIt {
            model_name: "deepinfra/google/gemma-3-27b-it",
            constructor_name: deepinfra_google_gemma_3_27b_it,
            display_name: "Gemma 3 27B IT (Deep Infra)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraGoogleGemma34bIt {
            model_name: "deepinfra/google/gemma-3-4b-it",
            constructor_name: deepinfra_google_gemma_3_4b_it,
            display_name: "Gemma 3 4B IT (Deep Infra)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraIbmGraniteGranite428b {
            model_name: "deepinfra/ibm-granite/granite-4.2-8b",
            constructor_name: deepinfra_ibm_granite_granite_4_2_8b,
            display_name: "Granite 4.2 8B (Deep Infra)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraMetaLlamaLlama3211bVisionInstruct {
            model_name: "deepinfra/meta-llama/Llama-3.2-11B-Vision-Instruct",
            constructor_name: deepinfra_meta_llama_llama_3_2_11b_vision_instruct,
            display_name: "Llama-3.2-11B-Vision-Instruct (Deep Infra)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        DeepinfraMetaLlamaLlama323bInstruct {
            model_name: "deepinfra/meta-llama/Llama-3.2-3B-Instruct",
            constructor_name: deepinfra_meta_llama_llama_3_2_3b_instruct,
            display_name: "Llama 3.2 3B Instruct (Deep Infra)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraMetaLlamaLlama3370bInstruct {
            model_name: "deepinfra/meta-llama/Llama-3.3-70B-Instruct",
            constructor_name: deepinfra_meta_llama_llama_3_3_70b_instruct,
            display_name: "Llama-3.3-70B-Instruct (Deep Infra)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraMetaLlamaLlamaGuard38b {
            model_name: "deepinfra/meta-llama/Llama-Guard-3-8B",
            constructor_name: deepinfra_meta_llama_llama_guard_3_8b,
            display_name: "Llama-Guard-3-8B (Deep Infra)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        DeepinfraMetaLlamaLlamaGuard412b {
            model_name: "deepinfra/meta-llama/Llama-Guard-4-12B",
            constructor_name: deepinfra_meta_llama_llama_guard_4_12b,
            display_name: "Llama Guard 4 12B (Deep Infra)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        DeepinfraMetaModelsMuseGlimmer30b {
            model_name: "deepinfra/meta-models/Muse-Glimmer-30B",
            constructor_name: deepinfra_meta_models_muse_glimmer_30b,
            display_name: "Muse Glimmer 30B (Deep Infra)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraMicrosoftWizardlm28x22b {
            model_name: "deepinfra/microsoft/WizardLM-2-8x22B",
            constructor_name: deepinfra_microsoft_wizardlm_2_8x22b,
            display_name: "WizardLM 2 8x22B (Deep Infra)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        DeepinfraMistralaiMistralSmall24bInstruct2501 {
            model_name: "deepinfra/mistralai/Mistral-Small-24B-Instruct-2501",
            constructor_name: deepinfra_mistralai_mistral_small_24b_instruct_2501,
            display_name: "Mistral Small 3 (Deep Infra)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        DeepinfraMistralaiMixtral8x7bInstructV01 {
            model_name: "deepinfra/mistralai/Mixtral-8x7B-Instruct-v0.1",
            constructor_name: deepinfra_mistralai_mixtral_8x7b_instruct_v0_1,
            display_name: "Mixtral 8x7B Instruct v0.1 (Deep Infra)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraMoonshotaiKimiK2Instruct {
            model_name: "deepinfra/moonshotai/Kimi-K2-Instruct",
            constructor_name: deepinfra_moonshotai_kimi_k2_instruct,
            display_name: "Kimi K2 Instruct (Deep Infra)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraMoonshotaiKimiK25 {
            model_name: "deepinfra/moonshotai/Kimi-K2.5",
            constructor_name: deepinfra_moonshotai_kimi_k2_5,
            display_name: "Kimi K2.5 (Deep Infra)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraNemotron3Super120bA12b {
            model_name: "deepinfra/nemotron-3-super-120b-a12b",
            constructor_name: deepinfra_nemotron_3_super_120b_a12b,
            display_name: "Nemotron 3 Super 120B A12B (Deep Infra)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraNemotron3Ultra550bA55b {
            model_name: "deepinfra/nemotron-3-ultra-550b-a55b",
            constructor_name: deepinfra_nemotron_3_ultra_550b_a55b,
            display_name: "Nemotron 3 Ultra 550B A55B (Deep Infra)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraNvidiaLlama31Nemotron70bInstruct {
            model_name: "deepinfra/nvidia/Llama-3.1-Nemotron-70B-Instruct",
            constructor_name: deepinfra_nvidia_llama_3_1_nemotron_70b_instruct,
            display_name: "Llama 3.1 Nemotron 70B Instruct (Deep Infra)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraNvidiaNemotron3Nano30bA3b {
            model_name: "deepinfra/nvidia/Nemotron-3-Nano-30B-A3B",
            constructor_name: deepinfra_nvidia_nemotron_3_nano_30b_a3b,
            display_name: "Nemotron 3 Nano 30B A3B (Deep Infra)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraOpenaiGptOss120b {
            model_name: "deepinfra/openai/gpt-oss-120b",
            constructor_name: deepinfra_openai_gpt_oss_120b,
            display_name: "GPT OSS 120B (Deep Infra)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraOpenaiGptOss20b {
            model_name: "deepinfra/openai/gpt-oss-20b",
            constructor_name: deepinfra_openai_gpt_oss_20b,
            display_name: "GPT OSS 20B (Deep Infra)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraStepfunAiStep35Flash {
            model_name: "deepinfra/stepfun-ai/Step-3.5-Flash",
            constructor_name: deepinfra_stepfun_ai_step_3_5_flash,
            display_name: "Step 3.5 Flash (Deep Infra)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraStepfunAiStep37Flash {
            model_name: "deepinfra/stepfun-ai/Step-3.7-Flash",
            constructor_name: deepinfra_stepfun_ai_step_3_7_flash,
            display_name: "Step 3.7 Flash (Deep Infra)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DeepinfraStepfunAiStep5Preview {
            model_name: "deepinfra/stepfun-ai/Step-5-Preview",
            constructor_name: deepinfra_stepfun_ai_step_5_preview,
            display_name: "Step 5 Preview (Deep Infra)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DeepinfraTencentHy3 {
            model_name: "deepinfra/tencent/Hy3",
            constructor_name: deepinfra_tencent_hy3,
            display_name: "Hy3 (Deep Infra)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraTencentHy4Preview {
            model_name: "deepinfra/tencent/Hy4-preview",
            constructor_name: deepinfra_tencent_hy4_preview,
            display_name: "Hy4 preview (Deep Infra)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraThinkingmachinesInkling {
            model_name: "deepinfra/thinkingmachines/Inkling",
            constructor_name: deepinfra_thinkingmachines_inkling,
            display_name: "Inkling (Deep Infra)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraThinkingmachinesInklingSmall {
            model_name: "deepinfra/thinkingmachines/Inkling-Small",
            constructor_name: deepinfra_thinkingmachines_inkling_small,
            display_name: "Inkling Small (Deep Infra)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraZaiOrgGlm47Flash {
            model_name: "deepinfra/zai-org/GLM-4.7-Flash",
            constructor_name: deepinfra_zai_org_glm_4_7_flash,
            display_name: "GLM-4.7-Flash (Deep Infra)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV4FlashVisionExp {
            model_name: "deepseek-v4-flash-vision-exp",
            constructor_name: deepseek_v4_flash_vision_exp,
            display_name: "DeepSeek V4 Flash Vision Exp",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekChat {
            model_name: "deepseek/deepseek-chat",
            constructor_name: deepseek_deepseek_chat,
            display_name: "DeepSeek Chat",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV4Flash {
            model_name: "deepseek/deepseek-v4-flash",
            constructor_name: deepseek_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV4FlashVisionExp {
            model_name: "deepseek/deepseek-v4-flash-vision-exp",
            constructor_name: deepseek_deepseek_v4_flash_vision_exp,
            display_name: "DeepSeek V4 Flash Vision Exp",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV4Pro {
            model_name: "deepseek/deepseek-v4-pro",
            constructor_name: deepseek_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FireworksAiAccountsFireworksModelsEmber1 {
            model_name: "fireworks_ai/accounts/fireworks/models/ember-1",
            constructor_name: fireworks_ai_accounts_fireworks_models_ember_1,
            display_name: "Ember-1 (Fireworks AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FireworksAiAccountsFireworksModelsGptOss120b {
            model_name: "fireworks_ai/accounts/fireworks/models/gpt-oss-120b",
            constructor_name: fireworks_ai_accounts_fireworks_models_gpt_oss_120b,
            display_name: "GPT OSS 120B (Fireworks AI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FireworksAiAccountsFireworksModelsInkling {
            model_name: "fireworks_ai/accounts/fireworks/models/inkling",
            constructor_name: fireworks_ai_accounts_fireworks_models_inkling,
            display_name: "Inkling (Fireworks AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FireworksAiAccountsFireworksModelsMuseSparkOpen {
            model_name: "fireworks_ai/accounts/fireworks/models/muse-spark-open",
            constructor_name: fireworks_ai_accounts_fireworks_models_muse_spark_open,
            display_name: "Muse Spark Open (Fireworks AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        FireworksAiGptOss120b {
            model_name: "fireworks_ai/gpt-oss-120b",
            constructor_name: fireworks_ai_gpt_oss_120b,
            display_name: "GPT OSS 120B (Fireworks AI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FlexaiDeepseekV4Flash0731 {
            model_name: "flexai/DeepSeek-V4-Flash-0731",
            constructor_name: flexai_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731 (FlexAI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FlexaiDeepseekV4Flash0731Eu {
            model_name: "flexai/DeepSeek-V4-Flash-0731@eu",
            constructor_name: flexai_deepseek_v4_flash_0731_eu,
            display_name: "DeepSeek V4 Flash 0731 (FlexAI, EU)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FlexaiDeepseekV41Flash {
            model_name: "flexai/DeepSeek-V4.1-Flash",
            constructor_name: flexai_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (FlexAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FlexaiDeepseekV41Flash1017 {
            model_name: "flexai/DeepSeek-V4.1-Flash-1017",
            constructor_name: flexai_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (FlexAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FlexaiMuseGlimmer30b {
            model_name: "flexai/Muse-Glimmer-30B",
            constructor_name: flexai_muse_glimmer_30b,
            display_name: "Muse Glimmer 30B (FlexAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FlexaiStep37Flash {
            model_name: "flexai/Step-3.7-Flash",
            constructor_name: flexai_step_3_7_flash,
            display_name: "Step 3.7 Flash (FlexAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FlexaiGptOss120b {
            model_name: "flexai/gpt-oss-120b",
            constructor_name: flexai_gpt_oss_120b,
            display_name: "GPT OSS 120B (FlexAI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FlexaiGptOss20b {
            model_name: "flexai/gpt-oss-20b",
            constructor_name: flexai_gpt_oss_20b,
            display_name: "GPT OSS 20B (FlexAI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleDeepResearchMaxPreview042026 {
            model_name: "google/deep-research-max-preview-04-2026",
            constructor_name: google_deep_research_max_preview_04_2026,
            display_name: "Deep Research Max Preview",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        GoogleDeepResearchPreview042026 {
            model_name: "google/deep-research-preview-04-2026",
            constructor_name: google_deep_research_preview_04_2026,
            display_name: "Gemini Deep Research Preview",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        GoogleGemini25FlashImage {
            model_name: "google/gemini-2.5-flash-image",
            constructor_name: google_gemini_2_5_flash_image,
            display_name: "Nano Banana",
            capabilities: [AudioInputSupport, ImageInputSupport, ImageOutputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, VideoInputSupport]
        },
        GoogleGemini3FlashPreview {
            model_name: "google/gemini-3-flash-preview",
            constructor_name: google_gemini_3_flash_preview,
            display_name: "Gemini 3 Flash Preview",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini3ProImage {
            model_name: "google/gemini-3-pro-image",
            constructor_name: google_gemini_3_pro_image,
            display_name: "Nano Banana Pro",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        GoogleGemini3ProImagePreview {
            model_name: "google/gemini-3-pro-image-preview",
            constructor_name: google_gemini_3_pro_image_preview,
            display_name: "Nano Banana Pro Preview",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        GoogleGemini31FlashImage {
            model_name: "google/gemini-3.1-flash-image",
            constructor_name: google_gemini_3_1_flash_image,
            display_name: "Nano Banana 2",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        GoogleGemini31FlashImagePreview {
            model_name: "google/gemini-3.1-flash-image-preview",
            constructor_name: google_gemini_3_1_flash_image_preview,
            display_name: "Nano Banana 2 Preview",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        GoogleGemini31FlashLite {
            model_name: "google/gemini-3.1-flash-lite",
            constructor_name: google_gemini_3_1_flash_lite,
            display_name: "Gemini 3.1 Flash Lite",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini31FlashLiteImage {
            model_name: "google/gemini-3.1-flash-lite-image",
            constructor_name: google_gemini_3_1_flash_lite_image,
            display_name: "Nano Banana 2 Lite",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        GoogleGemini31FlashLitePreview {
            model_name: "google/gemini-3.1-flash-lite-preview",
            constructor_name: google_gemini_3_1_flash_lite_preview,
            display_name: "Gemini 3.1 Flash Lite Preview",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini31ProPreview {
            model_name: "google/gemini-3.1-pro-preview",
            constructor_name: google_gemini_3_1_pro_preview,
            display_name: "Gemini 3.1 Pro Preview",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini31ProPreviewCustomtools {
            model_name: "google/gemini-3.1-pro-preview-customtools",
            constructor_name: google_gemini_3_1_pro_preview_customtools,
            display_name: "Gemini 3.1 Pro Preview Custom Tools",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini35Flash {
            model_name: "google/gemini-3.5-flash",
            constructor_name: google_gemini_3_5_flash,
            display_name: "Gemini 3.5 Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini35FlashLite {
            model_name: "google/gemini-3.5-flash-lite",
            constructor_name: google_gemini_3_5_flash_lite,
            display_name: "Gemini 3.5 Flash Lite",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini36Flash {
            model_name: "google/gemini-3.6-flash",
            constructor_name: google_gemini_3_6_flash,
            display_name: "Gemini 3.6 Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini37Flash {
            model_name: "google/gemini-3.7-flash",
            constructor_name: google_gemini_3_7_flash,
            display_name: "Gemini 3.7 Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini38Flash {
            model_name: "google/gemini-3.8-flash",
            constructor_name: google_gemini_3_8_flash,
            display_name: "Gemini 3.8 Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini4Argon {
            model_name: "google/gemini-4-argon",
            constructor_name: google_gemini_4_argon,
            display_name: "Gemini 4 Argon",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGeminiFlashLatest {
            model_name: "google/gemini-flash-latest",
            constructor_name: google_gemini_flash_latest,
            display_name: "Gemini Flash Latest (Gemini 3.8 Flash)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGeminiNanoBanana21 {
            model_name: "google/gemini-nano-banana-2.1",
            constructor_name: google_gemini_nano_banana_2_1,
            display_name: "Nano Banana 2.1",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGeminiProLatest {
            model_name: "google/gemini-pro-latest",
            constructor_name: google_gemini_pro_latest,
            display_name: "Gemini Pro Latest (Gemini 4 Argon)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGeminiRoboticsEr2Preview {
            model_name: "google/gemini-robotics-er-2-preview",
            constructor_name: google_gemini_robotics_er_2_preview,
            display_name: "Gemini Robotics-ER 2 Preview",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GreenferenceDeepseekV41Flash {
            model_name: "greenference/deepseek-v4.1-flash",
            constructor_name: greenference_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (Greenference)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GreenferenceGptOss20b {
            model_name: "greenference/gpt-oss-20b",
            constructor_name: greenference_gpt_oss_20b,
            display_name: "GPT OSS 20B (Greenference)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        GreenptDeepseekV4Flash0731 {
            model_name: "greenpt/deepseek-v4-flash-0731",
            constructor_name: greenpt_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731 (Greenpt)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GreenptDeepseekV41Flash {
            model_name: "greenpt/deepseek-v4.1-flash",
            constructor_name: greenpt_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (Greenpt)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GreenptGptOss120b {
            model_name: "greenpt/gpt-oss-120b",
            constructor_name: greenpt_gpt_oss_120b,
            display_name: "GPT OSS 120B (Greenpt)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GreenptLlama3370bInstruct {
            model_name: "greenpt/llama-3.3-70b-instruct",
            constructor_name: greenpt_llama_3_3_70b_instruct,
            display_name: "Llama-3.3-70B-Instruct (Greenpt)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GreenptQwen3235bA22bInstruct2507 {
            model_name: "greenpt/qwen3-235b-a22b-instruct-2507",
            constructor_name: greenpt_qwen3_235b_a22b_instruct_2507,
            display_name: "Qwen3 235B-A22B Instruct 2507 (Greenpt)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GroqMetaLlamaLlamaPromptGuard222m {
            model_name: "groq/meta-llama/llama-prompt-guard-2-22m",
            constructor_name: groq_meta_llama_llama_prompt_guard_2_22m,
            display_name: "Llama Prompt Guard 2 22M (Groq)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        GroqMetaLlamaLlamaPromptGuard286m {
            model_name: "groq/meta-llama/llama-prompt-guard-2-86m",
            constructor_name: groq_meta_llama_llama_prompt_guard_2_86m,
            display_name: "Llama Prompt Guard 2 86M (Groq)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        GroqOpenaiGptOss120b {
            model_name: "groq/openai/gpt-oss-120b",
            constructor_name: groq_openai_gpt_oss_120b,
            display_name: "GPT OSS 120B (Groq)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GroqOpenaiGptOss20b {
            model_name: "groq/openai/gpt-oss-20b",
            constructor_name: groq_openai_gpt_oss_20b,
            display_name: "GPT OSS 20B (Groq)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GroqOpenaiGptOssSafeguard20b {
            model_name: "groq/openai/gpt-oss-safeguard-20b",
            constructor_name: groq_openai_gpt_oss_safeguard_20b,
            display_name: "GPT OSS Safeguard 20B (Groq)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        InfomaniakMistralaiMinistral314bInstruct2512 {
            model_name: "infomaniak/mistralai/Ministral-3-14B-Instruct-2512",
            constructor_name: infomaniak_mistralai_ministral_3_14b_instruct_2512,
            display_name: "Ministral 3 14B (Infomaniak)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        IonosQwenQwen3CoderNext {
            model_name: "ionos/Qwen/Qwen3-Coder-Next",
            constructor_name: ionos_qwen_qwen3_coder_next,
            display_name: "Qwen3 Coder Next (IONOS)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        IonosMetaLlamaLlama3370bInstruct {
            model_name: "ionos/meta-llama/Llama-3.3-70B-Instruct",
            constructor_name: ionos_meta_llama_llama_3_3_70b_instruct,
            display_name: "Llama-3.3-70B-Instruct (IONOS)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        IonosOpenaiGptOss120b {
            model_name: "ionos/openai/gpt-oss-120b",
            constructor_name: ionos_openai_gpt_oss_120b,
            display_name: "GPT OSS 120B (IONOS)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaMuseSpark11 {
            model_name: "meta/muse-spark-1.1",
            constructor_name: meta_muse_spark_1_1,
            display_name: "Muse Spark 1.1 (Meta)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaMuseSpark12 {
            model_name: "meta/muse-spark-1.2",
            constructor_name: meta_muse_spark_1_2,
            display_name: "Muse Spark 1.2 (Meta)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaMuseSpark13 {
            model_name: "meta/muse-spark-1.3",
            constructor_name: meta_muse_spark_1_3,
            display_name: "Muse Spark 1.3 (Meta)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaMuseWonder10 {
            model_name: "meta/muse-wonder-1.0",
            constructor_name: meta_muse_wonder_1_0,
            display_name: "Muse Wonder (Meta)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM2 {
            model_name: "minimax/MiniMax-M2",
            constructor_name: minimax_minimax_m2,
            display_name: "MiniMax-M2",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM21 {
            model_name: "minimax/MiniMax-M2.1",
            constructor_name: minimax_minimax_m2_1,
            display_name: "MiniMax-M2.1",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM25 {
            model_name: "minimax/MiniMax-M2.5",
            constructor_name: minimax_minimax_m2_5,
            display_name: "MiniMax-M2.5",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM27 {
            model_name: "minimax/MiniMax-M2.7",
            constructor_name: minimax_minimax_m2_7,
            display_name: "MiniMax-M2.7",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM3 {
            model_name: "minimax/MiniMax-M3",
            constructor_name: minimax_minimax_m3,
            display_name: "MiniMax-M3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MinimaxMinimaxM31Flash {
            model_name: "minimax/MiniMax-M3.1-Flash",
            constructor_name: minimax_minimax_m3_1_flash,
            display_name: "MiniMax-M3.1-Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MistralCodestral2508 {
            model_name: "mistral/codestral-2508",
            constructor_name: mistral_codestral_2508,
            display_name: "Codestral 25.08",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralCodestralLatest {
            model_name: "mistral/codestral-latest",
            constructor_name: mistral_codestral_latest,
            display_name: "Codestral (latest)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralDevstral2512 {
            model_name: "mistral/devstral-2512",
            constructor_name: mistral_devstral_2512,
            display_name: "Devstral 2",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralDevstralMediumLatest {
            model_name: "mistral/devstral-medium-latest",
            constructor_name: mistral_devstral_medium_latest,
            display_name: "Devstral 2 (latest)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMagistralMediumLatest {
            model_name: "mistral/magistral-medium-latest",
            constructor_name: mistral_magistral_medium_latest,
            display_name: "Magistral Medium (latest)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMinistral14b2512 {
            model_name: "mistral/ministral-14b-2512",
            constructor_name: mistral_ministral_14b_2512,
            display_name: "Ministral 3 14B 2512",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMinistral3b2512 {
            model_name: "mistral/ministral-3b-2512",
            constructor_name: mistral_ministral_3b_2512,
            display_name: "Ministral 3 3B 2512",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMinistral8b2512 {
            model_name: "mistral/ministral-8b-2512",
            constructor_name: mistral_ministral_8b_2512,
            display_name: "Ministral 3 8B 2512",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMistralLarge2407 {
            model_name: "mistral/mistral-large-2407",
            constructor_name: mistral_mistral_large_2407,
            display_name: "Mistral Large 2407",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMistralLarge2512 {
            model_name: "mistral/mistral-large-2512",
            constructor_name: mistral_mistral_large_2512,
            display_name: "Mistral Large 3",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMistralLargeLatest {
            model_name: "mistral/mistral-large-latest",
            constructor_name: mistral_mistral_large_latest,
            display_name: "Mistral Large (latest)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMistralMedium2505 {
            model_name: "mistral/mistral-medium-2505",
            constructor_name: mistral_mistral_medium_2505,
            display_name: "Mistral Medium 3",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMistralMedium2604 {
            model_name: "mistral/mistral-medium-2604",
            constructor_name: mistral_mistral_medium_2604,
            display_name: "Mistral Medium 3.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMistralMedium3 {
            model_name: "mistral/mistral-medium-3",
            constructor_name: mistral_mistral_medium_3,
            display_name: "Mistral Medium 3",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMistralMedium35 {
            model_name: "mistral/mistral-medium-3-5",
            constructor_name: mistral_mistral_medium_3_5,
            display_name: "Mistral Medium 3.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMistralMediumLatest {
            model_name: "mistral/mistral-medium-latest",
            constructor_name: mistral_mistral_medium_latest,
            display_name: "Mistral Medium (latest)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMistralSmall2603 {
            model_name: "mistral/mistral-small-2603",
            constructor_name: mistral_mistral_small_2603,
            display_name: "Mistral Small 4",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMistralSmallLatest {
            model_name: "mistral/mistral-small-latest",
            constructor_name: mistral_mistral_small_latest,
            display_name: "Mistral Small (latest)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralVoxtralSmallLatest {
            model_name: "mistral/voxtral-small-latest",
            constructor_name: mistral_voxtral_small_latest,
            display_name: "Voxtral Small (latest)",
            capabilities: [AudioInputSupport, ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotKimiK26 {
            model_name: "moonshot/kimi-k2.6",
            constructor_name: moonshot_kimi_k2_6,
            display_name: "Kimi K2.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MoonshotKimiK27Code {
            model_name: "moonshot/kimi-k2.7-code",
            constructor_name: moonshot_kimi_k2_7_code,
            display_name: "Kimi K2.7 Code",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MoonshotKimiK27CodeHighspeed {
            model_name: "moonshot/kimi-k2.7-code-highspeed",
            constructor_name: moonshot_kimi_k2_7_code_highspeed,
            display_name: "Kimi K2.7 Code Highspeed",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, VideoInputSupport]
        },
        MoonshotKimiK3 {
            model_name: "moonshot/kimi-k3",
            constructor_name: moonshot_kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NebiusQwenQwen3235bA22bInstruct2507 {
            model_name: "nebius/Qwen/Qwen3-235B-A22B-Instruct-2507",
            constructor_name: nebius_qwen_qwen3_235b_a22b_instruct_2507,
            display_name: "Qwen3 235B-A22B Instruct 2507 (Nebius)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NebiusQwenQwen330bA3bInstruct2507 {
            model_name: "nebius/Qwen/Qwen3-30B-A3B-Instruct-2507",
            constructor_name: nebius_qwen_qwen3_30b_a3b_instruct_2507,
            display_name: "Qwen3 30B-A3B Instruct 2507 (Nebius)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NebiusDeepseekAiDeepseekV4Flash0731 {
            model_name: "nebius/deepseek-ai/DeepSeek-V4-Flash-0731",
            constructor_name: nebius_deepseek_ai_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731 (Nebius)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NebiusDeepseekAiDeepseekV4Pro0813 {
            model_name: "nebius/deepseek-ai/DeepSeek-V4-Pro-0813",
            constructor_name: nebius_deepseek_ai_deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813 (Nebius)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NebiusDeepseekAiDeepseekV41Flash {
            model_name: "nebius/deepseek-ai/DeepSeek-V4.1-Flash",
            constructor_name: nebius_deepseek_ai_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (Nebius)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NebiusDeepseekAiDeepseekV41Flash1017 {
            model_name: "nebius/deepseek-ai/DeepSeek-V4.1-Flash-1017",
            constructor_name: nebius_deepseek_ai_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (Nebius)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NebiusGoogleGemma327bIt {
            model_name: "nebius/google/gemma-3-27b-it",
            constructor_name: nebius_google_gemma_3_27b_it,
            display_name: "Gemma 3 27B IT (Nebius)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NebiusMetaModelsMuseSparkOpen {
            model_name: "nebius/meta-models/muse-spark-open",
            constructor_name: nebius_meta_models_muse_spark_open,
            display_name: "Muse Spark Open (Nebius)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NebiusNvidiaNemotron3Ultra550bA55b {
            model_name: "nebius/nvidia/Nemotron-3-Ultra-550b-a55b",
            constructor_name: nebius_nvidia_nemotron_3_ultra_550b_a55b,
            display_name: "Nemotron 3 Ultra 550B A55B (Nebius)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NebiusNvidiaNemotron3Super120bA12b {
            model_name: "nebius/nvidia/nemotron-3-super-120b-a12b",
            constructor_name: nebius_nvidia_nemotron_3_super_120b_a12b,
            display_name: "Nemotron 3 Super 120B A12B (Nebius)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NebiusOpenaiGptOss120b {
            model_name: "nebius/openai/gpt-oss-120b",
            constructor_name: nebius_openai_gpt_oss_120b,
            display_name: "GPT OSS 120B (Nebius)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt35Turbo {
            model_name: "openai/gpt-3.5-turbo",
            constructor_name: openai_gpt_3_5_turbo,
            display_name: "GPT-3.5-turbo",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4 {
            model_name: "openai/gpt-4",
            constructor_name: openai_gpt_4,
            display_name: "GPT-4",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4Turbo {
            model_name: "openai/gpt-4-turbo",
            constructor_name: openai_gpt_4_turbo,
            display_name: "GPT-4 Turbo",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt41 {
            model_name: "openai/gpt-4.1",
            constructor_name: openai_gpt_4_1,
            display_name: "GPT-4.1",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt41Mini {
            model_name: "openai/gpt-4.1-mini",
            constructor_name: openai_gpt_4_1_mini,
            display_name: "GPT-4.1 mini",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt41Nano {
            model_name: "openai/gpt-4.1-nano",
            constructor_name: openai_gpt_4_1_nano,
            display_name: "GPT-4.1 nano",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4o {
            model_name: "openai/gpt-4o",
            constructor_name: openai_gpt_4o,
            display_name: "GPT-4o",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4o20240806 {
            model_name: "openai/gpt-4o-2024-08-06",
            constructor_name: openai_gpt_4o_2024_08_06,
            display_name: "GPT-4o (2024-08-06)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4o20241120 {
            model_name: "openai/gpt-4o-2024-11-20",
            constructor_name: openai_gpt_4o_2024_11_20,
            display_name: "GPT-4o (2024-11-20)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4oMini {
            model_name: "openai/gpt-4o-mini",
            constructor_name: openai_gpt_4o_mini,
            display_name: "GPT-4o mini",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5 {
            model_name: "openai/gpt-5",
            constructor_name: openai_gpt_5,
            display_name: "GPT-5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5Mini {
            model_name: "openai/gpt-5-mini",
            constructor_name: openai_gpt_5_mini,
            display_name: "GPT-5 Mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5Nano {
            model_name: "openai/gpt-5-nano",
            constructor_name: openai_gpt_5_nano,
            display_name: "GPT-5 Nano",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5Pro {
            model_name: "openai/gpt-5-pro",
            constructor_name: openai_gpt_5_pro,
            display_name: "GPT-5 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5SearchApi {
            model_name: "openai/gpt-5-search-api",
            constructor_name: openai_gpt_5_search_api,
            display_name: "GPT-5 Search API",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt51 {
            model_name: "openai/gpt-5.1",
            constructor_name: openai_gpt_5_1,
            display_name: "GPT-5.1",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt52 {
            model_name: "openai/gpt-5.2",
            constructor_name: openai_gpt_5_2,
            display_name: "GPT-5.2",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt52Pro {
            model_name: "openai/gpt-5.2-pro",
            constructor_name: openai_gpt_5_2_pro,
            display_name: "GPT-5.2 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt53Codex {
            model_name: "openai/gpt-5.3-codex",
            constructor_name: openai_gpt_5_3_codex,
            display_name: "GPT-5.3 Codex",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54 {
            model_name: "openai/gpt-5.4",
            constructor_name: openai_gpt_5_4,
            display_name: "GPT-5.4",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Mini {
            model_name: "openai/gpt-5.4-mini",
            constructor_name: openai_gpt_5_4_mini,
            display_name: "GPT-5.4 mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Nano {
            model_name: "openai/gpt-5.4-nano",
            constructor_name: openai_gpt_5_4_nano,
            display_name: "GPT-5.4 nano",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Pro {
            model_name: "openai/gpt-5.4-pro",
            constructor_name: openai_gpt_5_4_pro,
            display_name: "GPT-5.4 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt55 {
            model_name: "openai/gpt-5.5",
            constructor_name: openai_gpt_5_5,
            display_name: "GPT-5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt55Pro {
            model_name: "openai/gpt-5.5-pro",
            constructor_name: openai_gpt_5_5_pro,
            display_name: "GPT-5.5 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Luna {
            model_name: "openai/gpt-5.6-luna",
            constructor_name: openai_gpt_5_6_luna,
            display_name: "GPT-5.6 Luna",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Sol {
            model_name: "openai/gpt-5.6-sol",
            constructor_name: openai_gpt_5_6_sol,
            display_name: "GPT-5.6 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Terra {
            model_name: "openai/gpt-5.6-terra",
            constructor_name: openai_gpt_5_6_terra,
            display_name: "GPT-5.6 Terra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Astra {
            model_name: "openai/gpt-6-astra",
            constructor_name: openai_gpt_6_astra,
            display_name: "GPT-6 Astra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Luna {
            model_name: "openai/gpt-6-luna",
            constructor_name: openai_gpt_6_luna,
            display_name: "GPT-6 Luna",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Sol {
            model_name: "openai/gpt-6-sol",
            constructor_name: openai_gpt_6_sol,
            display_name: "GPT-6 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt61Sol {
            model_name: "openai/gpt-6.1-sol",
            constructor_name: openai_gpt_6_1_sol,
            display_name: "GPT-6.1 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptLatest {
            model_name: "openai/gpt-latest",
            constructor_name: openai_gpt_latest,
            display_name: "GPT Latest (GPT-6 Astra)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptMiniLatest {
            model_name: "openai/gpt-mini-latest",
            constructor_name: openai_gpt_mini_latest,
            display_name: "GPT Mini Latest (GPT-5.6 Luna)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptProLatest {
            model_name: "openai/gpt-pro-latest",
            constructor_name: openai_gpt_pro_latest,
            display_name: "GPT Pro Latest (GPT-5.6 Sol)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiO1 {
            model_name: "openai/o1",
            constructor_name: openai_o1,
            display_name: "o1",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiO1Pro {
            model_name: "openai/o1-pro",
            constructor_name: openai_o1_pro,
            display_name: "o1-pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiO3 {
            model_name: "openai/o3",
            constructor_name: openai_o3,
            display_name: "o3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiO3Mini {
            model_name: "openai/o3-mini",
            constructor_name: openai_o3_mini,
            display_name: "o3-mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiO3Pro {
            model_name: "openai/o3-pro",
            constructor_name: openai_o3_pro,
            display_name: "o3-pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiO4Mini {
            model_name: "openai/o4-mini",
            constructor_name: openai_o4_mini,
            display_name: "o4-mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OvhcloudGptOss120b {
            model_name: "ovhcloud/gpt-oss-120b",
            constructor_name: ovhcloud_gpt_oss_120b,
            display_name: "GPT OSS 120B (OVHcloud)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OvhcloudGptOss20b {
            model_name: "ovhcloud/gpt-oss-20b",
            constructor_name: ovhcloud_gpt_oss_20b,
            display_name: "GPT OSS 20B (OVHcloud)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        PerplexityaiSonar {
            model_name: "perplexityai/sonar",
            constructor_name: perplexityai_sonar,
            display_name: "Sonar",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        PerplexityaiSonarDeepResearch {
            model_name: "perplexityai/sonar-deep-research",
            constructor_name: perplexityai_sonar_deep_research,
            display_name: "Sonar Deep Research",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        PerplexityaiSonarPro {
            model_name: "perplexityai/sonar-pro",
            constructor_name: perplexityai_sonar_pro,
            display_name: "Sonar Pro",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        PerplexityaiSonarReasoningPro {
            model_name: "perplexityai/sonar-reasoning-pro",
            constructor_name: perplexityai_sonar_reasoning_pro,
            display_name: "Sonar Reasoning Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        QwenDeepseekV4Flash0731 {
            model_name: "qwen/deepseek-v4-flash-0731",
            constructor_name: qwen_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731 (Alibaba)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenDeepseekV4Pro0813 {
            model_name: "qwen/deepseek-v4-pro-0813",
            constructor_name: qwen_deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813 (Alibaba)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenDeepseekV41Flash {
            model_name: "qwen/deepseek-v4.1-flash",
            constructor_name: qwen_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (Alibaba)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenDeepseekV41Flash1017 {
            model_name: "qwen/deepseek-v4.1-flash-1017",
            constructor_name: qwen_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (Alibaba)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwenMax {
            model_name: "qwen/qwen-max",
            constructor_name: qwen_qwen_max,
            display_name: "Qwen Max",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        QwenQwenMtPlus {
            model_name: "qwen/qwen-mt-plus",
            constructor_name: qwen_qwen_mt_plus,
            display_name: "Qwen MT Plus",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        QwenQwen3CoderFlash {
            model_name: "qwen/qwen3-coder-flash",
            constructor_name: qwen_qwen3_coder_flash,
            display_name: "Qwen3 Coder Flash",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        QwenQwen3824tA95b {
            model_name: "qwen/qwen3.8-2.4t-a95b",
            constructor_name: qwen_qwen3_8_2_4t_a95b,
            display_name: "Qwen3.8 2.4T A95B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3827b {
            model_name: "qwen/qwen3.8-27b",
            constructor_name: qwen_qwen3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen38Flash {
            model_name: "qwen/qwen3.8-flash",
            constructor_name: qwen_qwen3_8_flash,
            display_name: "Qwen3.8 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen38Max {
            model_name: "qwen/qwen3.8-max",
            constructor_name: qwen_qwen3_8_max,
            display_name: "Qwen3.8 Max",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen38Max0902 {
            model_name: "qwen/qwen3.8-max-0902",
            constructor_name: qwen_qwen3_8_max_0902,
            display_name: "Qwen3.8 Max 0902",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen38OmniFlash {
            model_name: "qwen/qwen3.8-omni-flash",
            constructor_name: qwen_qwen3_8_omni_flash,
            display_name: "Qwen3.8 Omni Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ScalewayDeepseekV4Flash0731 {
            model_name: "scaleway/deepseek-v4-flash-0731",
            constructor_name: scaleway_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731 (Scaleway)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ScalewayDeepseekV41Flash {
            model_name: "scaleway/deepseek-v4.1-flash",
            constructor_name: scaleway_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (Scaleway)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ScalewayGemma327bIt {
            model_name: "scaleway/gemma-3-27b-it",
            constructor_name: scaleway_gemma_3_27b_it,
            display_name: "Gemma 3 27B IT (Scaleway)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        ScalewayGptOss120b {
            model_name: "scaleway/gpt-oss-120b",
            constructor_name: scaleway_gpt_oss_120b,
            display_name: "GPT OSS 120B (Scaleway)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ScalewayLlama3370bInstruct {
            model_name: "scaleway/llama-3.3-70b-instruct",
            constructor_name: scaleway_llama_3_3_70b_instruct,
            display_name: "Llama-3.3-70B-Instruct (Scaleway)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ScalewayQwen3235bA22bInstruct2507 {
            model_name: "scaleway/qwen3-235b-a22b-instruct-2507",
            constructor_name: scaleway_qwen3_235b_a22b_instruct_2507,
            display_name: "Qwen3 235B-A22B Instruct 2507 (Scaleway)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ScalewayQwen3Coder30bA3bInstruct {
            model_name: "scaleway/qwen3-coder-30b-a3b-instruct",
            constructor_name: scaleway_qwen3_coder_30b_a3b_instruct,
            display_name: "Qwen3-Coder 30B-A3B Instruct (Scaleway)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        SferenceDeepseekAiDeepseekV4Flash0731 {
            model_name: "sference/deepseek-ai/DeepSeek-V4-Flash-0731",
            constructor_name: sference_deepseek_ai_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731 (Sference)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        SferenceDeepseekAiDeepseekV41Flash1017 {
            model_name: "sference/deepseek-ai/DeepSeek-V4.1-Flash-1017",
            constructor_name: sference_deepseek_ai_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (Sference)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentDeepseekV4Pro0813 {
            model_name: "tencent/deepseek-v4-pro-0813",
            constructor_name: tencent_deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813 (Tencent)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentDeepseekV41Flash {
            model_name: "tencent/deepseek-v4.1-flash",
            constructor_name: tencent_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (Tencent)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentHy3 {
            model_name: "tencent/hy3",
            constructor_name: tencent_hy3,
            display_name: "Hy3 (Tencent)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentHy4Preview {
            model_name: "tencent/hy4-preview",
            constructor_name: tencent_hy4_preview,
            display_name: "Hy4 preview (Tencent)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentStep5Preview {
            model_name: "tencent/step-5-preview",
            constructor_name: tencent_step_5_preview,
            display_name: "Step 5 Preview (Tencent)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        TensorxDeepseekDeepseekR10528 {
            model_name: "tensorx/deepseek/deepseek-r1-0528",
            constructor_name: tensorx_deepseek_deepseek_r1_0528,
            display_name: "DeepSeek R1 0528 (TensorX)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TensorxDeepseekDeepseekV4Flash0731 {
            model_name: "tensorx/deepseek/deepseek-v4-flash-0731",
            constructor_name: tensorx_deepseek_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731 (TensorX)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TensorxDeepseekDeepseekV4Pro0813 {
            model_name: "tensorx/deepseek/deepseek-v4-pro-0813",
            constructor_name: tensorx_deepseek_deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813 (TensorX)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TensorxDeepseekDeepseekV41Flash {
            model_name: "tensorx/deepseek/deepseek-v4.1-flash",
            constructor_name: tensorx_deepseek_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (TensorX)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TensorxDeepseekDeepseekV41Flash1017 {
            model_name: "tensorx/deepseek/deepseek-v4.1-flash-1017",
            constructor_name: tensorx_deepseek_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (TensorX)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TensorxMoonshotaiKimiK25 {
            model_name: "tensorx/moonshotai/kimi-k2.5",
            constructor_name: tensorx_moonshotai_kimi_k2_5,
            display_name: "Kimi K2.5 (TensorX)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TogetherAiDeepseekAiDeepseekV4Flash0731 {
            model_name: "together_ai/deepseek-ai/DeepSeek-V4-Flash-0731",
            constructor_name: together_ai_deepseek_ai_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731 (Together AI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TogetherAiDeepseekAiDeepseekV4Pro0813 {
            model_name: "together_ai/deepseek-ai/DeepSeek-V4-Pro-0813",
            constructor_name: together_ai_deepseek_ai_deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813 (Together AI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TogetherAiDeepseekAiDeepseekV41Flash {
            model_name: "together_ai/deepseek-ai/DeepSeek-V4.1-Flash",
            constructor_name: together_ai_deepseek_ai_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (Together AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TogetherAiDeepseekAiDeepseekV41Flash1017 {
            model_name: "together_ai/deepseek-ai/DeepSeek-V4.1-Flash-1017",
            constructor_name: together_ai_deepseek_ai_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (Together AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TogetherAiMetaModelsMuseGlimmer30b {
            model_name: "together_ai/meta-models/Muse-Glimmer-30B",
            constructor_name: together_ai_meta_models_muse_glimmer_30b,
            display_name: "Muse Glimmer 30B (Together AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TogetherAiOpenaiGptOss120b {
            model_name: "together_ai/openai/gpt-oss-120b",
            constructor_name: together_ai_openai_gpt_oss_120b,
            display_name: "GPT OSS 120B (Together AI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TogetherAiReflectionBeam10501bA23b {
            model_name: "together_ai/reflection/Beam-1.0-501B-A23B",
            constructor_name: together_ai_reflection_beam_1_0_501b_a23b,
            display_name: "Beam 1.0 501B A23B (Together AI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TogetherAiThinkingmachinesInkling {
            model_name: "together_ai/thinkingmachines/Inkling",
            constructor_name: together_ai_thinkingmachines_inkling,
            display_name: "Inkling (Together AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        VertexGemini25FlashImage {
            model_name: "vertex/gemini-2.5-flash-image",
            constructor_name: vertex_gemini_2_5_flash_image,
            display_name: "Nano Banana (Vertex AI)",
            capabilities: [ImageInputSupport, ImageOutputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        VertexGemini3FlashPreview {
            model_name: "vertex/gemini-3-flash-preview",
            constructor_name: vertex_gemini_3_flash_preview,
            display_name: "Gemini 3 Flash Preview (Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini3ProImage {
            model_name: "vertex/gemini-3-pro-image",
            constructor_name: vertex_gemini_3_pro_image,
            display_name: "Nano Banana Pro (Vertex AI)",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        VertexGemini31FlashImage {
            model_name: "vertex/gemini-3.1-flash-image",
            constructor_name: vertex_gemini_3_1_flash_image,
            display_name: "Nano Banana 2 (Vertex AI)",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        VertexGemini31FlashLite {
            model_name: "vertex/gemini-3.1-flash-lite",
            constructor_name: vertex_gemini_3_1_flash_lite,
            display_name: "Gemini 3.1 Flash Lite (Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini31FlashLiteImage {
            model_name: "vertex/gemini-3.1-flash-lite-image",
            constructor_name: vertex_gemini_3_1_flash_lite_image,
            display_name: "Nano Banana 2 Lite (Vertex AI)",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        VertexGemini31FlashLiteEu {
            model_name: "vertex/gemini-3.1-flash-lite@eu",
            constructor_name: vertex_gemini_3_1_flash_lite_eu,
            display_name: "Gemini 3.1 Flash Lite (Vertex AI, EU)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini31FlashLiteUs {
            model_name: "vertex/gemini-3.1-flash-lite@us",
            constructor_name: vertex_gemini_3_1_flash_lite_us,
            display_name: "Gemini 3.1 Flash Lite (Vertex AI, US)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini31ProPreview {
            model_name: "vertex/gemini-3.1-pro-preview",
            constructor_name: vertex_gemini_3_1_pro_preview,
            display_name: "Gemini 3.1 Pro Preview (Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini35Flash {
            model_name: "vertex/gemini-3.5-flash",
            constructor_name: vertex_gemini_3_5_flash,
            display_name: "Gemini 3.5 Flash (Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini35FlashLite {
            model_name: "vertex/gemini-3.5-flash-lite",
            constructor_name: vertex_gemini_3_5_flash_lite,
            display_name: "Gemini 3.5 Flash Lite (Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini35FlashLiteEu {
            model_name: "vertex/gemini-3.5-flash-lite@eu",
            constructor_name: vertex_gemini_3_5_flash_lite_eu,
            display_name: "Gemini 3.5 Flash Lite (Vertex AI, EU)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini35FlashLiteUs {
            model_name: "vertex/gemini-3.5-flash-lite@us",
            constructor_name: vertex_gemini_3_5_flash_lite_us,
            display_name: "Gemini 3.5 Flash Lite (Vertex AI, US)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini35FlashEu {
            model_name: "vertex/gemini-3.5-flash@eu",
            constructor_name: vertex_gemini_3_5_flash_eu,
            display_name: "Gemini 3.5 Flash (Vertex AI, EU)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini35FlashUs {
            model_name: "vertex/gemini-3.5-flash@us",
            constructor_name: vertex_gemini_3_5_flash_us,
            display_name: "Gemini 3.5 Flash (Vertex AI, US)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini36Flash {
            model_name: "vertex/gemini-3.6-flash",
            constructor_name: vertex_gemini_3_6_flash,
            display_name: "Gemini 3.6 Flash (Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini36FlashEu {
            model_name: "vertex/gemini-3.6-flash@eu",
            constructor_name: vertex_gemini_3_6_flash_eu,
            display_name: "Gemini 3.6 Flash (Vertex AI, EU)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini36FlashUs {
            model_name: "vertex/gemini-3.6-flash@us",
            constructor_name: vertex_gemini_3_6_flash_us,
            display_name: "Gemini 3.6 Flash (Vertex AI, US)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini37Flash {
            model_name: "vertex/gemini-3.7-flash",
            constructor_name: vertex_gemini_3_7_flash,
            display_name: "Gemini 3.7 Flash (Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini37FlashEu {
            model_name: "vertex/gemini-3.7-flash@eu",
            constructor_name: vertex_gemini_3_7_flash_eu,
            display_name: "Gemini 3.7 Flash (Vertex AI, EU)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini37FlashUs {
            model_name: "vertex/gemini-3.7-flash@us",
            constructor_name: vertex_gemini_3_7_flash_us,
            display_name: "Gemini 3.7 Flash (Vertex AI, US)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini38Flash {
            model_name: "vertex/gemini-3.8-flash",
            constructor_name: vertex_gemini_3_8_flash,
            display_name: "Gemini 3.8 Flash (Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini38FlashEu {
            model_name: "vertex/gemini-3.8-flash@eu",
            constructor_name: vertex_gemini_3_8_flash_eu,
            display_name: "Gemini 3.8 Flash (Vertex AI, EU)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini38FlashUs {
            model_name: "vertex/gemini-3.8-flash@us",
            constructor_name: vertex_gemini_3_8_flash_us,
            display_name: "Gemini 3.8 Flash (Vertex AI, US)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini4Argon {
            model_name: "vertex/gemini-4-argon",
            constructor_name: vertex_gemini_4_argon,
            display_name: "Gemini 4 Argon (Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini4ArgonEu {
            model_name: "vertex/gemini-4-argon@eu",
            constructor_name: vertex_gemini_4_argon_eu,
            display_name: "Gemini 4 Argon (Vertex AI, EU)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGemini4ArgonUs {
            model_name: "vertex/gemini-4-argon@us",
            constructor_name: vertex_gemini_4_argon_us,
            display_name: "Gemini 4 Argon (Vertex AI, US)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGeminiFlashLatest {
            model_name: "vertex/gemini-flash-latest",
            constructor_name: vertex_gemini_flash_latest,
            display_name: "Gemini Flash Latest (Gemini 3.8 Flash, Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VertexGeminiNanoBanana21 {
            model_name: "vertex/gemini-nano-banana-2.1",
            constructor_name: vertex_gemini_nano_banana_2_1,
            display_name: "Nano Banana 2.1 (Vertex AI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        VertexGeminiProLatest {
            model_name: "vertex/gemini-pro-latest",
            constructor_name: vertex_gemini_pro_latest,
            display_name: "Gemini Pro Latest (Gemini 4 Argon, Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        XaiGrok4FastNonReasoning {
            model_name: "xai/grok-4-fast-non-reasoning",
            constructor_name: xai_grok_4_fast_non_reasoning,
            display_name: "X-Ai/Grok-4-Fast-Non-Reasoning",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok4200309NonReasoning {
            model_name: "xai/grok-4.20-0309-non-reasoning",
            constructor_name: xai_grok_4_20_0309_non_reasoning,
            display_name: "Grok 4.20 (Non-Reasoning)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok4200309Reasoning {
            model_name: "xai/grok-4.20-0309-reasoning",
            constructor_name: xai_grok_4_20_0309_reasoning,
            display_name: "Grok 4.20 (Reasoning)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok43 {
            model_name: "xai/grok-4.3",
            constructor_name: xai_grok_4_3,
            display_name: "Grok 4.3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok45 {
            model_name: "xai/grok-4.5",
            constructor_name: xai_grok_4_5,
            display_name: "Grok 4.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok46 {
            model_name: "xai/grok-4.6",
            constructor_name: xai_grok_4_6,
            display_name: "Grok 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok47 {
            model_name: "xai/grok-4.7",
            constructor_name: xai_grok_4_7,
            display_name: "Grok 4.7",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrokBuild01 {
            model_name: "xai/grok-build-0.1",
            constructor_name: xai_grok_build_0_1,
            display_name: "Grok Build 0.1",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrokLatest {
            model_name: "xai/grok-latest",
            constructor_name: xai_grok_latest,
            display_name: "Grok Latest (Grok 4.7)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm46 {
            model_name: "zai/glm-4.6",
            constructor_name: zai_glm_4_6,
            display_name: "GLM-4.6",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm46v {
            model_name: "zai/glm-4.6v",
            constructor_name: zai_glm_4_6v,
            display_name: "GLM-4.6V",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ZaiGlm47 {
            model_name: "zai/glm-4.7",
            constructor_name: zai_glm_4_7,
            display_name: "GLM-4.7",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm5 {
            model_name: "zai/glm-5",
            constructor_name: zai_glm_5,
            display_name: "GLM-5",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm5Turbo {
            model_name: "zai/glm-5-turbo",
            constructor_name: zai_glm_5_turbo,
            display_name: "GLM-5-Turbo",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm51 {
            model_name: "zai/glm-5.1",
            constructor_name: zai_glm_5_1,
            display_name: "GLM-5.1",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm52 {
            model_name: "zai/glm-5.2",
            constructor_name: zai_glm_5_2,
            display_name: "GLM-5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm53 {
            model_name: "zai/glm-5.3",
            constructor_name: zai_glm_5_3,
            display_name: "GLM-5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm53Flash {
            model_name: "zai/glm-5.3-flash",
            constructor_name: zai_glm_5_3_flash,
            display_name: "GLM-5.3-Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ZaiGlm5vTurbo {
            model_name: "zai/glm-5v-turbo",
            constructor_name: zai_glm_5v_turbo,
            display_name: "GLM-5V-Turbo",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
