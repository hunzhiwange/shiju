//! Capabilities for llmgateway_providers models.
//!
//! This module defines model types and their capabilities for llmgateway_providers providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::llmgateway_providers::LlmgatewayProviders;

model_capabilities! {
    provider: LlmgatewayProviders,
    models: {
        AlibabaDeepseekV4Flash {
            model_name: "alibaba/deepseek-v4-flash",
            constructor_name: alibaba_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash (Alibaba Cloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaDeepseekV4Pro {
            model_name: "alibaba/deepseek-v4-pro",
            constructor_name: alibaba_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro (Alibaba Cloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaDeepseekV41Flash {
            model_name: "alibaba/deepseek-v4.1-flash",
            constructor_name: alibaba_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaDeepseekV41Flash1017 {
            model_name: "alibaba/deepseek-v4.1-flash-1017",
            constructor_name: alibaba_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaGlm5 {
            model_name: "alibaba/glm-5",
            constructor_name: alibaba_glm_5,
            display_name: "GLM-5 (Alibaba Cloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaGlm52 {
            model_name: "alibaba/glm-5.2",
            constructor_name: alibaba_glm_5_2,
            display_name: "GLM-5.2 (Alibaba Cloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaGlm53 {
            model_name: "alibaba/glm-5.3",
            constructor_name: alibaba_glm_5_3,
            display_name: "GLM-5.3 (Alibaba Cloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaKimiK25 {
            model_name: "alibaba/kimi-k2.5",
            constructor_name: alibaba_kimi_k2_5,
            display_name: "Kimi K2.5 (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaKimiK3 {
            model_name: "alibaba/kimi-k3",
            constructor_name: alibaba_kimi_k3,
            display_name: "Kimi K3 (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaQwenFlash {
            model_name: "alibaba/qwen-flash",
            constructor_name: alibaba_qwen_flash,
            display_name: "Qwen Flash (Alibaba Cloud)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaQwenMax {
            model_name: "alibaba/qwen-max",
            constructor_name: alibaba_qwen_max,
            display_name: "Qwen Max (Alibaba Cloud)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaQwenPlus {
            model_name: "alibaba/qwen-plus",
            constructor_name: alibaba_qwen_plus,
            display_name: "Qwen Plus (Alibaba Cloud)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaQwenPlusLatest {
            model_name: "alibaba/qwen-plus-latest",
            constructor_name: alibaba_qwen_plus_latest,
            display_name: "Qwen Plus Latest (Alibaba Cloud)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaQwen3CoderFlash {
            model_name: "alibaba/qwen3-coder-flash",
            constructor_name: alibaba_qwen3_coder_flash,
            display_name: "Qwen3 Coder Flash (Alibaba Cloud)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaQwen3VlPlus {
            model_name: "alibaba/qwen3-vl-plus",
            constructor_name: alibaba_qwen3_vl_plus,
            display_name: "Qwen3 VL Plus (Alibaba Cloud)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        AlibabaQwen35122bA10b {
            model_name: "alibaba/qwen3.5-122b-a10b",
            constructor_name: alibaba_qwen3_5_122b_a10b,
            display_name: "Qwen3.5 122B A10B (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaQwen3527b {
            model_name: "alibaba/qwen3.5-27b",
            constructor_name: alibaba_qwen3_5_27b,
            display_name: "Qwen3.5 27B (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaQwen3535bA3b {
            model_name: "alibaba/qwen3.5-35b-a3b",
            constructor_name: alibaba_qwen3_5_35b_a3b,
            display_name: "Qwen3.5 35B A3B (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaQwen3635bA3b {
            model_name: "alibaba/qwen3.6-35b-a3b",
            constructor_name: alibaba_qwen3_6_35b_a3b,
            display_name: "Qwen3.6 35B A3B (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaQwen36Flash {
            model_name: "alibaba/qwen3.6-flash",
            constructor_name: alibaba_qwen3_6_flash,
            display_name: "Qwen3.6 Flash (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaQwen36Plus {
            model_name: "alibaba/qwen3.6-plus",
            constructor_name: alibaba_qwen3_6_plus,
            display_name: "Qwen3.6 Plus (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaQwen37Flash {
            model_name: "alibaba/qwen3.7-flash",
            constructor_name: alibaba_qwen3_7_flash,
            display_name: "Qwen3.7 Flash (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaQwen37Max {
            model_name: "alibaba/qwen3.7-max",
            constructor_name: alibaba_qwen3_7_max,
            display_name: "Qwen3.7 Max (Alibaba Cloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AlibabaQwen37Plus {
            model_name: "alibaba/qwen3.7-plus",
            constructor_name: alibaba_qwen3_7_plus,
            display_name: "Qwen3.7 Plus (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaQwen38Flash {
            model_name: "alibaba/qwen3.8-flash",
            constructor_name: alibaba_qwen3_8_flash,
            display_name: "Qwen3.8 Flash (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaQwen38Max {
            model_name: "alibaba/qwen3.8-max",
            constructor_name: alibaba_qwen3_8_max,
            display_name: "Qwen3.8 Max (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AlibabaQwen35397bA17b {
            model_name: "alibaba/qwen35-397b-a17b",
            constructor_name: alibaba_qwen35_397b_a17b,
            display_name: "Qwen3.5 397B A17B (Alibaba Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AnthropicClaudeFable5 {
            model_name: "anthropic/claude-fable-5",
            constructor_name: anthropic_claude_fable_5,
            display_name: "Claude Fable 5 (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeFable51 {
            model_name: "anthropic/claude-fable-5-1",
            constructor_name: anthropic_claude_fable_5_1,
            display_name: "Claude Fable 5.1 (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeHaiku45 {
            model_name: "anthropic/claude-haiku-4-5",
            constructor_name: anthropic_claude_haiku_4_5,
            display_name: "Claude Haiku 4.5 (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeHaiku4520251001 {
            model_name: "anthropic/claude-haiku-4-5-20251001",
            constructor_name: anthropic_claude_haiku_4_5_20251001,
            display_name: "Claude Haiku 4.5 (2025-10-01) (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeHaiku55 {
            model_name: "anthropic/claude-haiku-5-5",
            constructor_name: anthropic_claude_haiku_5_5,
            display_name: "Claude Haiku 5.5 (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus4520251101 {
            model_name: "anthropic/claude-opus-4-5-20251101",
            constructor_name: anthropic_claude_opus_4_5_20251101,
            display_name: "Claude Opus 4.5 (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus46 {
            model_name: "anthropic/claude-opus-4-6",
            constructor_name: anthropic_claude_opus_4_6,
            display_name: "Claude Opus 4.6 (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus47 {
            model_name: "anthropic/claude-opus-4-7",
            constructor_name: anthropic_claude_opus_4_7,
            display_name: "Claude Opus 4.7 (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus48 {
            model_name: "anthropic/claude-opus-4-8",
            constructor_name: anthropic_claude_opus_4_8,
            display_name: "Claude Opus 4.8 (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus5 {
            model_name: "anthropic/claude-opus-5",
            constructor_name: anthropic_claude_opus_5,
            display_name: "Claude Opus 5 (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus55 {
            model_name: "anthropic/claude-opus-5-5",
            constructor_name: anthropic_claude_opus_5_5,
            display_name: "Claude Opus 5.5 (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet45 {
            model_name: "anthropic/claude-sonnet-4-5",
            constructor_name: anthropic_claude_sonnet_4_5,
            display_name: "Claude Sonnet 4.5 (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet4520250929 {
            model_name: "anthropic/claude-sonnet-4-5-20250929",
            constructor_name: anthropic_claude_sonnet_4_5_20250929,
            display_name: "Claude Sonnet 4.5 (2025-09-29) (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet46 {
            model_name: "anthropic/claude-sonnet-4-6",
            constructor_name: anthropic_claude_sonnet_4_6,
            display_name: "Claude Sonnet 4.6 (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet5 {
            model_name: "anthropic/claude-sonnet-5",
            constructor_name: anthropic_claude_sonnet_5,
            display_name: "Claude Sonnet 5 (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet55 {
            model_name: "anthropic/claude-sonnet-5-5",
            constructor_name: anthropic_claude_sonnet_5_5,
            display_name: "Claude Sonnet 5.5 (Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeFable5 {
            model_name: "aws-bedrock/claude-fable-5",
            constructor_name: aws_bedrock_claude_fable_5,
            display_name: "Claude Fable 5 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeFable51 {
            model_name: "aws-bedrock/claude-fable-5-1",
            constructor_name: aws_bedrock_claude_fable_5_1,
            display_name: "Claude Fable 5.1 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeHaiku45 {
            model_name: "aws-bedrock/claude-haiku-4-5",
            constructor_name: aws_bedrock_claude_haiku_4_5,
            display_name: "Claude Haiku 4.5 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeHaiku4520251001 {
            model_name: "aws-bedrock/claude-haiku-4-5-20251001",
            constructor_name: aws_bedrock_claude_haiku_4_5_20251001,
            display_name: "Claude Haiku 4.5 (2025-10-01) (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeHaiku55 {
            model_name: "aws-bedrock/claude-haiku-5-5",
            constructor_name: aws_bedrock_claude_haiku_5_5,
            display_name: "Claude Haiku 5.5 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeOpus4520251101 {
            model_name: "aws-bedrock/claude-opus-4-5-20251101",
            constructor_name: aws_bedrock_claude_opus_4_5_20251101,
            display_name: "Claude Opus 4.5 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeOpus46 {
            model_name: "aws-bedrock/claude-opus-4-6",
            constructor_name: aws_bedrock_claude_opus_4_6,
            display_name: "Claude Opus 4.6 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeOpus47 {
            model_name: "aws-bedrock/claude-opus-4-7",
            constructor_name: aws_bedrock_claude_opus_4_7,
            display_name: "Claude Opus 4.7 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeOpus48 {
            model_name: "aws-bedrock/claude-opus-4-8",
            constructor_name: aws_bedrock_claude_opus_4_8,
            display_name: "Claude Opus 4.8 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeOpus5 {
            model_name: "aws-bedrock/claude-opus-5",
            constructor_name: aws_bedrock_claude_opus_5,
            display_name: "Claude Opus 5 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeOpus55 {
            model_name: "aws-bedrock/claude-opus-5-5",
            constructor_name: aws_bedrock_claude_opus_5_5,
            display_name: "Claude Opus 5.5 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeSonnet45 {
            model_name: "aws-bedrock/claude-sonnet-4-5",
            constructor_name: aws_bedrock_claude_sonnet_4_5,
            display_name: "Claude Sonnet 4.5 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeSonnet4520250929 {
            model_name: "aws-bedrock/claude-sonnet-4-5-20250929",
            constructor_name: aws_bedrock_claude_sonnet_4_5_20250929,
            display_name: "Claude Sonnet 4.5 (2025-09-29) (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeSonnet46 {
            model_name: "aws-bedrock/claude-sonnet-4-6",
            constructor_name: aws_bedrock_claude_sonnet_4_6,
            display_name: "Claude Sonnet 4.6 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeSonnet5 {
            model_name: "aws-bedrock/claude-sonnet-5",
            constructor_name: aws_bedrock_claude_sonnet_5,
            display_name: "Claude Sonnet 5 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockClaudeSonnet55 {
            model_name: "aws-bedrock/claude-sonnet-5-5",
            constructor_name: aws_bedrock_claude_sonnet_5_5,
            display_name: "Claude Sonnet 5.5 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockGrok43 {
            model_name: "aws-bedrock/grok-4-3",
            constructor_name: aws_bedrock_grok_4_3,
            display_name: "Grok 4.3 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockGrok46 {
            model_name: "aws-bedrock/grok-4-6",
            constructor_name: aws_bedrock_grok_4_6,
            display_name: "Grok 4.6 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockGrok47 {
            model_name: "aws-bedrock/grok-4-7",
            constructor_name: aws_bedrock_grok_4_7,
            display_name: "Grok 4.7 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsBedrockKimiK3 {
            model_name: "aws-bedrock/kimi-k3",
            constructor_name: aws_bedrock_kimi_k3,
            display_name: "Kimi K3 (AWS Bedrock)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AwsMantleGpt56Luna {
            model_name: "aws-mantle/gpt-5.6-luna",
            constructor_name: aws_mantle_gpt_5_6_luna,
            display_name: "GPT-5.6 Luna (AWS Mantle)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsMantleGpt56Sol {
            model_name: "aws-mantle/gpt-5.6-sol",
            constructor_name: aws_mantle_gpt_5_6_sol,
            display_name: "GPT-5.6 Sol (AWS Mantle)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsMantleGpt56Terra {
            model_name: "aws-mantle/gpt-5.6-terra",
            constructor_name: aws_mantle_gpt_5_6_terra,
            display_name: "GPT-5.6 Terra (AWS Mantle)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AwsMantleGpt6Astra {
            model_name: "aws-mantle/gpt-6-astra",
            constructor_name: aws_mantle_gpt_6_astra,
            display_name: "GPT-6 Astra (AWS Mantle)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureAiFoundryGrok41FastNonReasoning {
            model_name: "azure-ai-foundry/grok-4-1-fast-non-reasoning",
            constructor_name: azure_ai_foundry_grok_4_1_fast_non_reasoning,
            display_name: "Grok 4.1 Fast Non-Reasoning (Azure AI Foundry)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureAiFoundryGrok41FastReasoning {
            model_name: "azure-ai-foundry/grok-4-1-fast-reasoning",
            constructor_name: azure_ai_foundry_grok_4_1_fast_reasoning,
            display_name: "Grok 4.1 Fast Reasoning (Azure AI Foundry)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureAiFoundryGrok43 {
            model_name: "azure-ai-foundry/grok-4-3",
            constructor_name: azure_ai_foundry_grok_4_3,
            display_name: "Grok 4.3 (Azure AI Foundry)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureAnthropicClaudeFable5 {
            model_name: "azure-anthropic/claude-fable-5",
            constructor_name: azure_anthropic_claude_fable_5,
            display_name: "Claude Fable 5 (Azure Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureAnthropicClaudeFable51 {
            model_name: "azure-anthropic/claude-fable-5-1",
            constructor_name: azure_anthropic_claude_fable_5_1,
            display_name: "Claude Fable 5.1 (Azure Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureAnthropicClaudeOpus46 {
            model_name: "azure-anthropic/claude-opus-4-6",
            constructor_name: azure_anthropic_claude_opus_4_6,
            display_name: "Claude Opus 4.6 (Azure Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureAnthropicClaudeOpus47 {
            model_name: "azure-anthropic/claude-opus-4-7",
            constructor_name: azure_anthropic_claude_opus_4_7,
            display_name: "Claude Opus 4.7 (Azure Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureAnthropicClaudeOpus48 {
            model_name: "azure-anthropic/claude-opus-4-8",
            constructor_name: azure_anthropic_claude_opus_4_8,
            display_name: "Claude Opus 4.8 (Azure Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureAnthropicClaudeOpus5 {
            model_name: "azure-anthropic/claude-opus-5",
            constructor_name: azure_anthropic_claude_opus_5,
            display_name: "Claude Opus 5 (Azure Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureAnthropicClaudeOpus55 {
            model_name: "azure-anthropic/claude-opus-5-5",
            constructor_name: azure_anthropic_claude_opus_5_5,
            display_name: "Claude Opus 5.5 (Azure Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureAnthropicClaudeSonnet5 {
            model_name: "azure-anthropic/claude-sonnet-5",
            constructor_name: azure_anthropic_claude_sonnet_5,
            display_name: "Claude Sonnet 5 (Azure Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureAnthropicClaudeSonnet55 {
            model_name: "azure-anthropic/claude-sonnet-5-5",
            constructor_name: azure_anthropic_claude_sonnet_5_5,
            display_name: "Claude Sonnet 5.5 (Azure Anthropic)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt41 {
            model_name: "azure/gpt-4.1",
            constructor_name: azure_gpt_4_1,
            display_name: "GPT-4.1 (Azure)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt41Mini {
            model_name: "azure/gpt-4.1-mini",
            constructor_name: azure_gpt_4_1_mini,
            display_name: "GPT-4.1 Mini (Azure)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt4o {
            model_name: "azure/gpt-4o",
            constructor_name: azure_gpt_4o,
            display_name: "GPT-4o (Azure)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt5 {
            model_name: "azure/gpt-5",
            constructor_name: azure_gpt_5,
            display_name: "GPT-5 (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt5Mini {
            model_name: "azure/gpt-5-mini",
            constructor_name: azure_gpt_5_mini,
            display_name: "GPT-5 Mini (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt5Nano {
            model_name: "azure/gpt-5-nano",
            constructor_name: azure_gpt_5_nano,
            display_name: "GPT-5 Nano (Azure)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt51 {
            model_name: "azure/gpt-5.1",
            constructor_name: azure_gpt_5_1,
            display_name: "GPT-5.1 (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt51Codex {
            model_name: "azure/gpt-5.1-codex",
            constructor_name: azure_gpt_5_1_codex,
            display_name: "GPT-5.1 Codex (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt51CodexMini {
            model_name: "azure/gpt-5.1-codex-mini",
            constructor_name: azure_gpt_5_1_codex_mini,
            display_name: "GPT-5.1 Codex mini (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt52 {
            model_name: "azure/gpt-5.2",
            constructor_name: azure_gpt_5_2,
            display_name: "GPT-5.2 (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt52Codex {
            model_name: "azure/gpt-5.2-codex",
            constructor_name: azure_gpt_5_2_codex,
            display_name: "GPT-5.2 Codex (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt53Codex {
            model_name: "azure/gpt-5.3-codex",
            constructor_name: azure_gpt_5_3_codex,
            display_name: "GPT-5.3 Codex (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt54 {
            model_name: "azure/gpt-5.4",
            constructor_name: azure_gpt_5_4,
            display_name: "GPT-5.4 (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt54Mini {
            model_name: "azure/gpt-5.4-mini",
            constructor_name: azure_gpt_5_4_mini,
            display_name: "GPT-5.4 Mini (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt54Nano {
            model_name: "azure/gpt-5.4-nano",
            constructor_name: azure_gpt_5_4_nano,
            display_name: "GPT-5.4 Nano (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt54Pro {
            model_name: "azure/gpt-5.4-pro",
            constructor_name: azure_gpt_5_4_pro,
            display_name: "GPT-5.4 Pro (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt55 {
            model_name: "azure/gpt-5.5",
            constructor_name: azure_gpt_5_5,
            display_name: "GPT-5.5 (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt56Luna {
            model_name: "azure/gpt-5.6-luna",
            constructor_name: azure_gpt_5_6_luna,
            display_name: "GPT-5.6 Luna (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt56Sol {
            model_name: "azure/gpt-5.6-sol",
            constructor_name: azure_gpt_5_6_sol,
            display_name: "GPT-5.6 Sol (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt56Terra {
            model_name: "azure/gpt-5.6-terra",
            constructor_name: azure_gpt_5_6_terra,
            display_name: "GPT-5.6 Terra (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt6Astra {
            model_name: "azure/gpt-6-astra",
            constructor_name: azure_gpt_6_astra,
            display_name: "GPT-6 Astra (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt6Luna {
            model_name: "azure/gpt-6-luna",
            constructor_name: azure_gpt_6_luna,
            display_name: "GPT-6 Luna (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt6Sol {
            model_name: "azure/gpt-6-sol",
            constructor_name: azure_gpt_6_sol,
            display_name: "GPT-6 Sol (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGpt61Sol {
            model_name: "azure/gpt-6.1-sol",
            constructor_name: azure_gpt_6_1_sol,
            display_name: "GPT-6.1 Sol (Azure)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AzureGptOss120b {
            model_name: "azure/gpt-oss-120b",
            constructor_name: azure_gpt_oss_120b,
            display_name: "GPT OSS 120B (Azure)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BaiduDeepseekV4Flash {
            model_name: "baidu/deepseek-v4-flash",
            constructor_name: baidu_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash (Baidu)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BaiduDeepseekV4Pro {
            model_name: "baidu/deepseek-v4-pro",
            constructor_name: baidu_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro (Baidu)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BaiduDeepseekV41Flash {
            model_name: "baidu/deepseek-v4.1-flash",
            constructor_name: baidu_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (Baidu)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BaiduGlm51 {
            model_name: "baidu/glm-5.1",
            constructor_name: baidu_glm_5_1,
            display_name: "GLM-5.1 (Baidu)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BaiduGlm52 {
            model_name: "baidu/glm-5.2",
            constructor_name: baidu_glm_5_2,
            display_name: "GLM-5.2 (Baidu)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BaiduGlm53 {
            model_name: "baidu/glm-5.3",
            constructor_name: baidu_glm_5_3,
            display_name: "GLM-5.3 (Baidu)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BaiduKimiK26 {
            model_name: "baidu/kimi-k2.6",
            constructor_name: baidu_kimi_k2_6,
            display_name: "Kimi K2.6 (Baidu)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BytedanceDeepseekV32 {
            model_name: "bytedance/deepseek-v3.2",
            constructor_name: bytedance_deepseek_v3_2,
            display_name: "DeepSeek V3.2 (ByteDance)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceDeepseekV4Flash {
            model_name: "bytedance/deepseek-v4-flash",
            constructor_name: bytedance_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash (ByteDance)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceDeepseekV4Pro {
            model_name: "bytedance/deepseek-v4-pro",
            constructor_name: bytedance_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro (ByteDance)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceGlm47 {
            model_name: "bytedance/glm-4.7",
            constructor_name: bytedance_glm_4_7,
            display_name: "GLM-4.7 (ByteDance)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceGlm52 {
            model_name: "bytedance/glm-5.2",
            constructor_name: bytedance_glm_5_2,
            display_name: "GLM-5.2 (ByteDance)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceGptOss120b {
            model_name: "bytedance/gpt-oss-120b",
            constructor_name: bytedance_gpt_oss_120b,
            display_name: "GPT OSS 120B (ByteDance)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceSeed16250615 {
            model_name: "bytedance/seed-1-6-250615",
            constructor_name: bytedance_seed_1_6_250615,
            display_name: "Seed 1.6 (250615) (ByteDance)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceSeed16250915 {
            model_name: "bytedance/seed-1-6-250915",
            constructor_name: bytedance_seed_1_6_250915,
            display_name: "Seed 1.6 (250915) (ByteDance)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceSeed16Flash250715 {
            model_name: "bytedance/seed-1-6-flash-250715",
            constructor_name: bytedance_seed_1_6_flash_250715,
            display_name: "Seed 1.6 Flash (250715) (ByteDance)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceSeed18251228 {
            model_name: "bytedance/seed-1-8-251228",
            constructor_name: bytedance_seed_1_8_251228,
            display_name: "Seed 1.8 (251228) (ByteDance)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceSeed20CodePreview260328 {
            model_name: "bytedance/seed-2-0-code-preview-260328",
            constructor_name: bytedance_seed_2_0_code_preview_260328,
            display_name: "Seed 2.0 Code Preview (260328) (ByteDance)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceSeed20Lite260428 {
            model_name: "bytedance/seed-2-0-lite-260428",
            constructor_name: bytedance_seed_2_0_lite_260428,
            display_name: "Seed 2.0 Lite (260428) (ByteDance)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceSeed20Mini260428 {
            model_name: "bytedance/seed-2-0-mini-260428",
            constructor_name: bytedance_seed_2_0_mini_260428,
            display_name: "Seed 2.0 Mini (260428) (ByteDance)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BytedanceSeed20Pro260328 {
            model_name: "bytedance/seed-2-0-pro-260328",
            constructor_name: bytedance_seed_2_0_pro_260328,
            display_name: "Seed 2.0 Pro (260328) (ByteDance)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CanopywaveDeepseekV4Flash {
            model_name: "canopywave/deepseek-v4-flash",
            constructor_name: canopywave_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash (CanopyWave)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CanopywaveDeepseekV4Pro {
            model_name: "canopywave/deepseek-v4-pro",
            constructor_name: canopywave_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro (CanopyWave)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CanopywaveGlm52 {
            model_name: "canopywave/glm-5.2",
            constructor_name: canopywave_glm_5_2,
            display_name: "GLM-5.2 (CanopyWave)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CanopywaveKimiK26 {
            model_name: "canopywave/kimi-k2.6",
            constructor_name: canopywave_kimi_k2_6,
            display_name: "Kimi K2.6 (CanopyWave)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        CanopywaveKimiK3 {
            model_name: "canopywave/kimi-k3",
            constructor_name: canopywave_kimi_k3,
            display_name: "Kimi K3 (CanopyWave)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        CerebrasGemma431bIt {
            model_name: "cerebras/gemma-4-31b-it",
            constructor_name: cerebras_gemma_4_31b_it,
            display_name: "Gemma 4 31B IT (Cerebras)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CerebrasGlm47 {
            model_name: "cerebras/glm-4.7",
            constructor_name: cerebras_glm_4_7,
            display_name: "GLM-4.7 (Cerebras)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CerebrasGptOss120b {
            model_name: "cerebras/gpt-oss-120b",
            constructor_name: cerebras_gpt_oss_120b,
            display_name: "GPT OSS 120B (Cerebras)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CerebrasLlama3370bInstruct {
            model_name: "cerebras/llama-3.3-70b-instruct",
            constructor_name: cerebras_llama_3_3_70b_instruct,
            display_name: "Llama 3.3 70B Instruct (Cerebras)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CerebrasQwen3235bA22bInstruct2507 {
            model_name: "cerebras/qwen3-235b-a22b-instruct-2507",
            constructor_name: cerebras_qwen3_235b_a22b_instruct_2507,
            display_name: "Qwen3 235B A22B Instruct 2507 (Cerebras)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ConsensusprotocolQwen3827b {
            model_name: "consensusprotocol/Qwen3.8-27B",
            constructor_name: consensusprotocol_qwen3_8_27b,
            display_name: "Qwen3.8 27B (Consensus Protocol)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ConsensusprotocolDeepseekV4Flash {
            model_name: "consensusprotocol/deepseek-v4-flash",
            constructor_name: consensusprotocol_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash (Consensus Protocol)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ConsensusprotocolDeepseekV41Flash {
            model_name: "consensusprotocol/deepseek-v4.1-flash",
            constructor_name: consensusprotocol_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (Consensus Protocol)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ConsensusprotocolGemma431bIt {
            model_name: "consensusprotocol/gemma-4-31b-it",
            constructor_name: consensusprotocol_gemma_4_31b_it,
            display_name: "Gemma 4 31B IT (Consensus Protocol)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ConsensusprotocolGlm53Flash {
            model_name: "consensusprotocol/glm-5.3-flash",
            constructor_name: consensusprotocol_glm_5_3_flash,
            display_name: "GLM-5.3 Flash (Consensus Protocol)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ConsensusprotocolGptOss20b {
            model_name: "consensusprotocol/gpt-oss-20b",
            constructor_name: consensusprotocol_gpt_oss_20b,
            display_name: "GPT OSS 20B (Consensus Protocol)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ConsensusprotocolMimoV26Flash {
            model_name: "consensusprotocol/mimo-v2.6-flash",
            constructor_name: consensusprotocol_mimo_v2_6_flash,
            display_name: "MiMo V2.6 Flash (Consensus Protocol)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ConsensusprotocolMinimaxM3 {
            model_name: "consensusprotocol/minimax-m3",
            constructor_name: consensusprotocol_minimax_m3,
            display_name: "MiniMax M3 (Consensus Protocol)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraDeepseekV32 {
            model_name: "deepinfra/deepseek-v3.2",
            constructor_name: deepinfra_deepseek_v3_2,
            display_name: "DeepSeek V3.2 (DeepInfra)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraDeepseekV4Flash {
            model_name: "deepinfra/deepseek-v4-flash",
            constructor_name: deepinfra_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash (DeepInfra)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraDeepseekV4Pro {
            model_name: "deepinfra/deepseek-v4-pro",
            constructor_name: deepinfra_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro (DeepInfra)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraDeepseekV41Flash {
            model_name: "deepinfra/deepseek-v4.1-flash",
            constructor_name: deepinfra_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (DeepInfra)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraDeepseekV41Flash1017 {
            model_name: "deepinfra/deepseek-v4.1-flash-1017",
            constructor_name: deepinfra_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (DeepInfra)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraGemma426bA4bIt {
            model_name: "deepinfra/gemma-4-26b-a4b-it",
            constructor_name: deepinfra_gemma_4_26b_a4b_it,
            display_name: "Gemma 4 26B A4B IT (DeepInfra)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraGemma431bIt {
            model_name: "deepinfra/gemma-4-31b-it",
            constructor_name: deepinfra_gemma_4_31b_it,
            display_name: "Gemma 4 31B IT (DeepInfra)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraGranite428b {
            model_name: "deepinfra/granite-4.2-8b",
            constructor_name: deepinfra_granite_4_2_8b,
            display_name: "Granite 4.2 8B (DeepInfra)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraLing30Flash {
            model_name: "deepinfra/ling-3.0-flash",
            constructor_name: deepinfra_ling_3_0_flash,
            display_name: "InclusionAI Ling 3.0 Flash (DeepInfra)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraLing30FlashVl {
            model_name: "deepinfra/ling-3.0-flash-vl",
            constructor_name: deepinfra_ling_3_0_flash_vl,
            display_name: "Ling 3.0 Flash VL (DeepInfra)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DeepinfraMimoV26Flash {
            model_name: "deepinfra/mimo-v2.6-flash",
            constructor_name: deepinfra_mimo_v2_6_flash,
            display_name: "MiMo V2.6 Flash (DeepInfra)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DeepinfraMimoV26Pro {
            model_name: "deepinfra/mimo-v2.6-pro",
            constructor_name: deepinfra_mimo_v2_6_pro,
            display_name: "MiMo V2.6 Pro (DeepInfra)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DeepinfraMuseGlimmer30b {
            model_name: "deepinfra/muse-glimmer-30b",
            constructor_name: deepinfra_muse_glimmer_30b,
            display_name: "Muse Glimmer 30B (DeepInfra)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraNemotron3Ultra550b {
            model_name: "deepinfra/nemotron-3-ultra-550b",
            constructor_name: deepinfra_nemotron_3_ultra_550b,
            display_name: "Nemotron 3 Ultra 550B (DeepInfra)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraQwen3Vl235bA22bInstruct {
            model_name: "deepinfra/qwen3-vl-235b-a22b-instruct",
            constructor_name: deepinfra_qwen3_vl_235b_a22b_instruct,
            display_name: "Qwen3 VL 235B A22B Instruct (DeepInfra)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepinfraQwen3Vl30bA3bInstruct {
            model_name: "deepinfra/qwen3-vl-30b-a3b-instruct",
            constructor_name: deepinfra_qwen3_vl_30b_a3b_instruct,
            display_name: "Qwen3 VL 30B A3B Instruct (DeepInfra)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DeepinfraQwen359b {
            model_name: "deepinfra/qwen3.5-9b",
            constructor_name: deepinfra_qwen3_5_9b,
            display_name: "Qwen3.5 9B (DeepInfra)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DeepseekDeepseekV4Pro {
            model_name: "deepseek/deepseek-v4-pro",
            constructor_name: deepseek_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro (DeepSeek)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV41Flash {
            model_name: "deepseek/deepseek-v4.1-flash",
            constructor_name: deepseek_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (DeepSeek)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV41Flash1017 {
            model_name: "deepseek/deepseek-v4.1-flash-1017",
            constructor_name: deepseek_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (DeepSeek)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EmbercloudGlm45 {
            model_name: "embercloud/glm-4.5",
            constructor_name: embercloud_glm_4_5,
            display_name: "GLM-4.5 (EmberCloud)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EmbercloudGlm45Air {
            model_name: "embercloud/glm-4.5-air",
            constructor_name: embercloud_glm_4_5_air,
            display_name: "GLM-4.5 Air (EmberCloud)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EmbercloudGlm47 {
            model_name: "embercloud/glm-4.7",
            constructor_name: embercloud_glm_4_7,
            display_name: "GLM-4.7 (EmberCloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EmbercloudGlm47Flash {
            model_name: "embercloud/glm-4.7-flash",
            constructor_name: embercloud_glm_4_7_flash,
            display_name: "GLM-4.7 Flash (EmberCloud)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EmbercloudGlm5 {
            model_name: "embercloud/glm-5",
            constructor_name: embercloud_glm_5,
            display_name: "GLM-5 (EmberCloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EmbercloudGlm51 {
            model_name: "embercloud/glm-5.1",
            constructor_name: embercloud_glm_5_1,
            display_name: "GLM-5.1 (EmberCloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EmbercloudGlm52 {
            model_name: "embercloud/glm-5.2",
            constructor_name: embercloud_glm_5_2,
            display_name: "GLM-5.2 (EmberCloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EmbercloudKimiK25 {
            model_name: "embercloud/kimi-k2.5",
            constructor_name: embercloud_kimi_k2_5,
            display_name: "Kimi K2.5 (EmberCloud)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EmbercloudQwen3CoderNext {
            model_name: "embercloud/qwen3-coder-next",
            constructor_name: embercloud_qwen3_coder_next,
            display_name: "Qwen3 Coder Next (EmberCloud)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EntrimDeepseekV41Flash {
            model_name: "entrim/deepseek-v4.1-flash",
            constructor_name: entrim_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (entrim)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EntrimGlm53 {
            model_name: "entrim/glm-5.3",
            constructor_name: entrim_glm_5_3,
            display_name: "GLM-5.3 (entrim)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EntrimGlm53Flash {
            model_name: "entrim/glm-5.3-flash",
            constructor_name: entrim_glm_5_3_flash,
            display_name: "GLM-5.3 Flash (entrim)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        FireworksDeepseekV41Flash {
            model_name: "fireworks/deepseek-v4.1-flash",
            constructor_name: fireworks_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (Fireworks AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FireworksGlm53 {
            model_name: "fireworks/glm-5.3",
            constructor_name: fireworks_glm_5_3,
            display_name: "GLM-5.3 (Fireworks AI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        FireworksGlm53Flash {
            model_name: "fireworks/glm-5.3-flash",
            constructor_name: fireworks_glm_5_3_flash,
            display_name: "GLM-5.3 Flash (Fireworks AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        FireworksKimiK3 {
            model_name: "fireworks/kimi-k3",
            constructor_name: fireworks_kimi_k3,
            display_name: "Kimi K3 (Fireworks AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        FireworksKimiK3Fast {
            model_name: "fireworks/kimi-k3-fast",
            constructor_name: fireworks_kimi_k3_fast,
            display_name: "Kimi K3 Fast (Fireworks AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Gonka24MinimaxM27 {
            model_name: "gonka24/minimax-m2.7",
            constructor_name: gonka24_minimax_m2_7,
            display_name: "MiniMax M2.7 (Gonka24)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleAiStudioGemini3FlashPreview {
            model_name: "google-ai-studio/gemini-3-flash-preview",
            constructor_name: google_ai_studio_gemini_3_flash_preview,
            display_name: "Gemini 3 Flash (Preview) (Google AI Studio)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleAiStudioGemini31FlashLite {
            model_name: "google-ai-studio/gemini-3.1-flash-lite",
            constructor_name: google_ai_studio_gemini_3_1_flash_lite,
            display_name: "Gemini 3.1 Flash Lite (Google AI Studio)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleAiStudioGemini31ProPreview {
            model_name: "google-ai-studio/gemini-3.1-pro-preview",
            constructor_name: google_ai_studio_gemini_3_1_pro_preview,
            display_name: "Gemini 3.1 Pro (Preview) (Google AI Studio)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleAiStudioGemini35FlashLite {
            model_name: "google-ai-studio/gemini-3.5-flash-lite",
            constructor_name: google_ai_studio_gemini_3_5_flash_lite,
            display_name: "Gemini 3.5 Flash Lite (Google AI Studio)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleAiStudioGemini36Flash {
            model_name: "google-ai-studio/gemini-3.6-flash",
            constructor_name: google_ai_studio_gemini_3_6_flash,
            display_name: "Gemini 3.6 Flash (Google AI Studio)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleAiStudioGemini38Flash {
            model_name: "google-ai-studio/gemini-3.8-flash",
            constructor_name: google_ai_studio_gemini_3_8_flash,
            display_name: "Gemini 3.8 Flash (Google AI Studio)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleAiStudioGemini4Argon {
            model_name: "google-ai-studio/gemini-4-argon",
            constructor_name: google_ai_studio_gemini_4_argon,
            display_name: "Gemini 4 Argon (Google AI Studio)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleAiStudioGeminiProLatest {
            model_name: "google-ai-studio/gemini-pro-latest",
            constructor_name: google_ai_studio_gemini_pro_latest,
            display_name: "Gemini Pro Latest (Google AI Studio)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleVertexGemini3FlashPreview {
            model_name: "google-vertex/gemini-3-flash-preview",
            constructor_name: google_vertex_gemini_3_flash_preview,
            display_name: "Gemini 3 Flash (Preview) (Google Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleVertexGemini31FlashLite {
            model_name: "google-vertex/gemini-3.1-flash-lite",
            constructor_name: google_vertex_gemini_3_1_flash_lite,
            display_name: "Gemini 3.1 Flash Lite (Google Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleVertexGemini31ProPreview {
            model_name: "google-vertex/gemini-3.1-pro-preview",
            constructor_name: google_vertex_gemini_3_1_pro_preview,
            display_name: "Gemini 3.1 Pro (Preview) (Google Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleVertexGemini35Flash {
            model_name: "google-vertex/gemini-3.5-flash",
            constructor_name: google_vertex_gemini_3_5_flash,
            display_name: "Gemini 3.5 Flash (Google Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleVertexGemini35FlashLite {
            model_name: "google-vertex/gemini-3.5-flash-lite",
            constructor_name: google_vertex_gemini_3_5_flash_lite,
            display_name: "Gemini 3.5 Flash Lite (Google Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleVertexGemini36Flash {
            model_name: "google-vertex/gemini-3.6-flash",
            constructor_name: google_vertex_gemini_3_6_flash,
            display_name: "Gemini 3.6 Flash (Google Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleVertexGemini37Flash {
            model_name: "google-vertex/gemini-3.7-flash",
            constructor_name: google_vertex_gemini_3_7_flash,
            display_name: "Gemini 3.7 Flash (Google Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleVertexGemini38Flash {
            model_name: "google-vertex/gemini-3.8-flash",
            constructor_name: google_vertex_gemini_3_8_flash,
            display_name: "Gemini 3.8 Flash (Google Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleVertexGemini4Argon {
            model_name: "google-vertex/gemini-4-argon",
            constructor_name: google_vertex_gemini_4_argon,
            display_name: "Gemini 4 Argon (Google Vertex AI)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GreenferenceGlm53Flash {
            model_name: "greenference/glm-5.3-flash",
            constructor_name: greenference_glm_5_3_flash,
            display_name: "GLM-5.3 Flash (greenference)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GreenferenceQwen314b {
            model_name: "greenference/qwen3-14b",
            constructor_name: greenference_qwen3_14b,
            display_name: "Qwen3 14B (greenference)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GreenferenceQwen330bA3b {
            model_name: "greenference/qwen3-30b-a3b",
            constructor_name: greenference_qwen3_30b_a3b,
            display_name: "Qwen3 30B A3B (greenference)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        GreenferenceQwen3627b {
            model_name: "greenference/qwen3.6-27b",
            constructor_name: greenference_qwen3_6_27b,
            display_name: "Qwen3.6 27B (greenference)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GroqGptOss120b {
            model_name: "groq/gpt-oss-120b",
            constructor_name: groq_gpt_oss_120b,
            display_name: "GPT OSS 120B (Groq)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GroqGptOss20b {
            model_name: "groq/gpt-oss-20b",
            constructor_name: groq_gpt_oss_20b,
            display_name: "GPT OSS 20B (Groq)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        InferenceNetDeepseekV41Flash1017 {
            model_name: "inference.net/deepseek-v4.1-flash-1017",
            constructor_name: inference_net_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (Inference.net)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        InferenceNetGlm53 {
            model_name: "inference.net/glm-5.3",
            constructor_name: inference_net_glm_5_3,
            display_name: "GLM-5.3 (Inference.net)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        InferenceNetGlm53Flash {
            model_name: "inference.net/glm-5.3-flash",
            constructor_name: inference_net_glm_5_3_flash,
            display_name: "GLM-5.3 Flash (Inference.net)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        InferenceNetLlama3211bInstruct {
            model_name: "inference.net/llama-3.2-11b-instruct",
            constructor_name: inference_net_llama_3_2_11b_instruct,
            display_name: "Llama 3.2 11B Instruct (Inference.net)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        LuminalDeepseekV41Flash {
            model_name: "luminal/deepseek-v4.1-flash",
            constructor_name: luminal_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (luminal)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaContributorMuseSpark12Contributor {
            model_name: "meta-contributor/muse-spark-1.2-contributor",
            constructor_name: meta_contributor_muse_spark_1_2_contributor,
            display_name: "Muse Spark 1.2 Contributor (Meta Contributor)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaContributorMuseSpark13Contributor {
            model_name: "meta-contributor/muse-spark-1.3-contributor",
            constructor_name: meta_contributor_muse_spark_1_3_contributor,
            display_name: "Muse Spark 1.3 Contributor (Meta Contributor)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaMuseSpark11 {
            model_name: "meta/muse-spark-1.1",
            constructor_name: meta_muse_spark_1_1,
            display_name: "Muse Spark 1.1 (Meta)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MetaMuseSpark12 {
            model_name: "meta/muse-spark-1.2",
            constructor_name: meta_muse_spark_1_2,
            display_name: "Muse Spark 1.2 (Meta)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MetaMuseSpark13 {
            model_name: "meta/muse-spark-1.3",
            constructor_name: meta_muse_spark_1_3,
            display_name: "Muse Spark 1.3 (Meta)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MetaMuseWonder10 {
            model_name: "meta/muse-wonder-1.0",
            constructor_name: meta_muse_wonder_1_0,
            display_name: "Muse Wonder 1.0 (Meta)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MinimaxMinimaxM2 {
            model_name: "minimax/minimax-m2",
            constructor_name: minimax_minimax_m2,
            display_name: "MiniMax M2 (MiniMax)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM21 {
            model_name: "minimax/minimax-m2.1",
            constructor_name: minimax_minimax_m2_1,
            display_name: "MiniMax M2.1 (MiniMax)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM21Lightning {
            model_name: "minimax/minimax-m2.1-lightning",
            constructor_name: minimax_minimax_m2_1_lightning,
            display_name: "MiniMax M2.1 Lightning (MiniMax)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM25 {
            model_name: "minimax/minimax-m2.5",
            constructor_name: minimax_minimax_m2_5,
            display_name: "MiniMax M2.5 (MiniMax)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM25Highspeed {
            model_name: "minimax/minimax-m2.5-highspeed",
            constructor_name: minimax_minimax_m2_5_highspeed,
            display_name: "MiniMax M2.5 Highspeed (MiniMax)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM27 {
            model_name: "minimax/minimax-m2.7",
            constructor_name: minimax_minimax_m2_7,
            display_name: "MiniMax M2.7 (MiniMax)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM27Highspeed {
            model_name: "minimax/minimax-m2.7-highspeed",
            constructor_name: minimax_minimax_m2_7_highspeed,
            display_name: "MiniMax M2.7 Highspeed (MiniMax)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM3 {
            model_name: "minimax/minimax-m3",
            constructor_name: minimax_minimax_m3,
            display_name: "MiniMax M3 (MiniMax)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MinimaxMinimaxM31Flash {
            model_name: "minimax/minimax-m3.1-flash",
            constructor_name: minimax_minimax_m3_1_flash,
            display_name: "MiniMax M3.1 Flash (MiniMax)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MinimaxMinimaxText01 {
            model_name: "minimax/minimax-text-01",
            constructor_name: minimax_minimax_text_01,
            display_name: "MiniMax Text 01 (MiniMax)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralCodestral2508 {
            model_name: "mistral/codestral-2508",
            constructor_name: mistral_codestral_2508,
            display_name: "Codestral (Mistral AI)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        MistralDevstral2512 {
            model_name: "mistral/devstral-2512",
            constructor_name: mistral_devstral_2512,
            display_name: "Devstral 2 (Mistral AI)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        MistralGlm53 {
            model_name: "mistral/glm-5.3",
            constructor_name: mistral_glm_5_3,
            display_name: "GLM-5.3 (Mistral AI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMinistral14b2512 {
            model_name: "mistral/ministral-14b-2512",
            constructor_name: mistral_ministral_14b_2512,
            display_name: "Ministral 14B (Mistral AI)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        MistralMinistral3b2512 {
            model_name: "mistral/ministral-3b-2512",
            constructor_name: mistral_ministral_3b_2512,
            display_name: "Ministral 3B (Mistral AI)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        MistralMinistral8b2512 {
            model_name: "mistral/ministral-8b-2512",
            constructor_name: mistral_ministral_8b_2512,
            display_name: "Ministral 8B (Mistral AI)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        MistralMistralLarge2512 {
            model_name: "mistral/mistral-large-2512",
            constructor_name: mistral_mistral_large_2512,
            display_name: "Mistral Large 3 (Mistral AI)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        MistralMistralLarge4 {
            model_name: "mistral/mistral-large-4",
            constructor_name: mistral_mistral_large_4,
            display_name: "Mistral Large 4 (Mistral AI)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMistralLargeLatest {
            model_name: "mistral/mistral-large-latest",
            constructor_name: mistral_mistral_large_latest,
            display_name: "Mistral Large Latest (Mistral AI)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        MistralMistralMedium35 {
            model_name: "mistral/mistral-medium-3-5",
            constructor_name: mistral_mistral_medium_3_5,
            display_name: "Mistral Medium 3.5 (Mistral AI)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMistralSmall2506 {
            model_name: "mistral/mistral-small-2506",
            constructor_name: mistral_mistral_small_2506,
            display_name: "Mistral Small 3.2 (Mistral AI)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        MistralMistralSmall2603 {
            model_name: "mistral/mistral-small-2603",
            constructor_name: mistral_mistral_small_2603,
            display_name: "Mistral Small 4 (Mistral AI)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotKimiK25 {
            model_name: "moonshot/kimi-k2.5",
            constructor_name: moonshot_kimi_k2_5,
            display_name: "Kimi K2.5 (Moonshot AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MoonshotKimiK26 {
            model_name: "moonshot/kimi-k2.6",
            constructor_name: moonshot_kimi_k2_6,
            display_name: "Kimi K2.6 (Moonshot AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MoonshotKimiK27Code {
            model_name: "moonshot/kimi-k2.7-code",
            constructor_name: moonshot_kimi_k2_7_code,
            display_name: "Kimi K2.7 Code (Moonshot AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MoonshotKimiK27CodeHighspeed {
            model_name: "moonshot/kimi-k2.7-code-highspeed",
            constructor_name: moonshot_kimi_k2_7_code_highspeed,
            display_name: "Kimi K2.7 Code Highspeed (Moonshot AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MoonshotKimiK3 {
            model_name: "moonshot/kimi-k3",
            constructor_name: moonshot_kimi_k3,
            display_name: "Kimi K3 (Moonshot AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NovitaDeepseekV32 {
            model_name: "novita/deepseek-v3.2",
            constructor_name: novita_deepseek_v3_2,
            display_name: "DeepSeek V3.2 (NovitaAI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaDeepseekV4Flash {
            model_name: "novita/deepseek-v4-flash",
            constructor_name: novita_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash (NovitaAI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaDeepseekV41Flash {
            model_name: "novita/deepseek-v4.1-flash",
            constructor_name: novita_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaDeepseekV41Flash1017 {
            model_name: "novita/deepseek-v4.1-flash-1017",
            constructor_name: novita_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaErnie45Vl424bA47b {
            model_name: "novita/ernie-4.5-vl-424b-a47b",
            constructor_name: novita_ernie_4_5_vl_424b_a47b,
            display_name: "ERNIE 4.5 VL 424B A47B (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        NovitaGemma426bA4bIt {
            model_name: "novita/gemma-4-26b-a4b-it",
            constructor_name: novita_gemma_4_26b_a4b_it,
            display_name: "Gemma 4 26B A4B IT (NovitaAI)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaGemma431bIt {
            model_name: "novita/gemma-4-31b-it",
            constructor_name: novita_gemma_4_31b_it,
            display_name: "Gemma 4 31B IT (NovitaAI)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaGlm45v {
            model_name: "novita/glm-4.5v",
            constructor_name: novita_glm_4_5v,
            display_name: "GLM-4.5V (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NovitaGlm46 {
            model_name: "novita/glm-4.6",
            constructor_name: novita_glm_4_6,
            display_name: "GLM-4.6 (NovitaAI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaGlm46v {
            model_name: "novita/glm-4.6v",
            constructor_name: novita_glm_4_6v,
            display_name: "GLM-4.6V (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NovitaGlm47 {
            model_name: "novita/glm-4.7",
            constructor_name: novita_glm_4_7,
            display_name: "GLM-4.7 (NovitaAI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaGlm5 {
            model_name: "novita/glm-5",
            constructor_name: novita_glm_5,
            display_name: "GLM-5 (NovitaAI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaGlm51 {
            model_name: "novita/glm-5.1",
            constructor_name: novita_glm_5_1,
            display_name: "GLM-5.1 (NovitaAI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaGlm52 {
            model_name: "novita/glm-5.2",
            constructor_name: novita_glm_5_2,
            display_name: "GLM-5.2 (NovitaAI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaGlm53 {
            model_name: "novita/glm-5.3",
            constructor_name: novita_glm_5_3,
            display_name: "GLM-5.3 (NovitaAI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaGlm53Flash {
            model_name: "novita/glm-5.3-flash",
            constructor_name: novita_glm_5_3_flash,
            display_name: "GLM-5.3 Flash (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NovitaHy3 {
            model_name: "novita/hy3",
            constructor_name: novita_hy3,
            display_name: "Hy3 (NovitaAI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaKimiK2 {
            model_name: "novita/kimi-k2",
            constructor_name: novita_kimi_k2,
            display_name: "Kimi K2 (NovitaAI)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaKimiK26 {
            model_name: "novita/kimi-k2.6",
            constructor_name: novita_kimi_k2_6,
            display_name: "Kimi K2.6 (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NovitaKimiK27Code {
            model_name: "novita/kimi-k2.7-code",
            constructor_name: novita_kimi_k2_7_code,
            display_name: "Kimi K2.7 Code (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NovitaKimiK3 {
            model_name: "novita/kimi-k3",
            constructor_name: novita_kimi_k3,
            display_name: "Kimi K3 (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NovitaLing30Flash {
            model_name: "novita/ling-3.0-flash",
            constructor_name: novita_ling_3_0_flash,
            display_name: "InclusionAI Ling 3.0 Flash (NovitaAI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaLlama370bInstruct {
            model_name: "novita/llama-3-70b-instruct",
            constructor_name: novita_llama_3_70b_instruct,
            display_name: "Llama 3 70B Instruct (NovitaAI)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        NovitaLlama323bInstruct {
            model_name: "novita/llama-3.2-3b-instruct",
            constructor_name: novita_llama_3_2_3b_instruct,
            display_name: "Llama 3.2 3B Instruct (NovitaAI)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        NovitaLlama3370bInstruct {
            model_name: "novita/llama-3.3-70b-instruct",
            constructor_name: novita_llama_3_3_70b_instruct,
            display_name: "Llama 3.3 70B Instruct (NovitaAI)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaLlama4Maverick17bInstruct {
            model_name: "novita/llama-4-maverick-17b-instruct",
            constructor_name: novita_llama_4_maverick_17b_instruct,
            display_name: "Llama 4 Maverick 17B Instruct (NovitaAI)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        NovitaLlama4Scout17bInstruct {
            model_name: "novita/llama-4-scout-17b-instruct",
            constructor_name: novita_llama_4_scout_17b_instruct,
            display_name: "Llama 4 Scout 17B Instruct (NovitaAI)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        NovitaMinimaxM21 {
            model_name: "novita/minimax-m2.1",
            constructor_name: novita_minimax_m2_1,
            display_name: "MiniMax M2.1 (NovitaAI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaMinimaxM25 {
            model_name: "novita/minimax-m2.5",
            constructor_name: novita_minimax_m2_5,
            display_name: "MiniMax M2.5 (NovitaAI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaMinimaxM27 {
            model_name: "novita/minimax-m2.7",
            constructor_name: novita_minimax_m2_7,
            display_name: "MiniMax M2.7 (NovitaAI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaQwen3235bA22bFp8 {
            model_name: "novita/qwen3-235b-a22b-fp8",
            constructor_name: novita_qwen3_235b_a22b_fp8,
            display_name: "Qwen3 235B A22B FP8 (NovitaAI)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        NovitaQwen3235bA22bInstruct2507 {
            model_name: "novita/qwen3-235b-a22b-instruct-2507",
            constructor_name: novita_qwen3_235b_a22b_instruct_2507,
            display_name: "Qwen3 235B A22B Instruct 2507 (NovitaAI)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaQwen3235bA22bThinking2507 {
            model_name: "novita/qwen3-235b-a22b-thinking-2507",
            constructor_name: novita_qwen3_235b_a22b_thinking_2507,
            display_name: "Qwen3 235B A22B Thinking 2507 (NovitaAI)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaQwen3Coder30bA3bInstruct {
            model_name: "novita/qwen3-coder-30b-a3b-instruct",
            constructor_name: novita_qwen3_coder_30b_a3b_instruct,
            display_name: "Qwen3 Coder 30B A3B Instruct (NovitaAI)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaQwen3Coder480bA35bInstruct {
            model_name: "novita/qwen3-coder-480b-a35b-instruct",
            constructor_name: novita_qwen3_coder_480b_a35b_instruct,
            display_name: "Qwen3 Coder 480B A35B Instruct (NovitaAI)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaQwen3Max {
            model_name: "novita/qwen3-max",
            constructor_name: novita_qwen3_max,
            display_name: "Qwen3 Max (NovitaAI)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaQwen3Next80bA3bInstruct {
            model_name: "novita/qwen3-next-80b-a3b-instruct",
            constructor_name: novita_qwen3_next_80b_a3b_instruct,
            display_name: "Qwen3 Next 80B A3B Instruct (NovitaAI)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaQwen3Vl235bA22bInstruct {
            model_name: "novita/qwen3-vl-235b-a22b-instruct",
            constructor_name: novita_qwen3_vl_235b_a22b_instruct,
            display_name: "Qwen3 VL 235B A22B Instruct (NovitaAI)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaQwen3Vl235bA22bThinking {
            model_name: "novita/qwen3-vl-235b-a22b-thinking",
            constructor_name: novita_qwen3_vl_235b_a22b_thinking,
            display_name: "Qwen3 VL 235B A22B Thinking (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        NovitaQwen3Vl30bA3bInstruct {
            model_name: "novita/qwen3-vl-30b-a3b-instruct",
            constructor_name: novita_qwen3_vl_30b_a3b_instruct,
            display_name: "Qwen3 VL 30B A3B Instruct (NovitaAI)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NovitaQwen35122bA10b {
            model_name: "novita/qwen3.5-122b-a10b",
            constructor_name: novita_qwen3_5_122b_a10b,
            display_name: "Qwen3.5 122B A10B (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NovitaQwen3635bA3b {
            model_name: "novita/qwen3.6-35b-a3b",
            constructor_name: novita_qwen3_6_35b_a3b,
            display_name: "Qwen3.6 35B A3B (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NovitaQwen37Max {
            model_name: "novita/qwen3.7-max",
            constructor_name: novita_qwen3_7_max,
            display_name: "Qwen3.7 Max (NovitaAI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaQwen3824tA95b {
            model_name: "novita/qwen3.8-2.4t-a95b",
            constructor_name: novita_qwen3_8_2_4t_a95b,
            display_name: "Qwen3.8 2.4T A95B (NovitaAI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaQwen3827b {
            model_name: "novita/qwen3.8-27b",
            constructor_name: novita_qwen3_8_27b,
            display_name: "Qwen3.8 27B (NovitaAI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaQwen38Flash {
            model_name: "novita/qwen3.8-flash",
            constructor_name: novita_qwen3_8_flash,
            display_name: "Qwen3.8 Flash (NovitaAI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaQwen38Max {
            model_name: "novita/qwen3.8-max",
            constructor_name: novita_qwen3_8_max,
            display_name: "Qwen3.8 Max (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NovitaQwen35397bA17b {
            model_name: "novita/qwen35-397b-a17b",
            constructor_name: novita_qwen35_397b_a17b,
            display_name: "Qwen3.5 397B A17B (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NovitaStep37Flash {
            model_name: "novita/step-3.7-flash",
            constructor_name: novita_step_3_7_flash,
            display_name: "Step 3.7 Flash (NovitaAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        OpenaiGpt41 {
            model_name: "openai/gpt-4.1",
            constructor_name: openai_gpt_4_1,
            display_name: "GPT-4.1 (OpenAI)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt41Mini {
            model_name: "openai/gpt-4.1-mini",
            constructor_name: openai_gpt_4_1_mini,
            display_name: "GPT-4.1 Mini (OpenAI)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4o {
            model_name: "openai/gpt-4o",
            constructor_name: openai_gpt_4o,
            display_name: "GPT-4o (OpenAI)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4oMini {
            model_name: "openai/gpt-4o-mini",
            constructor_name: openai_gpt_4o_mini,
            display_name: "GPT-4o Mini (OpenAI)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4oMiniTranscribe {
            model_name: "openai/gpt-4o-mini-transcribe",
            constructor_name: openai_gpt_4o_mini_transcribe,
            display_name: "GPT-4o Mini Transcribe (OpenAI)",
            capabilities: [AudioInputSupport, ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        OpenaiGpt4oTranscribe {
            model_name: "openai/gpt-4o-transcribe",
            constructor_name: openai_gpt_4o_transcribe,
            display_name: "GPT-4o Transcribe (OpenAI)",
            capabilities: [AudioInputSupport, ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        OpenaiGpt5 {
            model_name: "openai/gpt-5",
            constructor_name: openai_gpt_5,
            display_name: "GPT-5 (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5Mini {
            model_name: "openai/gpt-5-mini",
            constructor_name: openai_gpt_5_mini,
            display_name: "GPT-5 Mini (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5Nano {
            model_name: "openai/gpt-5-nano",
            constructor_name: openai_gpt_5_nano,
            display_name: "GPT-5 Nano (OpenAI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5Pro {
            model_name: "openai/gpt-5-pro",
            constructor_name: openai_gpt_5_pro,
            display_name: "GPT-5 Pro (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt51 {
            model_name: "openai/gpt-5.1",
            constructor_name: openai_gpt_5_1,
            display_name: "GPT-5.1 (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt52 {
            model_name: "openai/gpt-5.2",
            constructor_name: openai_gpt_5_2,
            display_name: "GPT-5.2 (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt52Pro {
            model_name: "openai/gpt-5.2-pro",
            constructor_name: openai_gpt_5_2_pro,
            display_name: "GPT-5.2 Pro (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt53Codex {
            model_name: "openai/gpt-5.3-codex",
            constructor_name: openai_gpt_5_3_codex,
            display_name: "GPT-5.3 Codex (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54 {
            model_name: "openai/gpt-5.4",
            constructor_name: openai_gpt_5_4,
            display_name: "GPT-5.4 (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Mini {
            model_name: "openai/gpt-5.4-mini",
            constructor_name: openai_gpt_5_4_mini,
            display_name: "GPT-5.4 Mini (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Nano {
            model_name: "openai/gpt-5.4-nano",
            constructor_name: openai_gpt_5_4_nano,
            display_name: "GPT-5.4 Nano (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Pro {
            model_name: "openai/gpt-5.4-pro",
            constructor_name: openai_gpt_5_4_pro,
            display_name: "GPT-5.4 Pro (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt55 {
            model_name: "openai/gpt-5.5",
            constructor_name: openai_gpt_5_5,
            display_name: "GPT-5.5 (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt55Pro {
            model_name: "openai/gpt-5.5-pro",
            constructor_name: openai_gpt_5_5_pro,
            display_name: "GPT-5.5 Pro (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Luna {
            model_name: "openai/gpt-5.6-luna",
            constructor_name: openai_gpt_5_6_luna,
            display_name: "GPT-5.6 Luna (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Sol {
            model_name: "openai/gpt-5.6-sol",
            constructor_name: openai_gpt_5_6_sol,
            display_name: "GPT-5.6 Sol (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Terra {
            model_name: "openai/gpt-5.6-terra",
            constructor_name: openai_gpt_5_6_terra,
            display_name: "GPT-5.6 Terra (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Astra {
            model_name: "openai/gpt-6-astra",
            constructor_name: openai_gpt_6_astra,
            display_name: "GPT-6 Astra (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Luna {
            model_name: "openai/gpt-6-luna",
            constructor_name: openai_gpt_6_luna,
            display_name: "GPT-6 Luna (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Sol {
            model_name: "openai/gpt-6-sol",
            constructor_name: openai_gpt_6_sol,
            display_name: "GPT-6 Sol (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt61Sol {
            model_name: "openai/gpt-6.1-sol",
            constructor_name: openai_gpt_6_1_sol,
            display_name: "GPT-6.1 Sol (OpenAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptLiveTranscribe {
            model_name: "openai/gpt-live-transcribe",
            constructor_name: openai_gpt_live_transcribe,
            display_name: "GPT Live Transcribe (OpenAI)",
            capabilities: [AudioInputSupport, ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        OpenaiGptTranscribe {
            model_name: "openai/gpt-transcribe",
            constructor_name: openai_gpt_transcribe,
            display_name: "GPT Transcribe (OpenAI)",
            capabilities: [AudioInputSupport, ImageInputSupport, TextInputSupport, TextOutputSupport]
        },
        OpenaiO3 {
            model_name: "openai/o3",
            constructor_name: openai_o3,
            display_name: "o3 (OpenAI)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        PerplexitySonar {
            model_name: "perplexity/sonar",
            constructor_name: perplexity_sonar,
            display_name: "Sonar (Perplexity)",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        QuartzGemini31ProPreview {
            model_name: "quartz/gemini-3.1-pro-preview",
            constructor_name: quartz_gemini_3_1_pro_preview,
            display_name: "Gemini 3.1 Pro (Preview) (Quartz)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        RanoaiDeepseekV4Flash {
            model_name: "ranoai/deepseek-v4-flash",
            constructor_name: ranoai_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash (RanoAI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        RekaAiDeepseekV4Pro {
            model_name: "reka-ai/deepseek-v4-pro",
            constructor_name: reka_ai_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro (reka-ai)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        RekaAiDeepseekV41Flash {
            model_name: "reka-ai/deepseek-v4.1-flash",
            constructor_name: reka_ai_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (reka-ai)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        RekaAiDeepseekV41Flash1017 {
            model_name: "reka-ai/deepseek-v4.1-flash-1017",
            constructor_name: reka_ai_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (reka-ai)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        RekaAiGlm53 {
            model_name: "reka-ai/glm-5.3",
            constructor_name: reka_ai_glm_5_3,
            display_name: "GLM-5.3 (reka-ai)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        RekaAiGlm53Flash {
            model_name: "reka-ai/glm-5.3-flash",
            constructor_name: reka_ai_glm_5_3_flash,
            display_name: "GLM-5.3 Flash (reka-ai)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        RekaAiRekaEdge2603 {
            model_name: "reka-ai/reka-edge-2603",
            constructor_name: reka_ai_reka_edge_2603,
            display_name: "Reka Edge 2603 (reka-ai)",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        RunpodKimiK3 {
            model_name: "runpod/kimi-k3",
            constructor_name: runpod_kimi_k3,
            display_name: "Kimi K3 (Runpod)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        RunwareDeepseekV4Flash {
            model_name: "runware/deepseek-v4-flash",
            constructor_name: runware_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash (Runware)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        RunwareDeepseekV4Pro {
            model_name: "runware/deepseek-v4-pro",
            constructor_name: runware_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro (Runware)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        RunwareDeepseekV41Flash {
            model_name: "runware/deepseek-v4.1-flash",
            constructor_name: runware_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (Runware)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        RunwareGemma431bIt {
            model_name: "runware/gemma-4-31b-it",
            constructor_name: runware_gemma_4_31b_it,
            display_name: "Gemma 4 31B IT (Runware)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        RunwareGlm52 {
            model_name: "runware/glm-5.2",
            constructor_name: runware_glm_5_2,
            display_name: "GLM-5.2 (Runware)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        RunwareGlm53 {
            model_name: "runware/glm-5.3",
            constructor_name: runware_glm_5_3,
            display_name: "GLM-5.3 (Runware)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        RunwareGlm53Flash {
            model_name: "runware/glm-5.3-flash",
            constructor_name: runware_glm_5_3_flash,
            display_name: "GLM-5.3 Flash (Runware)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        RunwareGptOss120b {
            model_name: "runware/gpt-oss-120b",
            constructor_name: runware_gpt_oss_120b,
            display_name: "GPT OSS 120B (Runware)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        RunwareKimiK26 {
            model_name: "runware/kimi-k2.6",
            constructor_name: runware_kimi_k2_6,
            display_name: "Kimi K2.6 (Runware)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        RunwareKimiK3 {
            model_name: "runware/kimi-k3",
            constructor_name: runware_kimi_k3,
            display_name: "Kimi K3 (Runware)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        SakanaFuguMax {
            model_name: "sakana/fugu-max",
            constructor_name: sakana_fugu_max,
            display_name: "Fugu Max (Sakana AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        SakanaFuguUltra {
            model_name: "sakana/fugu-ultra",
            constructor_name: sakana_fugu_ultra,
            display_name: "Fugu Ultra (Sakana AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        SakanaFuguUltraV20 {
            model_name: "sakana/fugu-ultra-v2.0",
            constructor_name: sakana_fugu_ultra_v2_0,
            display_name: "Fugu Ultra v2.0 (Sakana AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ScxAiGpDeepseekV41Flash {
            model_name: "scx-ai-gp/deepseek-v4.1-flash",
            constructor_name: scx_ai_gp_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (SCX.ai)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        ScxAiGpGlm52 {
            model_name: "scx-ai-gp/glm-5.2",
            constructor_name: scx_ai_gp_glm_5_2,
            display_name: "GLM-5.2 (SCX.ai)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ScxAiGpGlm52Fast {
            model_name: "scx-ai-gp/glm-5.2-fast",
            constructor_name: scx_ai_gp_glm_5_2_fast,
            display_name: "GLM-5.2 Turbo (SCX.ai)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ScxAiGpGlm53 {
            model_name: "scx-ai-gp/glm-5.3",
            constructor_name: scx_ai_gp_glm_5_3,
            display_name: "GLM-5.3 (SCX.ai)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ScxAiGpGlm53Fast {
            model_name: "scx-ai-gp/glm-5.3-fast",
            constructor_name: scx_ai_gp_glm_5_3_fast,
            display_name: "GLM 5.3 Fast (SCX.ai)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        ScxAiGpGlm53Flash {
            model_name: "scx-ai-gp/glm-5.3-flash",
            constructor_name: scx_ai_gp_glm_5_3_flash,
            display_name: "GLM-5.3 Flash (SCX.ai)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ScxAiGpKimiK27Code {
            model_name: "scx-ai-gp/kimi-k2.7-code",
            constructor_name: scx_ai_gp_kimi_k2_7_code,
            display_name: "Kimi K2.7 Code (SCX.ai)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ScxAiGpKimiK3 {
            model_name: "scx-ai-gp/kimi-k3",
            constructor_name: scx_ai_gp_kimi_k3,
            display_name: "Kimi K3 (SCX.ai)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ScxAiGpQwen38Max {
            model_name: "scx-ai-gp/qwen3.8-max",
            constructor_name: scx_ai_gp_qwen3_8_max,
            display_name: "Qwen3.8 Max (SCX.ai)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ScxAiGptOss120b {
            model_name: "scx-ai/gpt-oss-120b",
            constructor_name: scx_ai_gpt_oss_120b,
            display_name: "GPT OSS 120B (SCX.ai (Turbo))",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ScxAiMinimaxM27 {
            model_name: "scx-ai/minimax-m2.7",
            constructor_name: scx_ai_minimax_m2_7,
            display_name: "MiniMax M2.7 (SCX.ai (Turbo))",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ScxAiQwen332b {
            model_name: "scx-ai/qwen3-32b",
            constructor_name: scx_ai_qwen3_32b,
            display_name: "Qwen3 32B (SCX.ai (Turbo))",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentDeepseekV4Flash {
            model_name: "tencent/deepseek-v4-flash",
            constructor_name: tencent_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash (Tencent Cloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentDeepseekV4Pro {
            model_name: "tencent/deepseek-v4-pro",
            constructor_name: tencent_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro (Tencent Cloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentGlm5 {
            model_name: "tencent/glm-5",
            constructor_name: tencent_glm_5,
            display_name: "GLM-5 (Tencent Cloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentGlm5Turbo {
            model_name: "tencent/glm-5-turbo",
            constructor_name: tencent_glm_5_turbo,
            display_name: "GLM-5 Turbo (Tencent Cloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentGlm51 {
            model_name: "tencent/glm-5.1",
            constructor_name: tencent_glm_5_1,
            display_name: "GLM-5.1 (Tencent Cloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentGlm52 {
            model_name: "tencent/glm-5.2",
            constructor_name: tencent_glm_5_2,
            display_name: "GLM-5.2 (Tencent Cloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentGlm5vTurbo {
            model_name: "tencent/glm-5v-turbo",
            constructor_name: tencent_glm_5v_turbo,
            display_name: "GLM-5V Turbo (Tencent Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        TencentHyMt2Plus {
            model_name: "tencent/hy-mt2-plus",
            constructor_name: tencent_hy_mt2_plus,
            display_name: "Hy-MT2 Plus (Tencent Cloud)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        TencentHy3 {
            model_name: "tencent/hy3",
            constructor_name: tencent_hy3,
            display_name: "Hy3 (Tencent Cloud)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentHy4Preview {
            model_name: "tencent/hy4-preview",
            constructor_name: tencent_hy4_preview,
            display_name: "Hy4 Preview (Tencent Cloud)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentKimiK26 {
            model_name: "tencent/kimi-k2.6",
            constructor_name: tencent_kimi_k2_6,
            display_name: "Kimi K2.6 (Tencent Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        TencentKimiK27Code {
            model_name: "tencent/kimi-k2.7-code",
            constructor_name: tencent_kimi_k2_7_code,
            display_name: "Kimi K2.7 Code (Tencent Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        TencentKimiK27CodeHighspeed {
            model_name: "tencent/kimi-k2.7-code-highspeed",
            constructor_name: tencent_kimi_k2_7_code_highspeed,
            display_name: "Kimi K2.7 Code Highspeed (Tencent Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        TencentKimiK3 {
            model_name: "tencent/kimi-k3",
            constructor_name: tencent_kimi_k3,
            display_name: "Kimi K3 (Tencent Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        TencentMinimaxM27 {
            model_name: "tencent/minimax-m2.7",
            constructor_name: tencent_minimax_m2_7,
            display_name: "MiniMax M2.7 (Tencent Cloud)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TencentMinimaxM3 {
            model_name: "tencent/minimax-m3",
            constructor_name: tencent_minimax_m3,
            display_name: "MiniMax M3 (Tencent Cloud)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        TogetherAiDeepseekV4Pro {
            model_name: "together-ai/deepseek-v4-pro",
            constructor_name: together_ai_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro (Together AI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TogetherAiDeepseekV41Flash {
            model_name: "together-ai/deepseek-v4.1-flash",
            constructor_name: together_ai_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash (Together AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TogetherAiDeepseekV41Flash1017 {
            model_name: "together-ai/deepseek-v4.1-flash-1017",
            constructor_name: together_ai_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017 (Together AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TogetherAiGlm47 {
            model_name: "together-ai/glm-4.7",
            constructor_name: together_ai_glm_4_7,
            display_name: "GLM-4.7 (Together AI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport]
        },
        TogetherAiGlm53 {
            model_name: "together-ai/glm-5.3",
            constructor_name: together_ai_glm_5_3,
            display_name: "GLM-5.3 (Together AI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TogetherAiGlm53Flash {
            model_name: "together-ai/glm-5.3-flash",
            constructor_name: together_ai_glm_5_3_flash,
            display_name: "GLM-5.3 Flash (Together AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        TogetherAiGptOss120b {
            model_name: "together-ai/gpt-oss-120b",
            constructor_name: together_ai_gpt_oss_120b,
            display_name: "GPT OSS 120B (Together AI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TogetherAiKimiK3 {
            model_name: "together-ai/kimi-k3",
            constructor_name: together_ai_kimi_k3,
            display_name: "Kimi K3 (Together AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        TogetherAiMinimaxM3 {
            model_name: "together-ai/minimax-m3",
            constructor_name: together_ai_minimax_m3,
            display_name: "MiniMax M3 (Together AI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport]
        },
        TogetherAiMuseGlimmer30b {
            model_name: "together-ai/muse-glimmer-30b",
            constructor_name: together_ai_muse_glimmer_30b,
            display_name: "Muse Glimmer 30B (Together AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        TogetherAiQwen3824tA95b {
            model_name: "together-ai/qwen3.8-2.4t-a95b",
            constructor_name: together_ai_qwen3_8_2_4t_a95b,
            display_name: "Qwen3.8 2.4T A95B (Together AI)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        VertexAnthropicClaudeHaiku45 {
            model_name: "vertex-anthropic/claude-haiku-4-5",
            constructor_name: vertex_anthropic_claude_haiku_4_5,
            display_name: "Claude Haiku 4.5 (Vertex AI (Anthropic))",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        VertexAnthropicClaudeOpus4520251101 {
            model_name: "vertex-anthropic/claude-opus-4-5-20251101",
            constructor_name: vertex_anthropic_claude_opus_4_5_20251101,
            display_name: "Claude Opus 4.5 (Vertex AI (Anthropic))",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        VertexAnthropicClaudeOpus46 {
            model_name: "vertex-anthropic/claude-opus-4-6",
            constructor_name: vertex_anthropic_claude_opus_4_6,
            display_name: "Claude Opus 4.6 (Vertex AI (Anthropic))",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        VertexAnthropicClaudeOpus47 {
            model_name: "vertex-anthropic/claude-opus-4-7",
            constructor_name: vertex_anthropic_claude_opus_4_7,
            display_name: "Claude Opus 4.7 (Vertex AI (Anthropic))",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        VertexAnthropicClaudeSonnet45 {
            model_name: "vertex-anthropic/claude-sonnet-4-5",
            constructor_name: vertex_anthropic_claude_sonnet_4_5,
            display_name: "Claude Sonnet 4.5 (Vertex AI (Anthropic))",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        VertexAnthropicClaudeSonnet46 {
            model_name: "vertex-anthropic/claude-sonnet-4-6",
            constructor_name: vertex_anthropic_claude_sonnet_4_6,
            display_name: "Claude Sonnet 4.6 (Vertex AI (Anthropic))",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        VertexAnthropicClaudeSonnet5 {
            model_name: "vertex-anthropic/claude-sonnet-5",
            constructor_name: vertex_anthropic_claude_sonnet_5,
            display_name: "Claude Sonnet 5 (Vertex AI (Anthropic))",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        VertexOpenaiGrok420NonReasoning {
            model_name: "vertex-openai/grok-4-20-non-reasoning",
            constructor_name: vertex_openai_grok_4_20_non_reasoning,
            display_name: "Grok 4.20 Non-Reasoning (Vertex AI (OpenAI-compatible))",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        VertexOpenaiGrok420Reasoning {
            model_name: "vertex-openai/grok-4-20-reasoning",
            constructor_name: vertex_openai_grok_4_20_reasoning,
            display_name: "Grok 4.20 Reasoning (Vertex AI (OpenAI-compatible))",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        VertexOpenaiGrok46 {
            model_name: "vertex-openai/grok-4-6",
            constructor_name: vertex_openai_grok_4_6,
            display_name: "Grok 4.6 (Vertex AI (OpenAI-compatible))",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        VertexOpenaiGrok47 {
            model_name: "vertex-openai/grok-4-7",
            constructor_name: vertex_openai_grok_4_7,
            display_name: "Grok 4.7 (Vertex AI (OpenAI-compatible))",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok4 {
            model_name: "xai/grok-4",
            constructor_name: xai_grok_4,
            display_name: "Grok 4 (xAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok420Beta0309NonReasoning {
            model_name: "xai/grok-4-20-beta-0309-non-reasoning",
            constructor_name: xai_grok_4_20_beta_0309_non_reasoning,
            display_name: "Grok 4.20 Beta Non-Reasoning (0309) (xAI)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok420Beta0309Reasoning {
            model_name: "xai/grok-4-20-beta-0309-reasoning",
            constructor_name: xai_grok_4_20_beta_0309_reasoning,
            display_name: "Grok 4.20 Beta Reasoning (0309) (xAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok43 {
            model_name: "xai/grok-4-3",
            constructor_name: xai_grok_4_3,
            display_name: "Grok 4.3 (xAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok45 {
            model_name: "xai/grok-4-5",
            constructor_name: xai_grok_4_5,
            display_name: "Grok 4.5 (xAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok46 {
            model_name: "xai/grok-4-6",
            constructor_name: xai_grok_4_6,
            display_name: "Grok 4.6 (xAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok47 {
            model_name: "xai/grok-4-7",
            constructor_name: xai_grok_4_7,
            display_name: "Grok 4.7 (xAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrokBuild01 {
            model_name: "xai/grok-build-0-1",
            constructor_name: xai_grok_build_0_1,
            display_name: "Grok Build 0.1 (xAI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XiaomiMimoV26Flash {
            model_name: "xiaomi/mimo-v2.6-flash",
            constructor_name: xiaomi_mimo_v2_6_flash,
            display_name: "MiMo V2.6 Flash (Xiaomi)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        XiaomiMimoV26Pro {
            model_name: "xiaomi/mimo-v2.6-pro",
            constructor_name: xiaomi_mimo_v2_6_pro,
            display_name: "MiMo V2.6 Pro (Xiaomi)",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ZaiGlm432b0414128k {
            model_name: "zai/glm-4-32b-0414-128k",
            constructor_name: zai_glm_4_32b_0414_128k,
            display_name: "GLM-4 32B (0414-128k) (Z AI)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm45 {
            model_name: "zai/glm-4.5",
            constructor_name: zai_glm_4_5,
            display_name: "GLM-4.5 (Z AI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm45Air {
            model_name: "zai/glm-4.5-air",
            constructor_name: zai_glm_4_5_air,
            display_name: "GLM-4.5 Air (Z AI)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm45Airx {
            model_name: "zai/glm-4.5-airx",
            constructor_name: zai_glm_4_5_airx,
            display_name: "GLM-4.5 AirX (Z AI)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm45X {
            model_name: "zai/glm-4.5-x",
            constructor_name: zai_glm_4_5_x,
            display_name: "GLM-4.5 X (Z AI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm45v {
            model_name: "zai/glm-4.5v",
            constructor_name: zai_glm_4_5v,
            display_name: "GLM-4.5V (Z AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ZaiGlm46 {
            model_name: "zai/glm-4.6",
            constructor_name: zai_glm_4_6,
            display_name: "GLM-4.6 (Z AI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm46v {
            model_name: "zai/glm-4.6v",
            constructor_name: zai_glm_4_6v,
            display_name: "GLM-4.6V (Z AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ZaiGlm46vFlashx {
            model_name: "zai/glm-4.6v-flashx",
            constructor_name: zai_glm_4_6v_flashx,
            display_name: "GLM-4.6V FlashX (Z AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm47 {
            model_name: "zai/glm-4.7",
            constructor_name: zai_glm_4_7,
            display_name: "GLM-4.7 (Z AI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm47Flashx {
            model_name: "zai/glm-4.7-flashx",
            constructor_name: zai_glm_4_7_flashx,
            display_name: "GLM-4.7 FlashX (Z AI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm5 {
            model_name: "zai/glm-5",
            constructor_name: zai_glm_5,
            display_name: "GLM-5 (Z AI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm51 {
            model_name: "zai/glm-5.1",
            constructor_name: zai_glm_5_1,
            display_name: "GLM-5.1 (Z AI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm52 {
            model_name: "zai/glm-5.2",
            constructor_name: zai_glm_5_2,
            display_name: "GLM-5.2 (Z AI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm53 {
            model_name: "zai/glm-5.3",
            constructor_name: zai_glm_5_3,
            display_name: "GLM-5.3 (Z AI)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm53Flash {
            model_name: "zai/glm-5.3-flash",
            constructor_name: zai_glm_5_3_flash,
            display_name: "GLM-5.3 Flash (Z AI)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
