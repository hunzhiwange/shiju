//! 集成注册表单元测试模块
//!
//! 本模块包含针对 `registry` 模块的全面测试套件，验证集成注册表的各项功能：
//!
//! # 测试范围
//!
//! - 注册表基本属性（条目数量、分类覆盖、名称唯一性）
//! - 集成状态判断逻辑（Active/Available/ComingSoon）
//! - 特定集成的配置激活行为
//! - 平台相关集成的条件激活
//! - 区域化 Provider 别名支持
//!
//! # 测试策略
//!
//! 测试分为以下几类：
//! - 健全性测试：验证注册表基本结构和数据完整性
//! - 状态转换测试：验证配置变化如何影响集成状态
//! - 平台适配测试：验证平台相关集成的行为
//! - 别名解析测试：验证 Provider 别名正确激活对应集成

use super::*;
use crate::app::agent::config::Config;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::config::schema::EmailConfig;
use crate::app::agent::config::schema::{
    DingTalkConfig, DiscordConfig, IMessageConfig, MatrixConfig, QQConfig, QQReceiveMode,
    SignalConfig, SlackConfig, StreamMode, TelegramConfig, WebhookConfig, WhatsAppConfig,
};

/// 验证注册表包含足够的集成条目
///
/// 此测试确保注册表中至少包含 50 个集成条目，
/// 作为对集成生态规模的基本保证。
///
/// # 断言
///
/// - `entries.len() >= 50`：注册表条目数量不少于 50
#[test]
fn registry_has_entries() {
    let entries = all_integrations();
    assert!(entries.len() >= 50, "Expected 50+ integrations, got {}", entries.len());
}

/// 验证所有分类都有对应的集成条目
///
/// 此测试确保 `IntegrationCategory::all()` 返回的每个分类
/// 都在注册表中至少有一个对应的集成条目。
///
/// # 验证逻辑
///
/// 遍历所有分类，检查每个分类下至少有一个集成条目。
/// 这防止了出现"空分类"的情况。
///
/// # 断言
///
/// - 每个分类的条目计数 `count > 0`
#[test]
fn all_categories_represented() {
    let entries = all_integrations();
    for cat in IntegrationCategory::all() {
        let count = entries.iter().filter(|e| e.category == *cat).count();
        assert!(count > 0, "Category {cat:?} has no entries");
    }
}

/// 验证所有集成的状态函数不会 panic
///
/// 此测试确保每个集成的 `status_fn` 在使用默认配置调用时
/// 能够安全执行，不会发生 panic。
///
/// # 安全性
///
/// 状态函数应该是稳健的，即使在配置缺失的情况下
/// 也应返回合理的默认状态而非 panic。
#[test]
fn status_functions_dont_panic() {
    let config = Config::default();
    let entries = all_integrations();
    for entry in &entries {
        let _ = (entry.status_fn)(&config);
    }
}

/// 验证注册表中没有重复的集成名称
///
/// 此测试确保每个集成的 `name` 字段在整个注册表中是唯一的，
/// 防止因名称冲突导致的集成识别问题。
///
/// # 实现方式
///
/// 使用 HashSet 追踪已见名称，遇到重复时立即断言失败。
#[test]
fn no_duplicate_names() {
    let entries = all_integrations();
    let mut seen = std::collections::HashSet::new();
    for entry in &entries {
        assert!(seen.insert(entry.name), "Duplicate integration name: {}", entry.name);
    }
}

/// 验证所有集成都有非空的名称和描述
///
/// 此测试确保注册表中不存在名称或描述为空字符串的集成条目，
/// 保证用户界面显示的数据完整性。
///
/// # 断言
///
/// - 每个集成的 `name` 非空
/// - 每个集成的 `description` 非空
#[test]
fn no_empty_names_or_descriptions() {
    let entries = all_integrations();
    for entry in &entries {
        assert!(!entry.name.is_empty(), "Found integration with empty name");
        assert!(
            !entry.description.is_empty(),
            "Integration '{}' has empty description",
            entry.name
        );
    }
}

