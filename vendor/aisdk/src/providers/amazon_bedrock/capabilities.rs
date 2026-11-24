//! Capabilities for amazon_bedrock models.
//!
//! This module defines model types and their capabilities for amazon_bedrock providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::amazon_bedrock::AmazonBedrock;

model_capabilities! {
    provider: AmazonBedrock,
    models: {
        AmazonNova2LiteV10 {
            model_name: "amazon.nova-2-lite-v1:0",
            constructor_name: amazon_nova_2_lite_v1_0,
            display_name: "Nova 2 Lite",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AmazonNovaLiteV10 {
            model_name: "amazon.nova-lite-v1:0",
            constructor_name: amazon_nova_lite_v1_0,
            display_name: "Nova Lite",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AmazonNovaMicroV10 {
            model_name: "amazon.nova-micro-v1:0",
            constructor_name: amazon_nova_micro_v1_0,
            display_name: "Nova Micro",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AmazonNovaProV10 {
            model_name: "amazon.nova-pro-v1:0",
            constructor_name: amazon_nova_pro_v1_0,
            display_name: "Nova Pro",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AnthropicClaudeFable5 {
            model_name: "anthropic.claude-fable-5",
            constructor_name: anthropic_claude_fable_5,
            display_name: "Claude Fable 5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeFable51 {
            model_name: "anthropic.claude-fable-5-1",
            constructor_name: anthropic_claude_fable_5_1,
            display_name: "Claude Fable 5.1",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeHaiku4520251001V10 {
            model_name: "anthropic.claude-haiku-4-5-20251001-v1:0",
            constructor_name: anthropic_claude_haiku_4_5_20251001_v1_0,
            display_name: "Claude Haiku 4.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeHaiku55 {
            model_name: "anthropic.claude-haiku-5-5",
            constructor_name: anthropic_claude_haiku_5_5,
            display_name: "Claude Haiku 5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus4520251101V10 {
            model_name: "anthropic.claude-opus-4-5-20251101-v1:0",
            constructor_name: anthropic_claude_opus_4_5_20251101_v1_0,
            display_name: "Claude Opus 4.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus46V1 {
            model_name: "anthropic.claude-opus-4-6-v1",
            constructor_name: anthropic_claude_opus_4_6_v1,
            display_name: "Claude Opus 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus47 {
            model_name: "anthropic.claude-opus-4-7",
            constructor_name: anthropic_claude_opus_4_7,
            display_name: "Claude Opus 4.7",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus48 {
            model_name: "anthropic.claude-opus-4-8",
            constructor_name: anthropic_claude_opus_4_8,
            display_name: "Claude Opus 4.8",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus5 {
            model_name: "anthropic.claude-opus-5",
            constructor_name: anthropic_claude_opus_5,
            display_name: "Claude Opus 5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus55 {
            model_name: "anthropic.claude-opus-5-5",
            constructor_name: anthropic_claude_opus_5_5,
            display_name: "Claude Opus 5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet46 {
            model_name: "anthropic.claude-sonnet-4-6",
            constructor_name: anthropic_claude_sonnet_4_6,
            display_name: "Claude Sonnet 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet5 {
            model_name: "anthropic.claude-sonnet-5",
            constructor_name: anthropic_claude_sonnet_5,
            display_name: "Claude Sonnet 5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet55 {
            model_name: "anthropic.claude-sonnet-5-5",
            constructor_name: anthropic_claude_sonnet_5_5,
            display_name: "Claude Sonnet 5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ApacAmazonNovaLiteV10 {
            model_name: "apac.amazon.nova-lite-v1:0",
            constructor_name: apac_amazon_nova_lite_v1_0,
            display_name: "Nova Lite (APAC)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ApacAmazonNovaMicroV10 {
            model_name: "apac.amazon.nova-micro-v1:0",
            constructor_name: apac_amazon_nova_micro_v1_0,
            display_name: "Nova Micro (APAC)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ApacAmazonNovaProV10 {
            model_name: "apac.amazon.nova-pro-v1:0",
            constructor_name: apac_amazon_nova_pro_v1_0,
            display_name: "Nova Pro (APAC)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        AuAnthropicClaudeHaiku4520251001V10 {
            model_name: "au.anthropic.claude-haiku-4-5-20251001-v1:0",
            constructor_name: au_anthropic_claude_haiku_4_5_20251001_v1_0,
            display_name: "Claude Haiku 4.5 (AU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AuAnthropicClaudeHaiku55 {
            model_name: "au.anthropic.claude-haiku-5-5",
            constructor_name: au_anthropic_claude_haiku_5_5,
            display_name: "Claude Haiku 5.5 (AU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AuAnthropicClaudeOpus46V1 {
            model_name: "au.anthropic.claude-opus-4-6-v1",
            constructor_name: au_anthropic_claude_opus_4_6_v1,
            display_name: "AU Anthropic Claude Opus 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AuAnthropicClaudeOpus47 {
            model_name: "au.anthropic.claude-opus-4-7",
            constructor_name: au_anthropic_claude_opus_4_7,
            display_name: "Claude Opus 4.7 (AU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AuAnthropicClaudeOpus48 {
            model_name: "au.anthropic.claude-opus-4-8",
            constructor_name: au_anthropic_claude_opus_4_8,
            display_name: "Claude Opus 4.8 (AU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AuAnthropicClaudeOpus5 {
            model_name: "au.anthropic.claude-opus-5",
            constructor_name: au_anthropic_claude_opus_5,
            display_name: "Claude Opus 5 (AU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AuAnthropicClaudeOpus55 {
            model_name: "au.anthropic.claude-opus-5-5",
            constructor_name: au_anthropic_claude_opus_5_5,
            display_name: "Claude Opus 5.5 (AU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AuAnthropicClaudeSonnet46 {
            model_name: "au.anthropic.claude-sonnet-4-6",
            constructor_name: au_anthropic_claude_sonnet_4_6,
            display_name: "AU Anthropic Claude Sonnet 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AuAnthropicClaudeSonnet5 {
            model_name: "au.anthropic.claude-sonnet-5",
            constructor_name: au_anthropic_claude_sonnet_5,
            display_name: "Claude Sonnet 5 (AU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CaAmazonNovaLiteV10 {
            model_name: "ca.amazon.nova-lite-v1:0",
            constructor_name: ca_amazon_nova_lite_v1_0,
            display_name: "Nova Lite (CA)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DeepseekV32 {
            model_name: "deepseek.v3.2",
            constructor_name: deepseek_v3_2,
            display_name: "DeepSeek V3.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EuAmazonNova2LiteV10 {
            model_name: "eu.amazon.nova-2-lite-v1:0",
            constructor_name: eu_amazon_nova_2_lite_v1_0,
            display_name: "Nova 2 Lite (EU)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        EuAmazonNovaLiteV10 {
            model_name: "eu.amazon.nova-lite-v1:0",
            constructor_name: eu_amazon_nova_lite_v1_0,
            display_name: "Nova Lite (EU)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        EuAmazonNovaMicroV10 {
            model_name: "eu.amazon.nova-micro-v1:0",
            constructor_name: eu_amazon_nova_micro_v1_0,
            display_name: "Nova Micro (EU)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EuAmazonNovaProV10 {
            model_name: "eu.amazon.nova-pro-v1:0",
            constructor_name: eu_amazon_nova_pro_v1_0,
            display_name: "Nova Pro (EU)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        EuAnthropicClaudeFable5 {
            model_name: "eu.anthropic.claude-fable-5",
            constructor_name: eu_anthropic_claude_fable_5,
            display_name: "Claude Fable 5 (EU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EuAnthropicClaudeHaiku4520251001V10 {
            model_name: "eu.anthropic.claude-haiku-4-5-20251001-v1:0",
            constructor_name: eu_anthropic_claude_haiku_4_5_20251001_v1_0,
            display_name: "Claude Haiku 4.5 (EU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EuAnthropicClaudeHaiku55 {
            model_name: "eu.anthropic.claude-haiku-5-5",
            constructor_name: eu_anthropic_claude_haiku_5_5,
            display_name: "Claude Haiku 5.5 (EU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EuAnthropicClaudeOpus4520251101V10 {
            model_name: "eu.anthropic.claude-opus-4-5-20251101-v1:0",
            constructor_name: eu_anthropic_claude_opus_4_5_20251101_v1_0,
            display_name: "Claude Opus 4.5 (EU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EuAnthropicClaudeOpus46V1 {
            model_name: "eu.anthropic.claude-opus-4-6-v1",
            constructor_name: eu_anthropic_claude_opus_4_6_v1,
            display_name: "Claude Opus 4.6 (EU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EuAnthropicClaudeOpus47 {
            model_name: "eu.anthropic.claude-opus-4-7",
            constructor_name: eu_anthropic_claude_opus_4_7,
            display_name: "Claude Opus 4.7 (EU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EuAnthropicClaudeOpus48 {
            model_name: "eu.anthropic.claude-opus-4-8",
            constructor_name: eu_anthropic_claude_opus_4_8,
            display_name: "Claude Opus 4.8 (EU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EuAnthropicClaudeOpus5 {
            model_name: "eu.anthropic.claude-opus-5",
            constructor_name: eu_anthropic_claude_opus_5,
            display_name: "Claude Opus 5 (EU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EuAnthropicClaudeOpus55 {
            model_name: "eu.anthropic.claude-opus-5-5",
            constructor_name: eu_anthropic_claude_opus_5_5,
            display_name: "Claude Opus 5.5 (EU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EuAnthropicClaudeSonnet46 {
            model_name: "eu.anthropic.claude-sonnet-4-6",
            constructor_name: eu_anthropic_claude_sonnet_4_6,
            display_name: "Claude Sonnet 4.6 (EU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EuAnthropicClaudeSonnet5 {
            model_name: "eu.anthropic.claude-sonnet-5",
            constructor_name: eu_anthropic_claude_sonnet_5,
            display_name: "Claude Sonnet 5 (EU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EuAnthropicClaudeSonnet55 {
            model_name: "eu.anthropic.claude-sonnet-5-5",
            constructor_name: eu_anthropic_claude_sonnet_5_5,
            display_name: "Claude Sonnet 5.5 (EU)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EuMistralPixtralLarge2502V10 {
            model_name: "eu.mistral.pixtral-large-2502-v1:0",
            constructor_name: eu_mistral_pixtral_large_2502_v1_0,
            display_name: "Pixtral Large (25.02) (EU)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalAmazonNova2LiteV10 {
            model_name: "global.amazon.nova-2-lite-v1:0",
            constructor_name: global_amazon_nova_2_lite_v1_0,
            display_name: "Nova 2 Lite (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GlobalAnthropicClaudeFable5 {
            model_name: "global.anthropic.claude-fable-5",
            constructor_name: global_anthropic_claude_fable_5,
            display_name: "Claude Fable 5 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalAnthropicClaudeFable51 {
            model_name: "global.anthropic.claude-fable-5-1",
            constructor_name: global_anthropic_claude_fable_5_1,
            display_name: "Claude Fable 5.1 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalAnthropicClaudeHaiku4520251001V10 {
            model_name: "global.anthropic.claude-haiku-4-5-20251001-v1:0",
            constructor_name: global_anthropic_claude_haiku_4_5_20251001_v1_0,
            display_name: "Claude Haiku 4.5 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalAnthropicClaudeHaiku55 {
            model_name: "global.anthropic.claude-haiku-5-5",
            constructor_name: global_anthropic_claude_haiku_5_5,
            display_name: "Claude Haiku 5.5 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalAnthropicClaudeOpus4520251101V10 {
            model_name: "global.anthropic.claude-opus-4-5-20251101-v1:0",
            constructor_name: global_anthropic_claude_opus_4_5_20251101_v1_0,
            display_name: "Claude Opus 4.5 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalAnthropicClaudeOpus46V1 {
            model_name: "global.anthropic.claude-opus-4-6-v1",
            constructor_name: global_anthropic_claude_opus_4_6_v1,
            display_name: "Claude Opus 4.6 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalAnthropicClaudeOpus47 {
            model_name: "global.anthropic.claude-opus-4-7",
            constructor_name: global_anthropic_claude_opus_4_7,
            display_name: "Claude Opus 4.7 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalAnthropicClaudeOpus48 {
            model_name: "global.anthropic.claude-opus-4-8",
            constructor_name: global_anthropic_claude_opus_4_8,
            display_name: "Claude Opus 4.8 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalAnthropicClaudeOpus5 {
            model_name: "global.anthropic.claude-opus-5",
            constructor_name: global_anthropic_claude_opus_5,
            display_name: "Claude Opus 5 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalAnthropicClaudeOpus55 {
            model_name: "global.anthropic.claude-opus-5-5",
            constructor_name: global_anthropic_claude_opus_5_5,
            display_name: "Claude Opus 5.5 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalAnthropicClaudeSonnet46 {
            model_name: "global.anthropic.claude-sonnet-4-6",
            constructor_name: global_anthropic_claude_sonnet_4_6,
            display_name: "Claude Sonnet 4.6 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalAnthropicClaudeSonnet5 {
            model_name: "global.anthropic.claude-sonnet-5",
            constructor_name: global_anthropic_claude_sonnet_5,
            display_name: "Claude Sonnet 5 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalAnthropicClaudeSonnet55 {
            model_name: "global.anthropic.claude-sonnet-5-5",
            constructor_name: global_anthropic_claude_sonnet_5_5,
            display_name: "Claude Sonnet 5.5 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalMoonshotaiKimiK3 {
            model_name: "global.moonshotai.kimi-k3",
            constructor_name: global_moonshotai_kimi_k3,
            display_name: "Kimi K3 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalOpenaiGpt56Luna {
            model_name: "global.openai.gpt-5.6-luna",
            constructor_name: global_openai_gpt_5_6_luna,
            display_name: "GPT-5.6 Luna (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalOpenaiGpt56Sol {
            model_name: "global.openai.gpt-5.6-sol",
            constructor_name: global_openai_gpt_5_6_sol,
            display_name: "GPT-5.6 Sol (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalOpenaiGpt56Terra {
            model_name: "global.openai.gpt-5.6-terra",
            constructor_name: global_openai_gpt_5_6_terra,
            display_name: "GPT-5.6 Terra (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalOpenaiGpt6Astra {
            model_name: "global.openai.gpt-6-astra",
            constructor_name: global_openai_gpt_6_astra,
            display_name: "GPT-6 Astra (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalOpenaiGpt6Luna {
            model_name: "global.openai.gpt-6-luna",
            constructor_name: global_openai_gpt_6_luna,
            display_name: "GPT-6 Luna (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalOpenaiGpt6Sol {
            model_name: "global.openai.gpt-6-sol",
            constructor_name: global_openai_gpt_6_sol,
            display_name: "GPT-6 Sol (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalOpenaiGpt61Sol {
            model_name: "global.openai.gpt-6.1-sol",
            constructor_name: global_openai_gpt_6_1_sol,
            display_name: "GPT-6.1 Sol (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalXaiGrok46 {
            model_name: "global.xai.grok-4.6",
            constructor_name: global_xai_grok_4_6,
            display_name: "Grok 4.6 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalXaiGrok47 {
            model_name: "global.xai.grok-4.7",
            constructor_name: global_xai_grok_4_7,
            display_name: "Grok 4.7 (Global)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GlobalZaiGlm53 {
            model_name: "global.zai.glm-5.3",
            constructor_name: global_zai_glm_5_3,
            display_name: "GLM-5.3 (Global)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemma426bA4b {
            model_name: "google.gemma-4-26b-a4b",
            constructor_name: google_gemma_4_26b_a4b,
            display_name: "Gemma 4 26B A4B IT",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemma431b {
            model_name: "google.gemma-4-31b",
            constructor_name: google_gemma_4_31b,
            display_name: "Gemma 4 31B IT",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemma4E2b {
            model_name: "google.gemma-4-e2b",
            constructor_name: google_gemma_4_e2b,
            display_name: "Gemma 4 E2B IT",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        InAnthropicClaudeHaiku4520251001V10 {
            model_name: "in.anthropic.claude-haiku-4-5-20251001-v1:0",
            constructor_name: in_anthropic_claude_haiku_4_5_20251001_v1_0,
            display_name: "Claude Haiku 4.5 (India)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        InAnthropicClaudeOpus5 {
            model_name: "in.anthropic.claude-opus-5",
            constructor_name: in_anthropic_claude_opus_5,
            display_name: "Claude Opus 5 (India)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        InAnthropicClaudeSonnet5 {
            model_name: "in.anthropic.claude-sonnet-5",
            constructor_name: in_anthropic_claude_sonnet_5,
            display_name: "Claude Sonnet 5 (India)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        InOpenaiGpt56Luna {
            model_name: "in.openai.gpt-5.6-luna",
            constructor_name: in_openai_gpt_5_6_luna,
            display_name: "GPT-5.6 Luna (India)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        InOpenaiGpt56Terra {
            model_name: "in.openai.gpt-5.6-terra",
            constructor_name: in_openai_gpt_5_6_terra,
            display_name: "GPT-5.6 Terra (India)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        JpAmazonNova2LiteV10 {
            model_name: "jp.amazon.nova-2-lite-v1:0",
            constructor_name: jp_amazon_nova_2_lite_v1_0,
            display_name: "Nova 2 Lite (JP)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        JpAnthropicClaudeHaiku4520251001V10 {
            model_name: "jp.anthropic.claude-haiku-4-5-20251001-v1:0",
            constructor_name: jp_anthropic_claude_haiku_4_5_20251001_v1_0,
            display_name: "Claude Haiku 4.5 (JP)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        JpAnthropicClaudeHaiku55 {
            model_name: "jp.anthropic.claude-haiku-5-5",
            constructor_name: jp_anthropic_claude_haiku_5_5,
            display_name: "Claude Haiku 5.5 (JP)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        JpAnthropicClaudeOpus47 {
            model_name: "jp.anthropic.claude-opus-4-7",
            constructor_name: jp_anthropic_claude_opus_4_7,
            display_name: "Claude Opus 4.7 (JP)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        JpAnthropicClaudeOpus48 {
            model_name: "jp.anthropic.claude-opus-4-8",
            constructor_name: jp_anthropic_claude_opus_4_8,
            display_name: "Claude Opus 4.8 (JP)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        JpAnthropicClaudeOpus5 {
            model_name: "jp.anthropic.claude-opus-5",
            constructor_name: jp_anthropic_claude_opus_5,
            display_name: "Claude Opus 5 (JP)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        JpAnthropicClaudeOpus55 {
            model_name: "jp.anthropic.claude-opus-5-5",
            constructor_name: jp_anthropic_claude_opus_5_5,
            display_name: "Claude Opus 5.5 (JP)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        JpAnthropicClaudeSonnet46 {
            model_name: "jp.anthropic.claude-sonnet-4-6",
            constructor_name: jp_anthropic_claude_sonnet_4_6,
            display_name: "Claude Sonnet 4.6 (JP)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        JpAnthropicClaudeSonnet5 {
            model_name: "jp.anthropic.claude-sonnet-5",
            constructor_name: jp_anthropic_claude_sonnet_5,
            display_name: "Claude Sonnet 5 (JP)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM25 {
            model_name: "minimax.minimax-m2.5",
            constructor_name: minimax_minimax_m2_5,
            display_name: "MiniMax-M2.5",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMagistralSmall2509 {
            model_name: "mistral.magistral-small-2509",
            constructor_name: mistral_magistral_small_2509,
            display_name: "Magistral Small 1.2",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMinistral314bInstruct {
            model_name: "mistral.ministral-3-14b-instruct",
            constructor_name: mistral_ministral_3_14b_instruct,
            display_name: "Ministral 14B 3.0",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMinistral33bInstruct {
            model_name: "mistral.ministral-3-3b-instruct",
            constructor_name: mistral_ministral_3_3b_instruct,
            display_name: "Ministral 3 3B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMinistral38bInstruct {
            model_name: "mistral.ministral-3-8b-instruct",
            constructor_name: mistral_ministral_3_8b_instruct,
            display_name: "Ministral 3 8B",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralMistralLarge3675bInstruct {
            model_name: "mistral.mistral-large-3-675b-instruct",
            constructor_name: mistral_mistral_large_3_675b_instruct,
            display_name: "Mistral Large 3",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MistralPixtralLarge2502V10 {
            model_name: "mistral.pixtral-large-2502-v1:0",
            constructor_name: mistral_pixtral_large_2502_v1_0,
            display_name: "Pixtral Large (25.02)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MoonshotaiKimiK25 {
            model_name: "moonshotai.kimi-k2.5",
            constructor_name: moonshotai_kimi_k2_5,
            display_name: "Kimi K2.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NvidiaNemotronNano330b {
            model_name: "nvidia.nemotron-nano-3-30b",
            constructor_name: nvidia_nemotron_nano_3_30b,
            display_name: "NVIDIA Nemotron Nano 3 30B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NvidiaNemotronSuper3120b {
            model_name: "nvidia.nemotron-super-3-120b",
            constructor_name: nvidia_nemotron_super_3_120b,
            display_name: "NVIDIA Nemotron 3 Super 120B A12B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54 {
            model_name: "openai.gpt-5.4",
            constructor_name: openai_gpt_5_4,
            display_name: "GPT-5.4",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt55 {
            model_name: "openai.gpt-5.5",
            constructor_name: openai_gpt_5_5,
            display_name: "GPT-5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Luna {
            model_name: "openai.gpt-5.6-luna",
            constructor_name: openai_gpt_5_6_luna,
            display_name: "GPT-5.6 Luna",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Sol {
            model_name: "openai.gpt-5.6-sol",
            constructor_name: openai_gpt_5_6_sol,
            display_name: "GPT-5.6 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Terra {
            model_name: "openai.gpt-5.6-terra",
            constructor_name: openai_gpt_5_6_terra,
            display_name: "GPT-5.6 Terra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Astra {
            model_name: "openai.gpt-6-astra",
            constructor_name: openai_gpt_6_astra,
            display_name: "GPT-6 Astra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Luna {
            model_name: "openai.gpt-6-luna",
            constructor_name: openai_gpt_6_luna,
            display_name: "GPT-6 Luna",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Sol {
            model_name: "openai.gpt-6-sol",
            constructor_name: openai_gpt_6_sol,
            display_name: "GPT-6 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt61Sol {
            model_name: "openai.gpt-6.1-sol",
            constructor_name: openai_gpt_6_1_sol,
            display_name: "GPT-6.1 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOss120b {
            model_name: "openai.gpt-oss-120b",
            constructor_name: openai_gpt_oss_120b,
            display_name: "gpt-oss-120b",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOss120b10 {
            model_name: "openai.gpt-oss-120b-1:0",
            constructor_name: openai_gpt_oss_120b_1_0,
            display_name: "gpt-oss-120b",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOss20b {
            model_name: "openai.gpt-oss-20b",
            constructor_name: openai_gpt_oss_20b,
            display_name: "gpt-oss-20b",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOss20b10 {
            model_name: "openai.gpt-oss-20b-1:0",
            constructor_name: openai_gpt_oss_20b_1_0,
            display_name: "gpt-oss-20b",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOssSafeguard120b {
            model_name: "openai.gpt-oss-safeguard-120b",
            constructor_name: openai_gpt_oss_safeguard_120b,
            display_name: "GPT OSS Safeguard 120B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOssSafeguard20b {
            model_name: "openai.gpt-oss-safeguard-20b",
            constructor_name: openai_gpt_oss_safeguard_20b,
            display_name: "GPT OSS Safeguard 20B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3235bA22b2507V10 {
            model_name: "qwen.qwen3-235b-a22b-2507-v1:0",
            constructor_name: qwen_qwen3_235b_a22b_2507_v1_0,
            display_name: "Qwen3 235B-A22B Instruct 2507",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen332bV10 {
            model_name: "qwen.qwen3-32b-v1:0",
            constructor_name: qwen_qwen3_32b_v1_0,
            display_name: "Qwen3 32B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3CoderNext {
            model_name: "qwen.qwen3-coder-next",
            constructor_name: qwen_qwen3_coder_next,
            display_name: "Qwen3 Coder Next",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3Next80bA3b {
            model_name: "qwen.qwen3-next-80b-a3b",
            constructor_name: qwen_qwen3_next_80b_a3b,
            display_name: "Qwen3-Next 80B-A3B Instruct",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3Vl235bA22b {
            model_name: "qwen.qwen3-vl-235b-a22b",
            constructor_name: qwen_qwen3_vl_235b_a22b,
            display_name: "Qwen3 VL 235B A22B Instruct",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsGovOpenaiGptOss120b10 {
            model_name: "us-gov.openai.gpt-oss-120b-1:0",
            constructor_name: us_gov_openai_gpt_oss_120b_1_0,
            display_name: "gpt-oss-120b (GovCloud)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsGovOpenaiGptOss20b10 {
            model_name: "us-gov.openai.gpt-oss-20b-1:0",
            constructor_name: us_gov_openai_gpt_oss_20b_1_0,
            display_name: "gpt-oss-20b (GovCloud)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsAmazonNova2LiteV10 {
            model_name: "us.amazon.nova-2-lite-v1:0",
            constructor_name: us_amazon_nova_2_lite_v1_0,
            display_name: "Nova 2 Lite (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        UsAmazonNovaLiteV10 {
            model_name: "us.amazon.nova-lite-v1:0",
            constructor_name: us_amazon_nova_lite_v1_0,
            display_name: "Nova Lite (US)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        UsAmazonNovaMicroV10 {
            model_name: "us.amazon.nova-micro-v1:0",
            constructor_name: us_amazon_nova_micro_v1_0,
            display_name: "Nova Micro (US)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsAmazonNovaProV10 {
            model_name: "us.amazon.nova-pro-v1:0",
            constructor_name: us_amazon_nova_pro_v1_0,
            display_name: "Nova Pro (US)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        UsAnthropicClaudeFable5 {
            model_name: "us.anthropic.claude-fable-5",
            constructor_name: us_anthropic_claude_fable_5,
            display_name: "Claude Fable 5 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsAnthropicClaudeFable51 {
            model_name: "us.anthropic.claude-fable-5-1",
            constructor_name: us_anthropic_claude_fable_5_1,
            display_name: "Claude Fable 5.1 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsAnthropicClaudeHaiku4520251001V10 {
            model_name: "us.anthropic.claude-haiku-4-5-20251001-v1:0",
            constructor_name: us_anthropic_claude_haiku_4_5_20251001_v1_0,
            display_name: "Claude Haiku 4.5 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsAnthropicClaudeHaiku55 {
            model_name: "us.anthropic.claude-haiku-5-5",
            constructor_name: us_anthropic_claude_haiku_5_5,
            display_name: "Claude Haiku 5.5 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsAnthropicClaudeOpus4520251101V10 {
            model_name: "us.anthropic.claude-opus-4-5-20251101-v1:0",
            constructor_name: us_anthropic_claude_opus_4_5_20251101_v1_0,
            display_name: "Claude Opus 4.5 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsAnthropicClaudeOpus46V1 {
            model_name: "us.anthropic.claude-opus-4-6-v1",
            constructor_name: us_anthropic_claude_opus_4_6_v1,
            display_name: "Claude Opus 4.6 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsAnthropicClaudeOpus47 {
            model_name: "us.anthropic.claude-opus-4-7",
            constructor_name: us_anthropic_claude_opus_4_7,
            display_name: "Claude Opus 4.7 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsAnthropicClaudeOpus48 {
            model_name: "us.anthropic.claude-opus-4-8",
            constructor_name: us_anthropic_claude_opus_4_8,
            display_name: "Claude Opus 4.8 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsAnthropicClaudeOpus5 {
            model_name: "us.anthropic.claude-opus-5",
            constructor_name: us_anthropic_claude_opus_5,
            display_name: "Claude Opus 5 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsAnthropicClaudeOpus55 {
            model_name: "us.anthropic.claude-opus-5-5",
            constructor_name: us_anthropic_claude_opus_5_5,
            display_name: "Claude Opus 5.5 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsAnthropicClaudeSonnet46 {
            model_name: "us.anthropic.claude-sonnet-4-6",
            constructor_name: us_anthropic_claude_sonnet_4_6,
            display_name: "Claude Sonnet 4.6 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsAnthropicClaudeSonnet5 {
            model_name: "us.anthropic.claude-sonnet-5",
            constructor_name: us_anthropic_claude_sonnet_5,
            display_name: "Claude Sonnet 5 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsAnthropicClaudeSonnet55 {
            model_name: "us.anthropic.claude-sonnet-5-5",
            constructor_name: us_anthropic_claude_sonnet_5_5,
            display_name: "Claude Sonnet 5.5 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsMistralPixtralLarge2502V10 {
            model_name: "us.mistral.pixtral-large-2502-v1:0",
            constructor_name: us_mistral_pixtral_large_2502_v1_0,
            display_name: "Pixtral Large (25.02) (US)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsMoonshotaiKimiK3 {
            model_name: "us.moonshotai.kimi-k3",
            constructor_name: us_moonshotai_kimi_k3,
            display_name: "Kimi K3 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsOpenaiGpt56Luna {
            model_name: "us.openai.gpt-5.6-luna",
            constructor_name: us_openai_gpt_5_6_luna,
            display_name: "GPT-5.6 Luna (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsOpenaiGpt56Sol {
            model_name: "us.openai.gpt-5.6-sol",
            constructor_name: us_openai_gpt_5_6_sol,
            display_name: "GPT-5.6 Sol (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsOpenaiGpt56Terra {
            model_name: "us.openai.gpt-5.6-terra",
            constructor_name: us_openai_gpt_5_6_terra,
            display_name: "GPT-5.6 Terra (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsOpenaiGpt6Astra {
            model_name: "us.openai.gpt-6-astra",
            constructor_name: us_openai_gpt_6_astra,
            display_name: "GPT-6 Astra (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsOpenaiGpt6Luna {
            model_name: "us.openai.gpt-6-luna",
            constructor_name: us_openai_gpt_6_luna,
            display_name: "GPT-6 Luna (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsOpenaiGpt6Sol {
            model_name: "us.openai.gpt-6-sol",
            constructor_name: us_openai_gpt_6_sol,
            display_name: "GPT-6 Sol (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsOpenaiGpt61Sol {
            model_name: "us.openai.gpt-6.1-sol",
            constructor_name: us_openai_gpt_6_1_sol,
            display_name: "GPT-6.1 Sol (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsXaiGrok46 {
            model_name: "us.xai.grok-4.6",
            constructor_name: us_xai_grok_4_6,
            display_name: "Grok 4.6 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsXaiGrok47 {
            model_name: "us.xai.grok-4.7",
            constructor_name: us_xai_grok_4_7,
            display_name: "Grok 4.7 (US)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        UsZaiGlm53 {
            model_name: "us.zai.glm-5.3",
            constructor_name: us_zai_glm_5_3,
            display_name: "GLM-5.3 (US)",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok43 {
            model_name: "xai.grok-4.3",
            constructor_name: xai_grok_4_3,
            display_name: "Grok 4.3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok46 {
            model_name: "xai.grok-4.6",
            constructor_name: xai_grok_4_6,
            display_name: "Grok 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiGlm5 {
            model_name: "zai.glm-5",
            constructor_name: zai_glm_5,
            display_name: "GLM-5",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
