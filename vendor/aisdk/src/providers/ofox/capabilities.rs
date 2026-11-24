//! Capabilities for ofox models.
//!
//! This module defines model types and their capabilities for ofox providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::ofox::Ofox;

model_capabilities! {
    provider: Ofox,
    models: {
        AnthropicClaudeFable5 {
            model_name: "anthropic/claude-fable-5",
            constructor_name: anthropic_claude_fable_5,
            display_name: "Claude Fable 5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeFable51 {
            model_name: "anthropic/claude-fable-5.1",
            constructor_name: anthropic_claude_fable_5_1,
            display_name: "Claude Fable 5.1",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeHaiku45 {
            model_name: "anthropic/claude-haiku-4.5",
            constructor_name: anthropic_claude_haiku_4_5,
            display_name: "Claude Haiku 4.5 (latest)",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeHaiku55 {
            model_name: "anthropic/claude-haiku-5.5",
            constructor_name: anthropic_claude_haiku_5_5,
            display_name: "Claude Haiku 5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus46 {
            model_name: "anthropic/claude-opus-4.6",
            constructor_name: anthropic_claude_opus_4_6,
            display_name: "Claude Opus 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus47 {
            model_name: "anthropic/claude-opus-4.7",
            constructor_name: anthropic_claude_opus_4_7,
            display_name: "Claude Opus 4.7",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus48 {
            model_name: "anthropic/claude-opus-4.8",
            constructor_name: anthropic_claude_opus_4_8,
            display_name: "Claude Opus 4.8",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus5 {
            model_name: "anthropic/claude-opus-5",
            constructor_name: anthropic_claude_opus_5,
            display_name: "Claude Opus 5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeOpus55 {
            model_name: "anthropic/claude-opus-5.5",
            constructor_name: anthropic_claude_opus_5_5,
            display_name: "Claude Opus 5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet46 {
            model_name: "anthropic/claude-sonnet-4.6",
            constructor_name: anthropic_claude_sonnet_4_6,
            display_name: "Claude Sonnet 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet5 {
            model_name: "anthropic/claude-sonnet-5",
            constructor_name: anthropic_claude_sonnet_5,
            display_name: "Claude Sonnet 5",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        AnthropicClaudeSonnet55 {
            model_name: "anthropic/claude-sonnet-5.5",
            constructor_name: anthropic_claude_sonnet_5_5,
            display_name: "Claude Sonnet 5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BailianQwenFlash {
            model_name: "bailian/qwen-flash",
            constructor_name: bailian_qwen_flash,
            display_name: "Qwen Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BailianQwenMax {
            model_name: "bailian/qwen-max",
            constructor_name: bailian_qwen_max,
            display_name: "Qwen Max",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BailianQwenPlus {
            model_name: "bailian/qwen-plus",
            constructor_name: bailian_qwen_plus,
            display_name: "Qwen Plus",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BailianQwenTurbo {
            model_name: "bailian/qwen-turbo",
            constructor_name: bailian_qwen_turbo,
            display_name: "Qwen Turbo",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BailianQwenVlMax {
            model_name: "bailian/qwen-vl-max",
            constructor_name: bailian_qwen_vl_max,
            display_name: "Qwen-VL Max",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BailianQwen3CoderFlash {
            model_name: "bailian/qwen3-coder-flash",
            constructor_name: bailian_qwen3_coder_flash,
            display_name: "Qwen3 Coder Flash",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BailianQwen3CoderNext {
            model_name: "bailian/qwen3-coder-next",
            constructor_name: bailian_qwen3_coder_next,
            display_name: "Qwen3 Coder Next",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BailianQwen3CoderPlus {
            model_name: "bailian/qwen3-coder-plus",
            constructor_name: bailian_qwen3_coder_plus,
            display_name: "Qwen3 Coder Plus",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BailianQwen3Max {
            model_name: "bailian/qwen3-max",
            constructor_name: bailian_qwen3_max,
            display_name: "Qwen3 Max",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BailianQwen35122bA10b {
            model_name: "bailian/qwen3.5-122b-a10b",
            constructor_name: bailian_qwen3_5_122b_a10b,
            display_name: "Qwen3.5 122B-A10B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BailianQwen3527b {
            model_name: "bailian/qwen3.5-27b",
            constructor_name: bailian_qwen3_5_27b,
            display_name: "Qwen3.5 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BailianQwen3535bA3b {
            model_name: "bailian/qwen3.5-35b-a3b",
            constructor_name: bailian_qwen3_5_35b_a3b,
            display_name: "Qwen3.5 35B-A3B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BailianQwen35397bA17b {
            model_name: "bailian/qwen3.5-397b-a17b",
            constructor_name: bailian_qwen3_5_397b_a17b,
            display_name: "Qwen3.5 397B-A17B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BailianQwen35Flash {
            model_name: "bailian/qwen3.5-flash",
            constructor_name: bailian_qwen3_5_flash,
            display_name: "Qwen3.5 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BailianQwen35Plus {
            model_name: "bailian/qwen3.5-plus",
            constructor_name: bailian_qwen3_5_plus,
            display_name: "Qwen3.5 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BailianQwen3627b {
            model_name: "bailian/qwen3.6-27b",
            constructor_name: bailian_qwen3_6_27b,
            display_name: "Qwen3.6 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BailianQwen36Flash {
            model_name: "bailian/qwen3.6-flash",
            constructor_name: bailian_qwen3_6_flash,
            display_name: "Qwen3.6 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BailianQwen36MaxPreview {
            model_name: "bailian/qwen3.6-max-preview",
            constructor_name: bailian_qwen3_6_max_preview,
            display_name: "Qwen3.6 Max Preview",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BailianQwen36Plus {
            model_name: "bailian/qwen3.6-plus",
            constructor_name: bailian_qwen3_6_plus,
            display_name: "Qwen3.6 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BailianQwen37Max {
            model_name: "bailian/qwen3.7-max",
            constructor_name: bailian_qwen3_7_max,
            display_name: "Qwen3.7 Max",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        BailianQwen37Plus {
            model_name: "bailian/qwen3.7-plus",
            constructor_name: bailian_qwen3_7_plus,
            display_name: "Qwen3.7 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BailianQwen3827b {
            model_name: "bailian/qwen3.8-27b",
            constructor_name: bailian_qwen3_8_27b,
            display_name: "Qwen3.8 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BailianQwen38Flash {
            model_name: "bailian/qwen3.8-flash",
            constructor_name: bailian_qwen3_8_flash,
            display_name: "Qwen3.8 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BailianQwen38Max {
            model_name: "bailian/qwen3.8-max",
            constructor_name: bailian_qwen3_8_max,
            display_name: "Qwen3.8 Max",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        BailianQwen38Max0902 {
            model_name: "bailian/qwen3.8-max-0902",
            constructor_name: bailian_qwen3_8_max_0902,
            display_name: "Qwen3.8 Max 0902",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DeepseekDeepseekV32 {
            model_name: "deepseek/deepseek-v3.2",
            constructor_name: deepseek_deepseek_v3_2,
            display_name: "DeepSeek V3.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV4Flash {
            model_name: "deepseek/deepseek-v4-flash",
            constructor_name: deepseek_deepseek_v4_flash,
            display_name: "DeepSeek V4 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV4Flash0423 {
            model_name: "deepseek/deepseek-v4-flash-0423",
            constructor_name: deepseek_deepseek_v4_flash_0423,
            display_name: "DeepSeek V4 Flash 0423",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV4Flash0731 {
            model_name: "deepseek/deepseek-v4-flash-0731",
            constructor_name: deepseek_deepseek_v4_flash_0731,
            display_name: "DeepSeek V4 Flash 0731",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV4Pro {
            model_name: "deepseek/deepseek-v4-pro",
            constructor_name: deepseek_deepseek_v4_pro,
            display_name: "DeepSeek V4 Pro",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV4Pro0423 {
            model_name: "deepseek/deepseek-v4-pro-0423",
            constructor_name: deepseek_deepseek_v4_pro_0423,
            display_name: "DeepSeek V4 Pro 0423",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV4Pro0813 {
            model_name: "deepseek/deepseek-v4-pro-0813",
            constructor_name: deepseek_deepseek_v4_pro_0813,
            display_name: "DeepSeek V4 Pro 0813",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV41Flash {
            model_name: "deepseek/deepseek-v4.1-flash",
            constructor_name: deepseek_deepseek_v4_1_flash,
            display_name: "DeepSeek V4.1 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DeepseekDeepseekV41Flash1017 {
            model_name: "deepseek/deepseek-v4.1-flash-1017",
            constructor_name: deepseek_deepseek_v4_1_flash_1017,
            display_name: "DeepSeek V4.1 Flash 1017",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemini25Flash {
            model_name: "google/gemini-2.5-flash",
            constructor_name: google_gemini_2_5_flash,
            display_name: "Gemini 2.5 Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini25FlashLite {
            model_name: "google/gemini-2.5-flash-lite",
            constructor_name: google_gemini_2_5_flash_lite,
            display_name: "Gemini 2.5 Flash-Lite",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini25Pro {
            model_name: "google/gemini-2.5-pro",
            constructor_name: google_gemini_2_5_pro,
            display_name: "Gemini 2.5 Pro",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini3FlashPreview {
            model_name: "google/gemini-3-flash-preview",
            constructor_name: google_gemini_3_flash_preview,
            display_name: "Gemini 3 Flash Preview",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini31FlashLite {
            model_name: "google/gemini-3.1-flash-lite",
            constructor_name: google_gemini_3_1_flash_lite,
            display_name: "Gemini 3.1 Flash Lite",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        GoogleGemini31ProPreview {
            model_name: "google/gemini-3.1-pro-preview",
            constructor_name: google_gemini_3_1_pro_preview,
            display_name: "Gemini 3.1 Pro Preview",
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
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        GoogleGemini4Argon {
            model_name: "google/gemini-4-argon",
            constructor_name: google_gemini_4_argon,
            display_name: "Gemini 4 Argon",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MinimaxM2Her {
            model_name: "minimax/m2-her",
            constructor_name: minimax_m2_her,
            display_name: "MiniMax-M2 Her",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM2 {
            model_name: "minimax/minimax-m2",
            constructor_name: minimax_minimax_m2,
            display_name: "MiniMax-M2",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM21 {
            model_name: "minimax/minimax-m2.1",
            constructor_name: minimax_minimax_m2_1,
            display_name: "MiniMax-M2.1",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM21Lightning {
            model_name: "minimax/minimax-m2.1-lightning",
            constructor_name: minimax_minimax_m2_1_lightning,
            display_name: "MiniMax-M2.1 Lightning",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM25 {
            model_name: "minimax/minimax-m2.5",
            constructor_name: minimax_minimax_m2_5,
            display_name: "MiniMax-M2.5",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM25Lightning {
            model_name: "minimax/minimax-m2.5-lightning",
            constructor_name: minimax_minimax_m2_5_lightning,
            display_name: "MiniMax-M2.5 Lightning",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM27 {
            model_name: "minimax/minimax-m2.7",
            constructor_name: minimax_minimax_m2_7,
            display_name: "MiniMax-M2.7",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM27Highspeed {
            model_name: "minimax/minimax-m2.7-highspeed",
            constructor_name: minimax_minimax_m2_7_highspeed,
            display_name: "MiniMax-M2.7-highspeed",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MinimaxMinimaxM3 {
            model_name: "minimax/minimax-m3",
            constructor_name: minimax_minimax_m3,
            display_name: "MiniMax-M3",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MoonshotaiKimiK26 {
            model_name: "moonshotai/kimi-k2.6",
            constructor_name: moonshotai_kimi_k2_6,
            display_name: "Kimi K2.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MoonshotaiKimiK27Code {
            model_name: "moonshotai/kimi-k2.7-code",
            constructor_name: moonshotai_kimi_k2_7_code,
            display_name: "Kimi K2.7 Code",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MoonshotaiKimiK27CodeHighspeed {
            model_name: "moonshotai/kimi-k2.7-code-highspeed",
            constructor_name: moonshotai_kimi_k2_7_code_highspeed,
            display_name: "Kimi K2.7 Code Highspeed",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MoonshotaiKimiK3 {
            model_name: "moonshotai/kimi-k3",
            constructor_name: moonshotai_kimi_k3,
            display_name: "Kimi K3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
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
        OpenaiGpt4o {
            model_name: "openai/gpt-4o",
            constructor_name: openai_gpt_4o,
            display_name: "GPT-4o",
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
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt51 {
            model_name: "openai/gpt-5.1",
            constructor_name: openai_gpt_5_1,
            display_name: "GPT-5.1",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt51CodexMax {
            model_name: "openai/gpt-5.1-codex-max",
            constructor_name: openai_gpt_5_1_codex_max,
            display_name: "GPT-5.1 Codex Max",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt51CodexMini {
            model_name: "openai/gpt-5.1-codex-mini",
            constructor_name: openai_gpt_5_1_codex_mini,
            display_name: "GPT-5.1 Codex mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt52 {
            model_name: "openai/gpt-5.2",
            constructor_name: openai_gpt_5_2,
            display_name: "GPT-5.2",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt52Codex {
            model_name: "openai/gpt-5.2-codex",
            constructor_name: openai_gpt_5_2_codex,
            display_name: "GPT-5.2 Codex",
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
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGpt55 {
            model_name: "openai/gpt-5.5",
            constructor_name: openai_gpt_5_5,
            display_name: "GPT-5.5",
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
        QwenQwenFlash {
            model_name: "qwen/qwen-flash",
            constructor_name: qwen_qwen_flash,
            display_name: "Qwen Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwenMax {
            model_name: "qwen/qwen-max",
            constructor_name: qwen_qwen_max,
            display_name: "Qwen Max",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwenPlus {
            model_name: "qwen/qwen-plus",
            constructor_name: qwen_qwen_plus,
            display_name: "Qwen Plus",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwenTurbo {
            model_name: "qwen/qwen-turbo",
            constructor_name: qwen_qwen_turbo,
            display_name: "Qwen Turbo",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwenVlMax {
            model_name: "qwen/qwen-vl-max",
            constructor_name: qwen_qwen_vl_max,
            display_name: "Qwen-VL Max",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3CoderFlash {
            model_name: "qwen/qwen3-coder-flash",
            constructor_name: qwen_qwen3_coder_flash,
            display_name: "Qwen3 Coder Flash",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3CoderNext {
            model_name: "qwen/qwen3-coder-next",
            constructor_name: qwen_qwen3_coder_next,
            display_name: "Qwen3 Coder Next",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3CoderPlus {
            model_name: "qwen/qwen3-coder-plus",
            constructor_name: qwen_qwen3_coder_plus,
            display_name: "Qwen3 Coder Plus",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3Max {
            model_name: "qwen/qwen3-max",
            constructor_name: qwen_qwen3_max,
            display_name: "Qwen3 Max",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen35122bA10b {
            model_name: "qwen/qwen3.5-122b-a10b",
            constructor_name: qwen_qwen3_5_122b_a10b,
            display_name: "Qwen3.5 122B-A10B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3527b {
            model_name: "qwen/qwen3.5-27b",
            constructor_name: qwen_qwen3_5_27b,
            display_name: "Qwen3.5 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen3535bA3b {
            model_name: "qwen/qwen3.5-35b-a3b",
            constructor_name: qwen_qwen3_5_35b_a3b,
            display_name: "Qwen3.5 35B-A3B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen35397bA17b {
            model_name: "qwen/qwen3.5-397b-a17b",
            constructor_name: qwen_qwen3_5_397b_a17b,
            display_name: "Qwen3.5 397B-A17B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen35Flash {
            model_name: "qwen/qwen3.5-flash",
            constructor_name: qwen_qwen3_5_flash,
            display_name: "Qwen3.5 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen35Plus {
            model_name: "qwen/qwen3.5-plus",
            constructor_name: qwen_qwen3_5_plus,
            display_name: "Qwen3.5 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen3627b {
            model_name: "qwen/qwen3.6-27b",
            constructor_name: qwen_qwen3_6_27b,
            display_name: "Qwen3.6 27B",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen36Flash {
            model_name: "qwen/qwen3.6-flash",
            constructor_name: qwen_qwen3_6_flash,
            display_name: "Qwen3.6 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen36MaxPreview {
            model_name: "qwen/qwen3.6-max-preview",
            constructor_name: qwen_qwen3_6_max_preview,
            display_name: "Qwen3.6 Max Preview",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen36Plus {
            model_name: "qwen/qwen3.6-plus",
            constructor_name: qwen_qwen3_6_plus,
            display_name: "Qwen3.6 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        QwenQwen37Max {
            model_name: "qwen/qwen3.7-max",
            constructor_name: qwen_qwen3_7_max,
            display_name: "Qwen3.7 Max",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        QwenQwen37Plus {
            model_name: "qwen/qwen3.7-plus",
            constructor_name: qwen_qwen3_7_plus,
            display_name: "Qwen3.7 Plus",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
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
        VolcengineDoubaoSeed20Code {
            model_name: "volcengine/doubao-seed-2.0-code",
            constructor_name: volcengine_doubao_seed_2_0_code,
            display_name: "Seed 2.0 Code",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VolcengineDoubaoSeed20Lite {
            model_name: "volcengine/doubao-seed-2.0-lite",
            constructor_name: volcengine_doubao_seed_2_0_lite,
            display_name: "Seed 2.0 Lite",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VolcengineDoubaoSeed20Mini {
            model_name: "volcengine/doubao-seed-2.0-mini",
            constructor_name: volcengine_doubao_seed_2_0_mini,
            display_name: "Seed 2.0 Mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VolcengineDoubaoSeed20Pro {
            model_name: "volcengine/doubao-seed-2.0-pro",
            constructor_name: volcengine_doubao_seed_2_0_pro,
            display_name: "Seed 2.0 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VolcengineDoubaoSeed21Pro {
            model_name: "volcengine/doubao-seed-2.1-pro",
            constructor_name: volcengine_doubao_seed_2_1_pro,
            display_name: "Seed 2.1 Pro",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VolcengineDoubaoSeed21Turbo {
            model_name: "volcengine/doubao-seed-2.1-turbo",
            constructor_name: volcengine_doubao_seed_2_1_turbo,
            display_name: "Seed 2.1 Turbo",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VolcengineDoubaoSeedCharacter {
            model_name: "volcengine/doubao-seed-character",
            constructor_name: volcengine_doubao_seed_character,
            display_name: "Seed Character",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        VolcengineDoubaoSeedEvolving {
            model_name: "volcengine/doubao-seed-evolving",
            constructor_name: volcengine_doubao_seed_evolving,
            display_name: "Seed Evolving",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        XAiGrok41Fast {
            model_name: "x-ai/grok-4.1-fast",
            constructor_name: x_ai_grok_4_1_fast,
            display_name: "Grok 4.1 Fast",
            capabilities: [ImageInputSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XAiGrok420 {
            model_name: "x-ai/grok-4.20",
            constructor_name: x_ai_grok_4_20,
            display_name: "Grok 4.20 (Reasoning)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XAiGrok43 {
            model_name: "x-ai/grok-4.3",
            constructor_name: x_ai_grok_4_3,
            display_name: "Grok 4.3",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XAiGrok45 {
            model_name: "x-ai/grok-4.5",
            constructor_name: x_ai_grok_4_5,
            display_name: "Grok 4.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XAiGrok46 {
            model_name: "x-ai/grok-4.6",
            constructor_name: x_ai_grok_4_6,
            display_name: "Grok 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        XAiGrok47 {
            model_name: "x-ai/grok-4.7",
            constructor_name: x_ai_grok_4_7,
            display_name: "Grok 4.7",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZAiGlm46 {
            model_name: "z-ai/glm-4.6",
            constructor_name: z_ai_glm_4_6,
            display_name: "GLM-4.6",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZAiGlm47 {
            model_name: "z-ai/glm-4.7",
            constructor_name: z_ai_glm_4_7,
            display_name: "GLM-4.7",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZAiGlm47Flashx {
            model_name: "z-ai/glm-4.7-flashx",
            constructor_name: z_ai_glm_4_7_flashx,
            display_name: "GLM-4.7-FlashX",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZAiGlm5 {
            model_name: "z-ai/glm-5",
            constructor_name: z_ai_glm_5,
            display_name: "GLM-5",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZAiGlm5Turbo {
            model_name: "z-ai/glm-5-turbo",
            constructor_name: z_ai_glm_5_turbo,
            display_name: "GLM-5-Turbo",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZAiGlm51 {
            model_name: "z-ai/glm-5.1",
            constructor_name: z_ai_glm_5_1,
            display_name: "GLM-5.1",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZAiGlm52 {
            model_name: "z-ai/glm-5.2",
            constructor_name: z_ai_glm_5_2,
            display_name: "GLM-5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZAiGlm53 {
            model_name: "z-ai/glm-5.3",
            constructor_name: z_ai_glm_5_3,
            display_name: "GLM-5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZAiGlm53Flash {
            model_name: "z-ai/glm-5.3-flash",
            constructor_name: z_ai_glm_5_3_flash,
            display_name: "GLM-5.3-Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ZAiGlm53Flashx {
            model_name: "z-ai/glm-5.3-flashx",
            constructor_name: z_ai_glm_5_3_flashx,
            display_name: "GLM-5.3-FlashX",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        ZAiGlm5vTurbo {
            model_name: "z-ai/glm-5v-turbo",
            constructor_name: z_ai_glm_5v_turbo,
            display_name: "GLM-5V-Turbo",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