/// 验证 Telegram 在配置后状态变为 Active
///
/// 此测试验证当配置中包含有效的 Telegram 配置时，
/// Telegram 集成的状态应从 Available 转换为 Active。
///
/// # 测试配置
///
/// 使用包含以下字段的 `TelegramConfig`：
/// - `bot_token`: "123:ABC"（测试令牌）
/// - `allowed_users`: ["user"]
/// - 其他字段使用默认或非空值
///
/// # 预期结果
///
/// - 状态应为 `IntegrationStatus::Active`
#[test]
fn telegram_active_when_configured() {
    let mut config = Config::default();
    config.channels_config.telegram = Some(TelegramConfig {
        bot_token: "123:ABC".into(),
        allowed_users: vec!["user".into()],
        stream_mode: StreamMode::default(),
        draft_update_interval_ms: 1000,
        interrupt_on_new_message: false,
        mention_only: false,
        group_reply: None,
        base_url: None,
    });
    let entries = all_integrations();
    let tg = entries.iter().find(|e| e.name == "Telegram").unwrap();
    assert!(matches!((tg.status_fn)(&config), IntegrationStatus::Active));
}

/// 验证 Telegram 在未配置时状态为 Available
///
/// 此测试验证当配置中没有 Telegram 配置时，
/// Telegram 集成应保持 Available 状态（可配置但未激活）。
///
/// # 预期结果
///
/// - 状态应为 `IntegrationStatus::Available`
#[test]
fn telegram_available_when_not_configured() {
    let config = Config::default();
    let entries = all_integrations();
    let tg = entries.iter().find(|e| e.name == "Telegram").unwrap();
    assert!(matches!((tg.status_fn)(&config), IntegrationStatus::Available));
}

/// 验证 iMessage 在配置后状态变为 Active
///
/// 此测试验证当配置中包含有效的 iMessage 配置时，
/// iMessage 集成的状态应从 Available 转换为 Active。
///
/// # 测试配置
///
/// 使用包含以下字段的 `IMessageConfig`：
/// - `allowed_contacts`: ["*"]（允许所有联系人）
///
/// # 预期结果
///
/// - 状态应为 `IntegrationStatus::Active`
#[test]
fn imessage_active_when_configured() {
    let mut config = Config::default();
    config.channels_config.imessage = Some(IMessageConfig { allowed_contacts: vec!["*".into()] });
    let entries = all_integrations();
    let im = entries.iter().find(|e| e.name == "iMessage").unwrap();
    assert!(matches!((im.status_fn)(&config), IntegrationStatus::Active));
}

/// 验证 iMessage 在未配置时状态为 Available
///
/// 此测试验证当配置中没有 iMessage 配置时，
/// iMessage 集成应保持 Available 状态。
///
/// # 预期结果
///
/// - 状态应为 `IntegrationStatus::Available`
#[test]
fn imessage_available_when_not_configured() {
    let config = Config::default();
    let entries = all_integrations();
    let im = entries.iter().find(|e| e.name == "iMessage").unwrap();
    assert!(matches!((im.status_fn)(&config), IntegrationStatus::Available));
}

/// 验证 Matrix 在配置后状态变为 Active
///
/// 此测试验证当配置中包含有效的 Matrix 配置时，
/// Matrix 集成的状态应从 Available 转换为 Active。
///
/// # 测试配置
///
/// 使用包含以下字段的 `MatrixConfig`：
/// - `homeserver`: "https://m.org"
/// - `access_token`: "tok"
/// - `room_id`: "!r:m"
///
/// # 预期结果
///
/// - 状态应为 `IntegrationStatus::Active`
#[test]
fn matrix_active_when_configured() {
    let mut config = Config::default();
    config.channels_config.matrix = Some(MatrixConfig {
        homeserver: "https://m.org".into(),
        access_token: "tok".into(),
        user_id: None,
        device_id: None,
        room_id: "!r:m".into(),
        allowed_users: vec![],
        mention_only: false,
    });
    let entries = all_integrations();
    let mx = entries.iter().find(|e| e.name == "Matrix").unwrap();
    assert!(matches!((mx.status_fn)(&config), IntegrationStatus::Active));
}

