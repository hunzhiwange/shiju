//! Capabilities for poe models.
//!
//! This module defines model types and their capabilities for poe providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::poe::Poe;

model_capabilities! {
    provider: Poe,
    models: {
        AnthropicClaudeHaiku45 {
            model_name: "anthropic/claude-haiku-4.5",
            constructor_name: anthropic_claude_haiku_4_5,
            display_name: "Claude-Haiku-4.5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus45 {
            model_name: "anthropic/claude-opus-4.5",
            constructor_name: anthropic_claude_opus_4_5,
            display_name: "Claude-Opus-4.5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus46 {
            model_name: "anthropic/claude-opus-4.6",
            constructor_name: anthropic_claude_opus_4_6,
            display_name: "Claude-Opus-4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus47 {
            model_name: "anthropic/claude-opus-4.7",
            constructor_name: anthropic_claude_opus_4_7,
            display_name: "Claude-Opus-4.7",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus48 {
            model_name: "anthropic/claude-opus-4.8",
            constructor_name: anthropic_claude_opus_4_8,
            display_name: "Claude-Opus-4.8",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet45 {
            model_name: "anthropic/claude-sonnet-4.5",
            constructor_name: anthropic_claude_sonnet_4_5,
            display_name: "Claude-Sonnet-4.5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet46 {
            model_name: "anthropic/claude-sonnet-4.6",
            constructor_name: anthropic_claude_sonnet_4_6,
            display_name: "Claude-Sonnet-4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CerebrasGptOss120bCs {
            model_name: "cerebras/gpt-oss-120b-cs",
            constructor_name: cerebras_gpt_oss_120b_cs,
            display_name: "GPT-OSS-120B-CS",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        CerebrasLlama318bCs {
            model_name: "cerebras/llama-3.1-8b-cs",
            constructor_name: cerebras_llama_3_1_8b_cs,
            display_name: "Llama-3.1-8B-CS",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ElevenlabsElevenlabsMusic {
            model_name: "elevenlabs/elevenlabs-music",
            constructor_name: elevenlabs_elevenlabs_music,
            display_name: "ElevenLabs-Music",
            capabilities: [AudioOutputSupport, TextInputSupport, ToolCallSupport]
        },
        ElevenlabsElevenlabsV25Turbo {
            model_name: "elevenlabs/elevenlabs-v2.5-turbo",
            constructor_name: elevenlabs_elevenlabs_v2_5_turbo,
            display_name: "ElevenLabs-v2.5-Turbo",
            capabilities: [AudioOutputSupport, TextInputSupport, ToolCallSupport]
        },
        ElevenlabsElevenlabsV3 {
            model_name: "elevenlabs/elevenlabs-v3",
            constructor_name: elevenlabs_elevenlabs_v3,
            display_name: "ElevenLabs-v3",
            capabilities: [AudioOutputSupport, TextInputSupport, ToolCallSupport]
        },
        EmpiriolabsDeepseekV4FlashEl {
            model_name: "empiriolabs/deepseek-v4-flash-el",
            constructor_name: empiriolabs_deepseek_v4_flash_el,
            display_name: "DeepSeek-V4-Flash-EL",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        EmpiriolabsDeepseekV4ProEl {
            model_name: "empiriolabs/deepseek-v4-pro-el",
            constructor_name: empiriolabs_deepseek_v4_pro_el,
            display_name: "DeepSeek-V4-Pro-EL",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemini3Flash {
            model_name: "google/gemini-3-flash",
            constructor_name: google_gemini_3_flash,
            display_name: "Gemini-3-Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini31FlashLite {
            model_name: "google/gemini-3.1-flash-lite",
            constructor_name: google_gemini_3_1_flash_lite,
            display_name: "Gemini-3.1-Flash-Lite",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini31Pro {
            model_name: "google/gemini-3.1-pro",
            constructor_name: google_gemini_3_1_pro,
            display_name: "Gemini-3.1-Pro",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini35Flash {
            model_name: "google/gemini-3.5-flash",
            constructor_name: google_gemini_3_5_flash,
            display_name: "Gemini-3.5-Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini35FlashLite {
            model_name: "google/gemini-3.5-flash-lite",
            constructor_name: google_gemini_3_5_flash_lite,
            display_name: "Gemini-3.5-Flash-Lite",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini36Flash {
            model_name: "google/gemini-3.6-flash",
            constructor_name: google_gemini_3_6_flash,
            display_name: "Gemini-3.6-Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini37Flash {
            model_name: "google/gemini-3.7-flash",
            constructor_name: google_gemini_3_7_flash,
            display_name: "Gemini-3.7-Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini38Flash {
            model_name: "google/gemini-3.8-flash",
            constructor_name: google_gemini_3_8_flash,
            display_name: "Gemini-3.8-Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGeminiOmni11Flash {
            model_name: "google/gemini-omni-1.1-flash",
            constructor_name: google_gemini_omni_1_1_flash,
            display_name: "Gemini-Omni-1.1-Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, VideoInputSupport, VideoOutputSupport]
        },
        GoogleGemma431b {
            model_name: "google/gemma-4-31b",
            constructor_name: google_gemma_4_31b,
            display_name: "Gemma-4-31B",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleImagen4 {
            model_name: "google/imagen-4",
            constructor_name: google_imagen_4,
            display_name: "Imagen-4",
            capabilities: [ImageOutputSupport, TextInputSupport, ToolCallSupport]
        },
        GoogleImagen4Fast {
            model_name: "google/imagen-4-fast",
            constructor_name: google_imagen_4_fast,
            display_name: "Imagen-4-Fast",
            capabilities: [ImageOutputSupport, TextInputSupport, ToolCallSupport]
        },
        GoogleLyria {
            model_name: "google/lyria",
            constructor_name: google_lyria,
            display_name: "Lyria",
            capabilities: [AudioOutputSupport, TextInputSupport, ToolCallSupport]
        },
        GoogleNanoBananaPro {
            model_name: "google/nano-banana-pro",
            constructor_name: google_nano_banana_pro,
            display_name: "Nano-Banana-Pro",
            capabilities: [ImageInputSupport, ImageOutputSupport, TextInputSupport, ToolCallSupport]
        },
        GoogleVeo31 {
            model_name: "google/veo-3.1",
            constructor_name: google_veo_3_1,
            display_name: "Veo-3.1",
            capabilities: [TextInputSupport, ToolCallSupport, VideoOutputSupport]
        },
        GoogleVeo31Fast {
            model_name: "google/veo-3.1-fast",
            constructor_name: google_veo_3_1_fast,
            display_name: "Veo-3.1-Fast",
            capabilities: [ImageInputSupport, TextInputSupport, ToolCallSupport, VideoOutputSupport]
        },
        IdeogramaiIdeogram {
            model_name: "ideogramai/ideogram",
            constructor_name: ideogramai_ideogram,
            display_name: "Ideogram",
            capabilities: [ImageInputSupport, ImageOutputSupport, TextInputSupport, ToolCallSupport]
        },
        IdeogramaiIdeogramV2 {
            model_name: "ideogramai/ideogram-v2",
            constructor_name: ideogramai_ideogram_v2,
            display_name: "Ideogram-v2",
            capabilities: [ImageInputSupport, ImageOutputSupport, TextInputSupport, ToolCallSupport]
        },
        IdeogramaiIdeogramV2a {
            model_name: "ideogramai/ideogram-v2a",
            constructor_name: ideogramai_ideogram_v2a,
            display_name: "Ideogram-v2a",
            capabilities: [ImageOutputSupport, TextInputSupport, ToolCallSupport]
        },
        IdeogramaiIdeogramV2aTurbo {
            model_name: "ideogramai/ideogram-v2a-turbo",
            constructor_name: ideogramai_ideogram_v2a_turbo,
            display_name: "Ideogram-v2a-Turbo",
            capabilities: [ImageOutputSupport, TextInputSupport, ToolCallSupport]
        },
        LumalabsRay2 {
            model_name: "lumalabs/ray2",
            constructor_name: lumalabs_ray2,
            display_name: "Ray2",
            capabilities: [ImageInputSupport, TextInputSupport, ToolCallSupport, VideoOutputSupport]
        },
        NovitaDeepseekV32 {
            model_name: "novita/deepseek-v3.2",
            constructor_name: novita_deepseek_v3_2,
            display_name: "DeepSeek-V3.2",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaGlm46 {
            model_name: "novita/glm-4.6",
            constructor_name: novita_glm_4_6,
            display_name: "GLM-4.6",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaGlm46v {
            model_name: "novita/glm-4.6v",
            constructor_name: novita_glm_4_6v,
            display_name: "glm-4.6v",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaGlm47Flash {
            model_name: "novita/glm-4.7-flash",
            constructor_name: novita_glm_4_7_flash,
            display_name: "glm-4.7-flash",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaGlm47N {
            model_name: "novita/glm-4.7-n",
            constructor_name: novita_glm_4_7_n,
            display_name: "glm-4.7-n",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaGlm5 {
            model_name: "novita/glm-5",
            constructor_name: novita_glm_5,
            display_name: "GLM-5",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NovitaKimiK26 {
            model_name: "novita/kimi-k2.6",
            constructor_name: novita_kimi_k2_6,
            display_name: "Kimi-K2.6",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        NovitaMinimaxM21 {
            model_name: "novita/minimax-m2.1",
            constructor_name: novita_minimax_m2_1,
            display_name: "minimax-m2.1",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt41 {
            model_name: "openai/gpt-4.1",
            constructor_name: openai_gpt_4_1,
            display_name: "GPT-4.1",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt41Mini {
            model_name: "openai/gpt-4.1-mini",
            constructor_name: openai_gpt_4_1_mini,
            display_name: "GPT-4.1-mini",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4o {
            model_name: "openai/gpt-4o",
            constructor_name: openai_gpt_4o,
            display_name: "GPT-4o",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4oAug {
            model_name: "openai/gpt-4o-aug",
            constructor_name: openai_gpt_4o_aug,
            display_name: "GPT-4o-Aug",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4oMini {
            model_name: "openai/gpt-4o-mini",
            constructor_name: openai_gpt_4o_mini,
            display_name: "GPT-4o-mini",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt4oSearch {
            model_name: "openai/gpt-4o-search",
            constructor_name: openai_gpt_4o_search,
            display_name: "GPT-4o-Search",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5 {
            model_name: "openai/gpt-5",
            constructor_name: openai_gpt_5,
            display_name: "GPT-5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5Mini {
            model_name: "openai/gpt-5-mini",
            constructor_name: openai_gpt_5_mini,
            display_name: "GPT-5-mini",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5Nano {
            model_name: "openai/gpt-5-nano",
            constructor_name: openai_gpt_5_nano,
            display_name: "GPT-5-nano",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt5Pro {
            model_name: "openai/gpt-5-pro",
            constructor_name: openai_gpt_5_pro,
            display_name: "GPT-5-Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt51 {
            model_name: "openai/gpt-5.1",
            constructor_name: openai_gpt_5_1,
            display_name: "GPT-5.1",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt52 {
            model_name: "openai/gpt-5.2",
            constructor_name: openai_gpt_5_2,
            display_name: "GPT-5.2",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt52Pro {
            model_name: "openai/gpt-5.2-pro",
            constructor_name: openai_gpt_5_2_pro,
            display_name: "GPT-5.2-Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt53Codex {
            model_name: "openai/gpt-5.3-codex",
            constructor_name: openai_gpt_5_3_codex,
            display_name: "GPT-5.3-Codex",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt53CodexSpark {
            model_name: "openai/gpt-5.3-codex-spark",
            constructor_name: openai_gpt_5_3_codex_spark,
            display_name: "GPT-5.3-Codex-Spark",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54 {
            model_name: "openai/gpt-5.4",
            constructor_name: openai_gpt_5_4,
            display_name: "GPT-5.4",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Mini {
            model_name: "openai/gpt-5.4-mini",
            constructor_name: openai_gpt_5_4_mini,
            display_name: "GPT-5.4-Mini",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Nano {
            model_name: "openai/gpt-5.4-nano",
            constructor_name: openai_gpt_5_4_nano,
            display_name: "GPT-5.4-Nano",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt54Pro {
            model_name: "openai/gpt-5.4-pro",
            constructor_name: openai_gpt_5_4_pro,
            display_name: "GPT-5.4-Pro",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt55 {
            model_name: "openai/gpt-5.5",
            constructor_name: openai_gpt_5_5,
            display_name: "GPT-5.5",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt55Pro {
            model_name: "openai/gpt-5.5-pro",
            constructor_name: openai_gpt_5_5_pro,
            display_name: "GPT-5.5-Pro",
            capabilities: [ImageInputSupport, ImageOutputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptImage1Mini {
            model_name: "openai/gpt-image-1-mini",
            constructor_name: openai_gpt_image_1_mini,
            display_name: "GPT-Image-1-Mini",
            capabilities: [ImageInputSupport, ImageOutputSupport, TextInputSupport, ToolCallSupport]
        },
        OpenaiGptImage15 {
            model_name: "openai/gpt-image-1.5",
            constructor_name: openai_gpt_image_1_5,
            display_name: "gpt-image-1.5",
            capabilities: [ImageInputSupport, ImageOutputSupport, TextInputSupport]
        },
        OpenaiGptImage2 {
            model_name: "openai/gpt-image-2",
            constructor_name: openai_gpt_image_2,
            display_name: "GPT-Image-2",
            capabilities: [ImageInputSupport, ImageOutputSupport, TextInputSupport]
        },
        OpenaiO3 {
            model_name: "openai/o3",
            constructor_name: openai_o3,
            display_name: "o3",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiO3Pro {
            model_name: "openai/o3-pro",
            constructor_name: openai_o3_pro,
            display_name: "o3-pro",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        PoetoolsClaudeCode {
            model_name: "poetools/claude-code",
            constructor_name: poetools_claude_code,
            display_name: "claude-code",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        StabilityaiStablediffusionxl {
            model_name: "stabilityai/stablediffusionxl",
            constructor_name: stabilityai_stablediffusionxl,
            display_name: "StableDiffusionXL",
            capabilities: [ImageInputSupport, ImageOutputSupport, TextInputSupport, ToolCallSupport]
        },
        TopazlabsCoTopazlabs {
            model_name: "topazlabs-co/topazlabs",
            constructor_name: topazlabs_co_topazlabs,
            display_name: "TopazLabs",
            capabilities: [ImageInputSupport, ImageOutputSupport, TextInputSupport, ToolCallSupport]
        },
        TrytakoTako {
            model_name: "trytako/tako",
            constructor_name: trytako_tako,
            display_name: "Tako",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok3 {
            model_name: "xai/grok-3",
            constructor_name: xai_grok_3,
            display_name: "Grok 3",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok3Mini {
            model_name: "xai/grok-3-mini",
            constructor_name: xai_grok_3_mini,
            display_name: "Grok 3 Mini",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrok420MultiAgent {
            model_name: "xai/grok-4.20-multi-agent",
            constructor_name: xai_grok_4_20_multi_agent,
            display_name: "Grok-4.20-Multi-Agent",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XaiGrokCodeFast1 {
            model_name: "xai/grok-code-fast-1",
            constructor_name: xai_grok_code_fast_1,
            display_name: "Grok Code Fast 1",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
