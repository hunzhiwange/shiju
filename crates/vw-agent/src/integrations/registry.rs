//! # 集成注册表模块
//!
//! 本模块提供 VibeWindow 支持的所有外部集成和服务的完整目录。
//!
//! ## 功能概述
//!
//! - 定义并返回系统支持的全部集成条目列表
//! - 涵盖聊天通道、AI 模型、生产力工具、智能家居等多个类别
//! - 根据当前配置动态判断每个集成的状态（激活/可用/即将推出）
//!
//! ## 集成类别
//!
//! - **聊天通道 (Chat)**: Telegram、Discord、Slack 等即时通讯平台
//! - **AI 模型 (AiModel)**: OpenRouter、Anthropic、OpenAI 等大语言模型服务
//! - **生产力工具 (Productivity)**: GitHub、Notion、Obsidian 等办公协作工具
//! - **音乐音频 (MusicAudio)**: Spotify、Sonos 等音频服务
//! - **智能家居 (SmartHome)**: Home Assistant、Philips Hue 等家居自动化
//! - **工具自动化 (ToolsAutomation)**: Shell、文件系统、浏览器等本地工具
//! - **媒体创意 (MediaCreative)**: 图像生成、屏幕捕获等创意工具
//! - **社交平台 (Social)**: Twitter/X、电子邮件等社交渠道
//! - **平台支持 (Platform)**: macOS、Linux、Windows 等操作系统支持

use super::{IntegrationCategory, IntegrationEntry, IntegrationStatus};
use crate::app::agent::providers::is_moonshot_alias;

fn default_provider_matches(c: &crate::app::agent::config::Config, aliases: &[&str]) -> bool {
    let Some(provider) = c.default_provider.as_deref() else {
        return false;
    };
    aliases.iter().any(|alias| provider.eq_ignore_ascii_case(alias))
}