/// 验证 Matrix 在未配置时状态为 Available
///
/// 此测试验证当配置中没有 Matrix 配置时，
/// Matrix 集成应保持 Available 状态。
///
/// # 预期结果
///
/// - 状态应为 `IntegrationStatus::Available`
#[test]
fn matrix_available_when_not_configured() {
    let config = Config::default();
    let entries = all_integrations();
    let mx = entries.iter().find(|e| e.name == "Matrix").unwrap();
    assert!(matches!((mx.status_fn)(&config), IntegrationStatus::Available));
}

/// 验证"即将推出"集成保持 ComingSoon 状态
///
/// 此测试确保标记为"即将推出"的集成无论配置如何
/// 都始终返回 `ComingSoon` 状态。
///
/// # 测试的集成
///
/// - Nostr
/// - Spotify
/// - Home Assistant
///
/// # 预期结果
///
/// - 所有上述集成的状态应为 `IntegrationStatus::ComingSoon`
#[test]
fn coming_soon_integrations_stay_coming_soon() {
    let config = Config::default();
    let entries = all_integrations();
    for name in ["Nostr", "Spotify", "Home Assistant"] {
        let entry = entries.iter().find(|e| e.name == name).unwrap();
        assert!(
            matches!((entry.status_fn)(&config), IntegrationStatus::ComingSoon),
            "{name} should be ComingSoon"
        );
    }
}

/// 验证 LM Studio 在未设为默认 Provider 时状态为 Available
///
/// 此测试验证当 LM Studio 不是配置中的默认 Provider 时，
/// 其状态应为 Available（可用但未激活）。
///
/// # 预期结果
///
/// - 状态应为 `IntegrationStatus::Available`
#[test]
fn lm_studio_available_when_not_selected_as_default_provider() {
    let config = Config::default();
    let entries = all_integrations();
    let lm_studio = entries.iter().find(|e| e.name == "LM Studio").unwrap();
    assert!(matches!((lm_studio.status_fn)(&config), IntegrationStatus::Available));
}

/// 验证 LM Studio 对别名 `lmstudio` 和 `lm-studio` 的正确激活
///
/// 此测试验证当 `default_provider` 设置为 LM Studio 的任一别名时，
/// LM Studio 集成都应正确识别并变为 Active 状态。
///
/// # 测试的别名
///
/// 1. `"lmstudio"` - 无连字符别名
/// 2. `"lm-studio"` - 带连字符别名
///
/// # 预期结果
///
/// - 使用任一别名时状态都应为 `IntegrationStatus::Active`
#[test]
fn lm_studio_active_for_lmstudio_default_provider_aliases() {
    let entries = all_integrations();
    let lm_studio = entries.iter().find(|e| e.name == "LM Studio").unwrap();

    let mut config = Config { default_provider: Some("lmstudio".to_string()), ..Config::default() };
    assert!(matches!((lm_studio.status_fn)(&config), IntegrationStatus::Active));

    config.default_provider = Some("lm-studio".to_string());
    assert!(matches!((lm_studio.status_fn)(&config), IntegrationStatus::Active));
}

/// 验证 WhatsApp 在未配置时状态为 Available
///
/// 此测试确保 WhatsApp 集成在默认配置下
/// 返回 Available 状态，表示功能可用但未配置激活。
///
/// # 预期结果
///
/// - 状态应为 `IntegrationStatus::Available`
#[test]
fn whatsapp_available_when_not_configured() {
    let config = Config::default();
    let entries = all_integrations();
    let wa = entries.iter().find(|e| e.name == "WhatsApp").unwrap();
    assert!(matches!((wa.status_fn)(&config), IntegrationStatus::Available));
}

