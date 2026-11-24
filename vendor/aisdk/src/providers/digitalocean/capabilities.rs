//! Capabilities for digitalocean models.
//!
//! This module defines model types and their capabilities for digitalocean providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::digitalocean::Digitalocean;

model_capabilities! {
    provider: Digitalocean,
    models: {
        AllMiniLmL6V2 {
            model_name: "all-mini-lm-l6-v2",
            constructor_name: all_mini_lm_l6_v2,
            display_name: "All-MiniLM-L6-v2",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        AnthropicClaude45Sonnet {
            model_name: "anthropic-claude-4.5-sonnet",
            constructor_name: anthropic_claude_4_5_sonnet,
            display_name: "Anthropic Claude 4.5 Sonnet",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaude46Sonnet {
            model_name: "anthropic-claude-4.6-sonnet",
            constructor_name: anthropic_claude_4_6_sonnet,
            display_name: "Anthropic Claude Sonnet 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaude5Sonnet {
            model_name: "anthropic-claude-5-sonnet",
            constructor_name: anthropic_claude_5_sonnet,
            display_name: "Anthropic Claude Sonnet 5",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaude55Haiku {
            model_name: "anthropic-claude-5.5-haiku",
            constructor_name: anthropic_claude_5_5_haiku,
            display_name: "Anthropic Claude 5.5 Haiku",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeFable5 {
            model_name: "anthropic-claude-fable-5",
            constructor_name: anthropic_claude_fable_5,
            display_name: "Anthropic Claude Fable 5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeFable51 {
            model_name: "anthropic-claude-fable-5.1",
            constructor_name: anthropic_claude_fable_5_1,
            display_name: "Anthropic Claude Fable 5.1",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeHaiku45 {
            model_name: "anthropic-claude-haiku-4.5",
            constructor_name: anthropic_claude_haiku_4_5,
            display_name: "Anthropic Claude Haiku 4.5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus45 {
            model_name: "anthropic-claude-opus-4.5",
            constructor_name: anthropic_claude_opus_4_5,
            display_name: "Anthropic Claude Opus 4.5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus46 {
            model_name: "anthropic-claude-opus-4.6",
            constructor_name: anthropic_claude_opus_4_6,
            display_name: "Anthropic Claude Opus 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus47 {
            model_name: "anthropic-claude-opus-4.7",
            constructor_name: anthropic_claude_opus_4_7,
            display_name: "Anthropic Claude Opus 4.7",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus48 {
            model_name: "anthropic-claude-opus-4.8",
            constructor_name: anthropic_claude_opus_4_8,
            display_name: "Anthropic Claude Opus 4.8",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus5 {
            model_name: "anthropic-claude-opus-5",
            constructor_name: anthropic_claude_opus_5,
            display_name: "Anthropic Claude Opus 5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus55 {
            model_name: "anthropic-claude-opus-5.5",
            constructor_name: anthropic_claude_opus_5_5,
            display_name: "Anthropic Claude Opus 5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet55 {
            model_name: "anthropic-claude-sonnet-5.5",
            constructor_name: anthropic_claude_sonnet_5_5,
            display_name: "Anthropic Claude Sonnet 5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ArceeTrinityLargeThinking {
            model_name: "arcee-trinity-large-thinking",
            constructor_name: arcee_trinity_large_thinking,
            display_name: "Arcee Trinity Large Thinking (Public Preview)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BgeM3 {
            model_name: "bge-m3",
            constructor_name: bge_m3,
            display_name: "BGE M3",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        BgeRerankerV2M3 {
            model_name: "bge-reranker-v2-m3",
            constructor_name: bge_reranker_v2_m3,
            display_name: "BGE Reranker v2 M3",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        Deepseek32 {
            model_name: "deepseek-3.2",
            constructor_name: deepseek_3_2,
            display_name: "Deepseek 3.2",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Deepseek4Flash {
            model_name: "deepseek-4-flash",
            constructor_name: deepseek_4_flash,
            display_name: "Deepseek V4 Flash",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
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
            display_name: "Deepseek V4 Pro",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV4Pro0813 {
            model_name: "deepseek-v4-pro-0813",
            constructor_name: deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV41Flash {
            model_name: "deepseek-v4.1-flash",
            constructor_name: deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekV41Flash1017 {
            model_name: "deepseek-v4.1-flash-1017",
            constructor_name: deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        E5LargeV2 {
            model_name: "e5-large-v2",
            constructor_name: e5_large_v2,
            display_name: "E5 Large v2",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        FalAiElevenlabsTtsMultilingualV2 {
            model_name: "fal-ai/elevenlabs/tts/multilingual-v2",
            constructor_name: fal_ai_elevenlabs_tts_multilingual_v2,
            display_name: "ElevenLabs Multilingual TTS v2",
            capabilities: [AudioOutputSupport, TextInputSupport]
        },
        FalAiFastSdxl {
            model_name: "fal-ai/fast-sdxl",
            constructor_name: fal_ai_fast_sdxl,
            display_name: "Fast SDXL",
            capabilities: [ImageOutputSupport, TextInputSupport]
        },
        FalAiFluxSchnell {
            model_name: "fal-ai/flux/schnell",
            constructor_name: fal_ai_flux_schnell,
            display_name: "FLUX.1 [schnell]",
            capabilities: [ImageOutputSupport, TextInputSupport]
        },
        FalAiStableAudio25TextToAudio {
            model_name: "fal-ai/stable-audio-25/text-to-audio",
            constructor_name: fal_ai_stable_audio_25_text_to_audio,
            display_name: "Stable Audio 2.5 (Text-to-Audio)",
            capabilities: [AudioOutputSupport, TextInputSupport]
        },
        Gemma431bIt {
            model_name: "gemma-4-31B-it",
            constructor_name: gemma_4_31b_it,
            display_name: "Gemma 4",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
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
            display_name: "GLM5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Glm53Flash {
            model_name: "glm-5.3-flash",
            constructor_name: glm_5_3_flash,
            display_name: "GLM5.3 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GteLargeEnV15 {
            model_name: "gte-large-en-v1.5",
            constructor_name: gte_large_en_v1_5,
            display_name: "GTE Large (v1.5)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        KimiK26 {
            model_name: "kimi-k2.6",
            constructor_name: kimi_k2_6,
            display_name: "Kimi K2.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        KimiK3 {
            model_name: "kimi-k3",
            constructor_name: kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Llama4Maverick {
            model_name: "llama-4-maverick",
            constructor_name: llama_4_maverick,
            display_name: "Llama 4 Maverick",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MimoV25Pro {
            model_name: "mimo-v2.5-pro",
            constructor_name: mimo_v2_5_pro,
            display_name: "MiMo V2.5 Pro",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxM25 {
            model_name: "minimax-m2.5",
            constructor_name: minimax_m2_5,
            display_name: "MiniMax M2.5 (Public Preview)",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Mistral314b {
            model_name: "mistral-3-14B",
            constructor_name: mistral_3_14b,
            display_name: "Ministral 3 14B",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MultiQaMpnetBaseDotV1 {
            model_name: "multi-qa-mpnet-base-dot-v1",
            constructor_name: multi_qa_mpnet_base_dot_v1,
            display_name: "Multi-QA-mpnet-base-dot-v1",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        Nemotron3NanoOmni {
            model_name: "nemotron-3-nano-omni",
            constructor_name: nemotron_3_nano_omni,
            display_name: "Nemotron 3 Nano Omni",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Nemotron3Ultra550b {
            model_name: "nemotron-3-ultra-550b",
            constructor_name: nemotron_3_ultra_550b,
            display_name: "Nemotron 3 Ultra",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Nemotron35SuperVl {
            model_name: "nemotron-3.5-super-vl",
            constructor_name: nemotron_3_5_super_vl,
            display_name: "NVIDIA Nemotron 3.5 Super VL",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NemotronNano12bV2Vl {
            model_name: "nemotron-nano-12b-v2-vl",
            constructor_name: nemotron_nano_12b_v2_vl,
            display_name: "Nemotron-nano 12b v2-vl",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        OpenaiGpt41 {
            model_name: "openai-gpt-4.1",
            constructor_name: openai_gpt_4_1,
            display_name: "OpenAI GPT-4.1",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4o {
            model_name: "openai-gpt-4o",
            constructor_name: openai_gpt_4o,
            display_name: "OpenAI GPT-4o",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4oMini {
            model_name: "openai-gpt-4o-mini",
            constructor_name: openai_gpt_4o_mini,
            display_name: "OpenAI GPT-4o mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5 {
            model_name: "openai-gpt-5",
            constructor_name: openai_gpt_5,
            display_name: "OpenAI GPT-5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5Mini {
            model_name: "openai-gpt-5-mini",
            constructor_name: openai_gpt_5_mini,
            display_name: "OpenAI GPT-5 Mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5Nano {
            model_name: "openai-gpt-5-nano",
            constructor_name: openai_gpt_5_nano,
            display_name: "OpenAI GPT-5 Nano",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt52 {
            model_name: "openai-gpt-5.2",
            constructor_name: openai_gpt_5_2,
            display_name: "OpenAI GPT-5.2",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt52Pro {
            model_name: "openai-gpt-5.2-pro",
            constructor_name: openai_gpt_5_2_pro,
            display_name: "OpenAI GPT-5.2 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt53Codex {
            model_name: "openai-gpt-5.3-codex",
            constructor_name: openai_gpt_5_3_codex,
            display_name: "OpenAI GPT-5.3 Codex",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54 {
            model_name: "openai-gpt-5.4",
            constructor_name: openai_gpt_5_4,
            display_name: "OpenAI GPT-5.4",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Mini {
            model_name: "openai-gpt-5.4-mini",
            constructor_name: openai_gpt_5_4_mini,
            display_name: "OpenAI GPT-5.4 Mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Nano {
            model_name: "openai-gpt-5.4-nano",
            constructor_name: openai_gpt_5_4_nano,
            display_name: "OpenAI GPT-5.4 Nano",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Pro {
            model_name: "openai-gpt-5.4-pro",
            constructor_name: openai_gpt_5_4_pro,
            display_name: "OpenAI GPT-5.4 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt55 {
            model_name: "openai-gpt-5.5",
            constructor_name: openai_gpt_5_5,
            display_name: "OpenAI GPT-5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Luna {
            model_name: "openai-gpt-5.6-luna",
            constructor_name: openai_gpt_5_6_luna,
            display_name: "OpenAI GPT-5.6 Luna",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Sol {
            model_name: "openai-gpt-5.6-sol",
            constructor_name: openai_gpt_5_6_sol,
            display_name: "OpenAI GPT-5.6 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt56Terra {
            model_name: "openai-gpt-5.6-terra",
            constructor_name: openai_gpt_5_6_terra,
            display_name: "OpenAI GPT-5.6 Terra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt61Sol {
            model_name: "openai-gpt-6-1-sol",
            constructor_name: openai_gpt_6_1_sol,
            display_name: "OpenAI GPT-6.1 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Astra {
            model_name: "openai-gpt-6-astra",
            constructor_name: openai_gpt_6_astra,
            display_name: "OpenAI GPT-6 Astra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Luna {
            model_name: "openai-gpt-6-luna",
            constructor_name: openai_gpt_6_luna,
            display_name: "OpenAI GPT-6 Luna",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt6Sol {
            model_name: "openai-gpt-6-sol",
            constructor_name: openai_gpt_6_sol,
            display_name: "OpenAI GPT-6 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptImage1 {
            model_name: "openai-gpt-image-1",
            constructor_name: openai_gpt_image_1,
            display_name: "GPT Image 1",
            capabilities: [ImageInputSupport, ImageOutputSupport, TextInputSupport]
        },
        OpenaiGptImage15 {
            model_name: "openai-gpt-image-1.5",
            constructor_name: openai_gpt_image_1_5,
            display_name: "OpenAI GPT Image 1.5",
            capabilities: [ImageInputSupport, ImageOutputSupport, TextInputSupport, TextOutputSupport]
        },
        OpenaiGptImage2 {
            model_name: "openai-gpt-image-2",
            constructor_name: openai_gpt_image_2,
            display_name: "OpenAI GPT Image 2",
            capabilities: [ImageInputSupport, ImageOutputSupport, TextInputSupport, TextOutputSupport]
        },
        OpenaiGptOss120b {
            model_name: "openai-gpt-oss-120b",
            constructor_name: openai_gpt_oss_120b,
            display_name: "OpenAI GPT-oss-120b",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOss20b {
            model_name: "openai-gpt-oss-20b",
            constructor_name: openai_gpt_oss_20b,
            display_name: "OpenAI GPT-oss-20b",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiO1 {
            model_name: "openai-o1",
            constructor_name: openai_o1,
            display_name: "OpenAI o1",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiO3 {
            model_name: "openai-o3",
            constructor_name: openai_o3,
            display_name: "OpenAI o3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiO3Mini {
            model_name: "openai-o3-mini",
            constructor_name: openai_o3_mini,
            display_name: "OpenAI o3 mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen3Embedding06b {
            model_name: "qwen3-embedding-0.6b",
            constructor_name: qwen3_embedding_0_6b,
            display_name: "Qwen3 Embedding 0.6B",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        Qwen3TtsVoicedesign {
            model_name: "qwen3-tts-voicedesign",
            constructor_name: qwen3_tts_voicedesign,
            display_name: "Qwen3 TTS VoiceDesign",
            capabilities: [AudioOutputSupport, TextInputSupport]
        },
        Qwen35397bA17b {
            model_name: "qwen3.5-397b-a17b",
            constructor_name: qwen3_5_397b_a17b,
            display_name: "Qwen 3.5 397B A17B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Qwen38Max {
            model_name: "qwen3.8-max",
            constructor_name: qwen3_8_max,
            display_name: "Qwen3.8-Max",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ReflectionBeam {
            model_name: "reflection-beam",
            constructor_name: reflection_beam,
            display_name: "Beam",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        StableDiffusion35Large {
            model_name: "stable-diffusion-3.5-large",
            constructor_name: stable_diffusion_3_5_large,
            display_name: "Stable Diffusion 3.5 Large",
            capabilities: [ImageOutputSupport, TextInputSupport]
        },
        Wan22T2vA14b {
            model_name: "wan2-2-t2v-a14b",
            constructor_name: wan2_2_t2v_a14b,
            display_name: "Wan2.2-T2V-A14B",
            capabilities: [TextInputSupport, VideoOutputSupport]
        },
    }
}