/// 返回系统支持的所有集成条目的完整目录
///
/// 该函数构建并返回一个包含所有受支持集成的向量。
/// 每个集成条目包含名称、描述、类别以及状态判断函数。
///
/// # 返回值
///
/// 返回 `Vec<IntegrationEntry>`，包含所有已注册的集成条目。
///
/// # 状态判断逻辑
///
/// 每个集成通过 `status_fn` 闭包根据当前配置动态判断其状态：
/// - `Active`: 集成已配置并正在使用
/// - `Available`: 集成可用但尚未配置
/// - `ComingSoon`: 集成即将推出，暂不可用
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::integrations::registry::all_integrations;
///
/// let integrations = all_integrations();
/// for entry in integrations {
///     println!("{}: {:?}", entry.name, entry.category);
/// }
/// ```
///
/// # 注意事项
///
/// - 该函数在每次调用时都会重新构建向量，适合在需要时按需调用
/// - 状态判断基于传入的配置对象，确保状态始终反映最新配置
#[allow(clippy::too_many_lines)]
pub fn all_integrations() -> Vec<IntegrationEntry> {
    vec![
        // ═══════════════════════════════════════════════════════════
        // 聊天通道集成
        // ═══════════════════════════════════════════════════════════
        IntegrationEntry {
            name: "Telegram",
            description: "Bot API — long-polling",
            category: IntegrationCategory::Chat,
            status_fn: |c| {
                if c.channels_config.telegram.is_some() {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Discord",
            description: "Servers, channels & DMs",
            category: IntegrationCategory::Chat,
            status_fn: |c| {
                if c.channels_config.discord.is_some() {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Slack",
            description: "Workspace apps via Web API",
            category: IntegrationCategory::Chat,
            status_fn: |c| {
                if c.channels_config.slack.is_some() {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Webhooks",
            description: "HTTP endpoint for triggers",
            category: IntegrationCategory::Chat,
            status_fn: |c| {
                if c.channels_config.webhook.is_some() {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "WhatsApp",
            description: "Meta Cloud API via webhook",
            category: IntegrationCategory::Chat,
            status_fn: |c| {
                if c.channels_config.whatsapp.is_some() {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Signal",
            description: "Privacy-focused via signal-cli",
            category: IntegrationCategory::Chat,
            status_fn: |c| {
                if c.channels_config.signal.is_some() {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "iMessage",
            description: "macOS AppleScript bridge",
            category: IntegrationCategory::Chat,
            status_fn: |c| {
                if c.channels_config.imessage.is_some() {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Microsoft Teams",
            description: "Enterprise chat support",
            category: IntegrationCategory::Chat,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Matrix",
            description: "Matrix protocol (Element)",
            category: IntegrationCategory::Chat,
            status_fn: |c| {
                if c.channels_config.matrix.is_some() {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Nostr",
            description: "Decentralized DMs (NIP-04)",
            category: IntegrationCategory::Chat,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "WebChat",
            description: "Browser-based chat UI",
            category: IntegrationCategory::Chat,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Nextcloud Talk",
            description: "Self-hosted Nextcloud chat",
            category: IntegrationCategory::Chat,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Zalo",
            description: "Zalo Bot API",
            category: IntegrationCategory::Chat,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "DingTalk",
            description: "DingTalk Stream Mode",
            category: IntegrationCategory::Chat,
            status_fn: |c| {
                if c.channels_config.dingtalk.is_some() {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "QQ Official",
            description: "Tencent QQ Bot SDK",
            category: IntegrationCategory::Chat,
            status_fn: |c| {
                if c.channels_config.qq.is_some() {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        // ═══════════════════════════════════════════════════════════
        // AI 模型集成
        // ═══════════════════════════════════════════════════════════
        IntegrationEntry {
            name: "OpenRouter",
            description: "Claude Sonnet 4.6, GPT-5.2, Gemini 3.1 Pro",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_provider.as_deref() == Some("openrouter") && c.api_key.is_some() {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Anthropic",
            description: "Claude Sonnet 4.6, Claude Opus 4.6",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_provider.as_deref() == Some("anthropic") {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "OpenAI",
            description: "GPT-5.2, GPT-5.2-Codex",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_provider.as_deref() == Some("openai") {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Google",
            description: "Gemini 3.1 Pro, Gemini 3 Flash",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_model.as_deref().is_some_and(|m| m.starts_with("google/")) {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "DeepSeek",
            description: "DeepSeek-Reasoner, DeepSeek-Chat",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_model.as_deref().is_some_and(|m| m.starts_with("deepseek/")) {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "xAI",
            description: "Grok 4, Grok 3",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_model.as_deref().is_some_and(|m| m.starts_with("x-ai/")) {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Mistral",
            description: "Mistral Large Latest, Codestral",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_model.as_deref().is_some_and(|m| m.starts_with("mistral")) {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Ollama",
            description: "Local models (Llama, etc.)",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_provider.as_deref() == Some("ollama") {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Perplexity",
            description: "Sonar Pro, Sonar Reasoning Pro",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_provider.as_deref() == Some("perplexity") {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Hugging Face",
            description: "Open-source models",
            category: IntegrationCategory::AiModel,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "LM Studio",
            description: "Local model server",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_provider.as_deref().is_some_and(|provider| {
                    provider.eq_ignore_ascii_case("lmstudio")
                        || provider.eq_ignore_ascii_case("lm-studio")
                }) {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Venice",
            description: "Venice Llama 3.3 70B and frontier blends",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_provider.as_deref() == Some("venice") {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Vercel AI",
            description: "Gateway for GPT-5.2 and multi-provider routing",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_provider.as_deref() == Some("vercel") {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Cloudflare AI",
            description: "Workers AI + Llama 3.3 / gateway routing",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_provider.as_deref() == Some("cloudflare") {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Moonshot",
            description: "Kimi 2.5 and Kimi Coding",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_provider.as_deref().is_some_and(is_moonshot_alias) {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Synthetic",
            description: "Synthetic-1 and synthetic family models",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_provider.as_deref() == Some("synthetic") {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "OpenCode Zen",
            description: "OpenCode Zen and coding-specialized models",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if c.default_provider.as_deref() == Some("opencode") {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Z.AI",
            description: "GLM 4.7 and Z.AI hosted variants",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if default_provider_matches(c, &["zai", "zai-cn"]) {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "GLM",
            description: "GLM 4.7 and GLM 4.5 family",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if default_provider_matches(c, &["glm", "glm-cn"]) {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "MiniMax",
            description: "MiniMax M1 and latest multimodal variants",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if default_provider_matches(c, &["minimax", "minimax-cn"]) {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Qwen",
            description: "Qwen Max and Qwen reasoning families",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if default_provider_matches(c, &["qwen", "qwen-intl", "dashscope"]) {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Amazon Bedrock",
            description: "Claude Sonnet 4.5 and Bedrock model catalog",
            category: IntegrationCategory::AiModel,
            status_fn: |_c| IntegrationStatus::Available,
        },
        IntegrationEntry {
            name: "Qianfan",
            description: "ERNIE 4.x and Qianfan model catalog",
            category: IntegrationCategory::AiModel,
            status_fn: |c| {
                if default_provider_matches(c, &["baidu", "qianfan"]) {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Groq",
            description: "Llama 3.3 70B Versatile and low-latency models",
            category: IntegrationCategory::AiModel,
            status_fn: |_c| IntegrationStatus::Active,
        },
        IntegrationEntry {
            name: "Together AI",
            description: "Llama 3.3 70B Turbo and open model hosting",
            category: IntegrationCategory::AiModel,
            status_fn: |_c| IntegrationStatus::Available,
        },
        IntegrationEntry {
            name: "Fireworks AI",
            description: "DeepSeek / Llama high-throughput inference",
            category: IntegrationCategory::AiModel,
            status_fn: |_c| IntegrationStatus::Available,
        },
        IntegrationEntry {
            name: "Cohere",
            description: "Command R+ (08-2024) and embedding models",
            category: IntegrationCategory::AiModel,
            status_fn: |_c| IntegrationStatus::Available,
        },
        // ═══════════════════════════════════════════════════════════
        // 生产力工具集成
        // ═══════════════════════════════════════════════════════════
        IntegrationEntry {
            name: "GitHub",
            description: "Code, issues, PRs",
            category: IntegrationCategory::Productivity,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Notion",
            description: "Workspace & databases",
            category: IntegrationCategory::Productivity,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Apple Notes",
            description: "Native macOS/iOS notes",
            category: IntegrationCategory::Productivity,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Apple Reminders",
            description: "Task management",
            category: IntegrationCategory::Productivity,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Obsidian",
            description: "Knowledge graph notes",
            category: IntegrationCategory::Productivity,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Things 3",
            description: "GTD task manager",
            category: IntegrationCategory::Productivity,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Bear Notes",
            description: "Markdown notes",
            category: IntegrationCategory::Productivity,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Trello",
            description: "Kanban boards",
            category: IntegrationCategory::Productivity,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Linear",
            description: "Issue tracking",
            category: IntegrationCategory::Productivity,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        // ═══════════════════════════════════════════════════════════
        // 音乐与音频集成
        // ═══════════════════════════════════════════════════════════
        IntegrationEntry {
            name: "Spotify",
            description: "Music playback control",
            category: IntegrationCategory::MusicAudio,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Sonos",
            description: "Multi-room audio",
            category: IntegrationCategory::MusicAudio,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Shazam",
            description: "Song recognition",
            category: IntegrationCategory::MusicAudio,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        // ═══════════════════════════════════════════════════════════
        // 智能家居集成
        // ═══════════════════════════════════════════════════════════
        IntegrationEntry {
            name: "Home Assistant",
            description: "Home automation hub",
            category: IntegrationCategory::SmartHome,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Philips Hue",
            description: "Smart lighting",
            category: IntegrationCategory::SmartHome,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "8Sleep",
            description: "Smart mattress",
            category: IntegrationCategory::SmartHome,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        // ═══════════════════════════════════════════════════════════
        // 工具与自动化集成
        // ═══════════════════════════════════════════════════════════
        IntegrationEntry {
            name: "Browser",
            description: "Chrome/Chromium control",
            category: IntegrationCategory::ToolsAutomation,
            status_fn: |_| IntegrationStatus::Available,
        },
        IntegrationEntry {
            name: "Shell",
            description: "Terminal command execution",
            category: IntegrationCategory::ToolsAutomation,
            status_fn: |_| IntegrationStatus::Active,
        },
        IntegrationEntry {
            name: "File System",
            description: "Read/write files",
            category: IntegrationCategory::ToolsAutomation,
            status_fn: |_| IntegrationStatus::Active,
        },
        IntegrationEntry {
            name: "Cron",
            description: "Scheduled tasks",
            category: IntegrationCategory::ToolsAutomation,
            status_fn: |_| IntegrationStatus::Available,
        },
        IntegrationEntry {
            name: "Voice",
            description: "Voice wake + talk mode",
            category: IntegrationCategory::ToolsAutomation,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Gmail",
            description: "Email triggers & send",
            category: IntegrationCategory::ToolsAutomation,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "1Password",
            description: "Secure credentials",
            category: IntegrationCategory::ToolsAutomation,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Weather",
            description: "Forecasts & conditions",
            category: IntegrationCategory::ToolsAutomation,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Canvas",
            description: "Visual workspace + A2UI",
            category: IntegrationCategory::ToolsAutomation,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        // ═══════════════════════════════════════════════════════════
        // 媒体与创意工具集成
        // ═══════════════════════════════════════════════════════════
        IntegrationEntry {
            name: "Image Gen",
            description: "AI image generation",
            category: IntegrationCategory::MediaCreative,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "GIF Search",
            description: "Find the perfect GIF",
            category: IntegrationCategory::MediaCreative,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Screen Capture",
            description: "Screenshot & screen control",
            category: IntegrationCategory::MediaCreative,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Camera",
            description: "Photo/video capture",
            category: IntegrationCategory::MediaCreative,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        // ═══════════════════════════════════════════════════════════
        // 社交平台集成
        // ═══════════════════════════════════════════════════════════
        IntegrationEntry {
            name: "Twitter/X",
            description: "Tweet, reply, search",
            category: IntegrationCategory::Social,
            status_fn: |_| IntegrationStatus::ComingSoon,
        },
        IntegrationEntry {
            name: "Email",
            description: "IMAP/SMTP email channel",
            category: IntegrationCategory::Social,
            status_fn: |c| {
                // 电子邮件集成在非 WASM 目标平台可用
                #[cfg(not(target_arch = "wasm32"))]
                if c.channels_config.email.is_some() {
                    IntegrationStatus::Active
                } else {
                    IntegrationStatus::Available
                }
                // WASM 目标平台暂不支持
                #[cfg(target_arch = "wasm32")]
                IntegrationStatus::Available
            },
        },
        // ═══════════════════════════════════════════════════════════
        // 操作系统平台集成
        // ═══════════════════════════════════════════════════════════
        IntegrationEntry {
            name: "macOS",
            description: "Native support + AppleScript",
            category: IntegrationCategory::Platform,
            status_fn: |_| {
                #[cfg(target_os = "macos")]
                {
                    IntegrationStatus::Active
                }
                #[cfg(not(target_os = "macos"))]
                {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Linux",
            description: "Native support",
            category: IntegrationCategory::Platform,
            status_fn: |_| {
                #[cfg(target_os = "linux")]
                {
                    IntegrationStatus::Active
                }
                #[cfg(not(target_os = "linux"))]
                {
                    IntegrationStatus::Available
                }
            },
        },
        IntegrationEntry {
            name: "Windows",
            description: "WSL2 recommended",
            category: IntegrationCategory::Platform,
            status_fn: |_| IntegrationStatus::Available,
        },
        IntegrationEntry {
            name: "iOS",
            description: "Chat via Telegram/Discord",
            category: IntegrationCategory::Platform,
            status_fn: |_| IntegrationStatus::Available,
        },
        IntegrationEntry {
            name: "Android",
            description: "Chat via Telegram/Discord",
            category: IntegrationCategory::Platform,
            status_fn: |_| IntegrationStatus::Available,
        },
    ]
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