/// 验证 Email 在未配置时状态为 Available
///
/// 此测试确保 Email 集成在默认配置下
/// 返回 Available 状态。
///
/// # 预期结果
///
/// - 状态应为 `IntegrationStatus::Available`
#[test]
fn email_available_when_not_configured() {
    let config = Config::default();
    let entries = all_integrations();
    let email = entries.iter().find(|e| e.name == "Email").unwrap();
    assert!(matches!((email.status_fn)(&config), IntegrationStatus::Available));
}

/// 验证 Shell 和 File System 集成始终为 Active
///
/// 此测试确保 Shell 和 File System 这两个核心集成
/// 无论配置如何都始终处于 Active 状态，因为它们是
/// 代理运行时的基础能力。
///
/// # 测试的集成
///
/// - Shell - 命令行执行能力
/// - File System - 文件系统访问能力
///
/// # 预期结果
///
/// - 两个集成的状态都应为 `IntegrationStatus::Active`
#[test]
fn shell_and_filesystem_always_active() {
    let config = Config::default();
    let entries = all_integrations();
    for name in ["Shell", "File System"] {
        let entry = entries.iter().find(|e| e.name == name).unwrap();
        assert!(
            matches!((entry.status_fn)(&config), IntegrationStatus::Active),
            "{name} should always be Active"
        );
    }
}

/// 验证 macOS 集成的平台条件激活
///
/// 此测试验证 macOS 特定集成的状态取决于当前运行平台：
///
/// # 平台行为
///
/// - 在 macOS 上：状态应为 `Active`（原生功能可用）
/// - 在其他平台：状态应为 `Available`（功能存在但非原生）
///
/// # 实现细节
///
/// 使用 `cfg!(target_os = "macos")` 在编译时检测目标平台。
#[test]
fn macos_active_on_macos() {
    let config = Config::default();
    let entries = all_integrations();
    let macos = entries.iter().find(|e| e.name == "macOS").unwrap();
    let status = (macos.status_fn)(&config);
    if cfg!(target_os = "macos") {
        assert!(matches!(status, IntegrationStatus::Active));
    } else {
        assert!(matches!(status, IntegrationStatus::Available));
    }
}

/// 验证关键分类的集成数量满足最低要求
///
/// 此测试确保 Chat（聊天）和 AI Model（AI 模型）这两个
/// 核心分类的集成数量达到预期，保证生态系统的完整性。
///
/// # 最低数量要求
///
/// - Chat 分类：至少 5 个集成
/// - AI Model 分类：至少 5 个集成
///
/// # 目的
///
/// 作为回归测试，防止意外删除重要集成导致分类数量不足。
#[test]
fn category_counts_reasonable() {
    let entries = all_integrations();
    let chat_count = entries.iter().filter(|e| e.category == IntegrationCategory::Chat).count();
    let ai_count = entries.iter().filter(|e| e.category == IntegrationCategory::AiModel).count();
    assert!(chat_count >= 5, "Expected 5+ chat integrations, got {chat_count}");
    assert!(ai_count >= 5, "Expected 5+ AI model integrations, got {ai_count}");
}

/// 验证区域化 Provider 别名正确激活对应的 AI 集成
///
/// 此测试确保各 AI Provider 的区域化别名（如 `-cn`、`-intl` 后缀）
/// 能够正确识别并激活对应的集成。
///
/// # 测试的 Provider 别名映射
///
/// | 别名 | 对应集成 | 说明 |
/// |------|----------|------|
/// | `minimax-cn` | MiniMax | MiniMax 中国区 |
/// | `glm-cn` | GLM | GLM 中国区 |
/// | `moonshot-intl` | Moonshot | Moonshot 国际版 |
/// | `qwen-intl` | Qwen | Qwen 国际版 |
/// | `zai-cn` | Z.AI | Z.AI 中国区 |
/// | `baidu` | Qianfan | 百度千帆 |
///
/// # 预期结果
///
/// 每个别名设置后，对应的 AI 集成状态应为 `IntegrationStatus::Active`
#[test]
fn regional_provider_aliases_activate_expected_ai_integrations() {
    let entries = all_integrations();

    // 测试 MiniMax 中国区别名
    let mut config =
        Config { default_provider: Some("minimax-cn".to_string()), ..Config::default() };

    let minimax = entries.iter().find(|e| e.name == "MiniMax").unwrap();
    assert!(matches!((minimax.status_fn)(&config), IntegrationStatus::Active));

    // 测试 GLM 中国区别名
    config.default_provider = Some("glm-cn".to_string());
    let glm = entries.iter().find(|e| e.name == "GLM").unwrap();
    assert!(matches!((glm.status_fn)(&config), IntegrationStatus::Active));

    // 测试 Moonshot 国际版别名
    config.default_provider = Some("moonshot-intl".to_string());
    let moonshot = entries.iter().find(|e| e.name == "Moonshot").unwrap();
    assert!(matches!((moonshot.status_fn)(&config), IntegrationStatus::Active));

    // 测试 Qwen 国际版别名
    config.default_provider = Some("qwen-intl".to_string());
    let qwen = entries.iter().find(|e| e.name == "Qwen").unwrap();
    assert!(matches!((qwen.status_fn)(&config), IntegrationStatus::Active));

    // 测试 Z.AI 中国区别名
    config.default_provider = Some("zai-cn".to_string());
    let zai = entries.iter().find(|e| e.name == "Z.AI").unwrap();
    assert!(matches!((zai.status_fn)(&config), IntegrationStatus::Active));

    // 测试百度千帆别名
    config.default_provider = Some("baidu".to_string());
    let qianfan = entries.iter().find(|e| e.name == "Qianfan").unwrap();
    assert!(matches!((qianfan.status_fn)(&config), IntegrationStatus::Active));
}

fn entry<'a>(entries: &'a [IntegrationEntry], name: &str) -> &'a IntegrationEntry {
    entries.iter().find(|entry| entry.name == name).unwrap_or_else(|| panic!("{name} missing"))
}

#[test]
fn default_provider_matcher_is_case_insensitive_and_handles_missing_values() {
    let empty = Config::default();
    assert!(!default_provider_matches(&empty, &["zai"]));

    let config = Config { default_provider: Some("ZAI-CN".to_string()), ..Config::default() };
    assert!(default_provider_matches(&config, &["zai", "zai-cn"]));
    assert!(!default_provider_matches(&config, &["glm", "glm-cn"]));
}

#[test]
fn configured_chat_channels_become_active() {
    let entries = all_integrations();
    let mut config = Config::default();

    config.channels_config.discord = Some(DiscordConfig {
        bot_token: "discord-token".into(),
        guild_id: None,
        allowed_users: vec![],
        listen_to_bots: false,
        mention_only: false,
        group_reply: None,
    });
    config.channels_config.slack = Some(SlackConfig {
        bot_token: "slack-token".into(),
        app_token: None,
        channel_id: None,
        allowed_users: vec![],
        group_reply: None,
    });
    config.channels_config.webhook = Some(WebhookConfig { port: 8787, secret: None });
    config.channels_config.whatsapp = Some(WhatsAppConfig {
        access_token: Some("whatsapp-token".into()),
        phone_number_id: Some("phone-id".into()),
        verify_token: Some("verify".into()),
        app_secret: None,
        session_path: None,
        pair_phone: None,
        pair_code: None,
        allowed_numbers: vec![],
    });
    config.channels_config.signal = Some(SignalConfig {
        http_url: "http://127.0.0.1:8080".into(),
        account: "+15555550100".into(),
        group_id: None,
        allowed_from: vec![],
        ignore_attachments: false,
        ignore_stories: false,
    });
    config.channels_config.dingtalk = Some(DingTalkConfig {
        client_id: "ding-id".into(),
        client_secret: "ding-secret".into(),
        allowed_users: vec![],
    });
    config.channels_config.qq = Some(QQConfig {
        app_id: "qq-id".into(),
        app_secret: "qq-secret".into(),
        allowed_users: vec![],
        receive_mode: QQReceiveMode::Webhook,
    });

    for name in ["Discord", "Slack", "Webhooks", "WhatsApp", "Signal", "DingTalk", "QQ Official"] {
        assert_eq!((entry(&entries, name).status_fn)(&config), IntegrationStatus::Active, "{name}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn email_becomes_active_when_configured() {
    let entries = all_integrations();
    let mut config = Config::default();
    config.channels_config.email =
        Some(EmailConfig { imap_host: "imap.example.test".into(), ..EmailConfig::default() });

    assert_eq!((entry(&entries, "Email").status_fn)(&config), IntegrationStatus::Active);
}

#[test]
fn ai_provider_entries_reflect_provider_and_model_configuration() {
    let entries = all_integrations();
    let mut config = Config::default();

    config.default_provider = Some("openrouter".into());
    config.api_key = Some("sk-test".into());
    assert_eq!((entry(&entries, "OpenRouter").status_fn)(&config), IntegrationStatus::Active);

    for (provider, name) in [
        ("anthropic", "Anthropic"),
        ("openai", "OpenAI"),
        ("ollama", "Ollama"),
        ("perplexity", "Perplexity"),
        ("venice", "Venice"),
        ("vercel", "Vercel AI"),
        ("cloudflare", "Cloudflare AI"),
        ("synthetic", "Synthetic"),
        ("opencode", "OpenCode Zen"),
        ("dashscope", "Qwen"),
        ("qianfan", "Qianfan"),
    ] {
        config = Config { default_provider: Some(provider.to_string()), ..Config::default() };
        assert_eq!((entry(&entries, name).status_fn)(&config), IntegrationStatus::Active, "{name}");
    }

    for (model, name) in [
        ("google/gemini-pro", "Google"),
        ("deepseek/deepseek-chat", "DeepSeek"),
        ("x-ai/grok", "xAI"),
        ("mistral-large", "Mistral"),
    ] {
        config = Config { default_model: Some(model.to_string()), ..Config::default() };
        assert_eq!((entry(&entries, name).status_fn)(&config), IntegrationStatus::Active, "{name}");
    }
}

#[test]
fn openrouter_requires_api_key_even_when_selected() {
    let entries = all_integrations();
    let mut config =
        Config { default_provider: Some("openrouter".to_string()), ..Config::default() };

    assert_eq!((entry(&entries, "OpenRouter").status_fn)(&config), IntegrationStatus::Available);

    config.api_key = Some("sk-test".to_string());
    assert_eq!((entry(&entries, "OpenRouter").status_fn)(&config), IntegrationStatus::Active);
}

#[test]
fn available_and_always_active_ai_entries_are_stable() {
    let entries = all_integrations();
    let config = Config::default();

    for name in ["Amazon Bedrock", "Together AI", "Fireworks AI", "Cohere"] {
        assert_eq!(
            (entry(&entries, name).status_fn)(&config),
            IntegrationStatus::Available,
            "{name}"
        );
    }
    assert_eq!((entry(&entries, "Groq").status_fn)(&config), IntegrationStatus::Active);
}

#[test]
fn platform_entries_reflect_current_target() {
    let entries = all_integrations();
    let config = Config::default();

    let linux_status = (entry(&entries, "Linux").status_fn)(&config);
    if cfg!(target_os = "linux") {
        assert_eq!(linux_status, IntegrationStatus::Active);
    } else {
        assert_eq!(linux_status, IntegrationStatus::Available);
    }

    assert_eq!((entry(&entries, "Windows").status_fn)(&config), IntegrationStatus::Available);
    assert_eq!((entry(&entries, "iOS").status_fn)(&config), IntegrationStatus::Available);
    assert_eq!((entry(&entries, "Android").status_fn)(&config), IntegrationStatus::Available);
}
