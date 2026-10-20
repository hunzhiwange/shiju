//! # Provider 模块 - 统一的大模型提供商入口
//!
//! 本模块提供统一的 Provider 抽象层，由 `session::llm` 模块作为底层实现支撑。
//!
//! ## 模块职责
//!
//! - 定义 Provider 的核心 trait 和数据类型
//! - 提供工厂函数用于创建各种 Provider 实例
//! - 实现敏感信息（如 API 密钥、令牌）的脱敏处理
//! - 提供 API 错误的标准化处理和清理
//!
//! ## 主要组件
//!
//! - [`traits`] - 定义 `Provider` trait 及相关的数据结构
//! - [`session_llm`] - 基于会话的 LLM Provider 实现
//! - [`CompatibleApiMode`] - 兼容 API 模式的枚举类型
//! - [`ProviderRuntimeOptions`] - Provider 运行时配置选项
//!
//! ## 工厂函数
//!
//! 本模块提供多个工厂函数用于创建 Provider 实例：
//!
//! - [`create_provider`] - 基础 Provider 创建
//! - [`create_provider_with_options`] - 带运行时选项的 Provider 创建
//! - [`create_provider_with_url`] - 带自定义 URL 的 Provider 创建
//! - [`create_resilient_provider`] - 创建具有重试能力的 Provider
//! - [`create_routed_provider`] - 创建支持模型路由的 Provider
//!
//! ## 安全特性
//!
//! - [`scrub_secret_patterns`] - 从文本中脱敏敏感令牌模式
//! - [`sanitize_api_error`] - 清理并截断 API 错误信息

pub mod session_llm;
pub mod traits;

#[allow(unused_imports)]
pub use traits::{
    ChatMessage, ChatRequest, ChatResponse, ConversationMessage, Provider, ProviderCapabilityError,
    ToolCall, ToolResultMessage,
};

use std::path::PathBuf;

/// API 错误消息的最大字符数限制
///
/// 超过此长度的错误消息将被截断，以避免日志过大或内存占用过高。
const MAX_API_ERROR_CHARS: usize = 4096;

pub use vw_config_types::provider::CompatibleApiMode;

/// Provider 运行时配置选项
///
/// 包含 Provider 初始化和运行时行为的可配置参数。
/// 这些选项可以覆盖默认的配置值，用于定制 Provider 的行为。
#[derive(Debug, Clone)]
pub struct ProviderRuntimeOptions {
    /// 当前配置中声明的自定义 Provider ID。
    ///
    /// Provider 的 API、adapter 与模型信息仍由 provider resolver 解析；这里仅用于让同步
    /// 工厂区分已配置的动态 Provider 与拼写错误的未知名称。
    pub configured_provider_ids: Vec<String>,

    /// 认证配置文件的覆盖名称
    ///
    /// 当设置时，使用指定的配置文件而非默认配置进行认证。
    pub auth_profile_override: Option<String>,

    /// Provider API 的基础 URL
    ///
    /// 用于覆盖默认的 API 端点地址，支持自托管或代理场景。
    pub provider_api_url: Option<String>,

    /// VibeWindow 工作目录路径
    ///
    /// 用于存储 Provider 相关的配置和缓存文件。
    pub vibewindow_dir: Option<PathBuf>,

    /// 是否启用密钥加密
    ///
    /// 控制敏感数据（如 API 密钥）在存储时是否进行加密。
    /// 默认为 `true` 以确保安全。
    pub secrets_encrypt: bool,

    /// 是否启用推理（reasoning）功能
    ///
    /// 某些模型支持链式推理能力，此选项控制是否启用。
    pub reasoning_enabled: Option<bool>,

    /// 推理级别
    ///
    /// 控制模型推理的深度或详细程度，具体含义取决于模型实现。
    pub reasoning_level: Option<String>,

    /// 自定义 Provider 的 API 兼容模式
    ///
    /// 指定第三方 Provider 应使用的 API 兼容模式。
    pub custom_provider_api_mode: Option<CompatibleApiMode>,

    /// 最大令牌数的覆盖值
    ///
    /// 用于覆盖模型默认的 `max_tokens` 参数。
    pub max_tokens_override: Option<u32>,

    /// 模型是否支持视觉功能
    ///
    /// 显式指定模型是否具备图像理解能力。
    pub model_support_vision: Option<bool>,
}

impl Default for ProviderRuntimeOptions {
    /// 返回默认的运行时配置选项
    ///
    /// 默认值：
    /// - `auth_profile_override`: `None`（使用默认配置）
    /// - `provider_api_url`: `None`（使用默认 URL）
    /// - `vibewindow_dir`: `None`
    /// - `secrets_encrypt`: `true`（默认启用加密）
    /// - `reasoning_enabled`: `None`
    /// - `reasoning_level`: `None`
    /// - `custom_provider_api_mode`: `None`
    /// - `max_tokens_override`: `None`
    /// - `model_support_vision`: `None`
    fn default() -> Self {
        Self {
            configured_provider_ids: Vec::new(),
            auth_profile_override: None,
            provider_api_url: None,
            vibewindow_dir: None,
            secrets_encrypt: true,
            reasoning_enabled: None,
            reasoning_level: None,
            custom_provider_api_mode: None,
            max_tokens_override: None,
            model_support_vision: None,
        }
    }
}

/// 判断字符是否为有效的密钥令牌字符
///
/// 密钥令牌通常由字母数字字符和部分特殊字符组成。
///
/// # 参数
///
/// - `c`: 待检测的字符
///
/// # 返回值
///
/// 如果字符是有效的密钥令牌字符（字母、数字、连字符、下划线、点、冒号），
/// 返回 `true`；否则返回 `false`。
fn is_secret_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':')
}

/// 查找密钥令牌的结束位置
///
/// 从指定位置开始，向后扫描直到遇到非密钥字符，返回令牌结束位置。
///
/// # 参数
///
/// - `input`: 输入字符串
/// - `from`: 开始扫描的位置（应为令牌内容的起始位置）
///
/// # 返回值
///
/// 令牌内容结束的位置（不包含该位置的字符）。
fn token_end(input: &str, from: usize) -> usize {
    let mut end = from;
    // 遍历从 `from` 开始的每个字符
    for (i, c) in input[from..].char_indices() {
        if is_secret_char(c) {
            // 更新结束位置，包含当前字符
            end = from + i + c.len_utf8();
        } else {
            // 遇到非密钥字符，停止扫描
            break;
        }
    }
    end
}

/// 从文本中脱敏已知的敏感令牌模式
///
/// 扫描输入字符串，识别并替换各种常见的敏感信息模式，
/// 如 API 密钥、访问令牌、OAuth 令牌等，将其替换为 `[REDACTED]`。
///
/// # 支持的敏感模式
///
/// 包括但不限于：
/// - OpenAI 密钥前缀 (`sk-`)
/// - Slack 令牌前缀 (`xoxb-`, `xoxp-`)
/// - GitHub 个人访问令牌 (`ghp_`, `gho_`, `ghu_`, `github_pat_`)
/// - Google API 密钥 (`AIza`)
/// - AWS 访问密钥 (`AKIA`)
/// - JSON 格式的各种令牌字段
/// - URL 查询参数格式的令牌
/// - Bearer 认证头
///
/// # 参数
///
/// - `input`: 需要脱敏的原始字符串
///
/// # 返回值
///
/// 返回已脱敏的字符串，所有匹配的敏感令牌已被替换为 `[REDACTED]`。
///
/// # 示例
///
/// ```ignore
/// let input = "Error: Invalid API key sk-1234567890abcdef";
/// let scrubbed = scrub_secret_patterns(input);
/// assert_eq!(scrubbed, "Error: Invalid API key [REDACTED]");
/// ```
pub fn scrub_secret_patterns(input: &str) -> String {
    // 定义敏感前缀及其最小有效长度
    // 元组格式：(前缀字符串, 令牌内容的最小字符数)
    const PREFIXES: [(&str, usize); 26] = [
        ("sk-", 1),                  // OpenAI 密钥
        ("xoxb-", 1),                // Slack Bot 令牌
        ("xoxp-", 1),                // Slack 用户令牌
        ("ghp_", 1),                 // GitHub 个人访问令牌
        ("gho_", 1),                 // GitHub OAuth 令牌
        ("ghu_", 1),                 // GitHub 用户到服务器令牌
        ("github_pat_", 1),          // GitHub 细粒度个人访问令牌
        ("AIza", 1),                 // Google API 密钥
        ("AKIA", 1),                 // AWS 访问密钥 ID
        ("\"access_token\":\"", 8),  // JSON access_token 字段
        ("\"refresh_token\":\"", 8), // JSON refresh_token 字段
        ("\"id_token\":\"", 8),      // JSON id_token 字段
        ("\"token\":\"", 8),         // JSON token 字段
        ("\"api_key\":\"", 8),       // JSON api_key 字段
        ("\"client_secret\":\"", 8), // JSON client_secret 字段
        ("\"app_secret\":\"", 8),    // JSON app_secret 字段
        ("\"verify_token\":\"", 8),  // JSON verify_token 字段
        ("access_token=", 8),        // URL 参数 access_token
        ("refresh_token=", 8),       // URL 参数 refresh_token
        ("id_token=", 8),            // URL 参数 id_token
        ("token=", 8),               // URL 参数 token
        ("api_key=", 8),             // URL 参数 api_key
        ("client_secret=", 8),       // URL 参数 client_secret
        ("app_secret=", 8),          // URL 参数 app_secret
        ("Bearer ", 16),             // Bearer 认证头
        ("bearer ", 16),             // bearer 认证头（小写）
    ];

    let mut scrubbed = input.to_string();

    // 遍历每个敏感前缀模式
    for (prefix, min_len) in PREFIXES {
        let mut search_from = 0;

        // 循环查找并替换所有匹配项
        loop {
            // 在剩余字符串中查找前缀
            let Some(rel) = scrubbed[search_from..].find(prefix) else {
                break;
            };

            // 计算前缀在原字符串中的绝对位置
            let start = search_from + rel;
            // 令牌内容的起始位置（前缀之后）
            let content_start = start + prefix.len();
            // 查找令牌内容的结束位置
            let end = token_end(&scrubbed, content_start);
            // 计算令牌长度
            let token_len = end.saturating_sub(content_start);

            // 如果令牌长度不足，可能是误匹配，跳过
            if token_len < min_len {
                search_from = content_start;
                continue;
            }

            // 将敏感令牌替换为 [REDACTED]
            scrubbed.replace_range(start..end, "[REDACTED]");
            // 更新搜索起点，避免重复处理已替换的区域
            search_from = start + "[REDACTED]".len();
        }
    }

    scrubbed
}

/// 清理并截断 API 错误信息
///
/// 对 API 错误文本进行两步处理：
/// 1. 脱敏所有敏感信息模式
/// 2. 如果超过最大长度限制，进行安全截断
///
/// # 参数
///
/// - `input`: 原始 API 错误文本
///
/// # 返回值
///
/// 返回已脱敏且（必要时）截断的错误字符串。截断时会添加 `...` 后缀。
///
/// # 示例
///
/// ```ignore
/// let error = "Error with token sk-verylongtoken123...";
/// let sanitized = sanitize_api_error(error);
/// // sanitized: "Error with token [REDACTED]..."
/// ```
pub fn sanitize_api_error(input: &str) -> String {
    // 首先脱敏敏感信息
    let scrubbed = scrub_secret_patterns(input);

    // 检查是否超过字符限制
    if scrubbed.chars().count() <= MAX_API_ERROR_CHARS {
        return scrubbed;
    }

    // 安全截断到最大长度
    let mut end = MAX_API_ERROR_CHARS;
    // 确保截断位置在 UTF-8 字符边界上
    while end > 0 && !scrubbed.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &scrubbed[..end])
}

/// 从失败的 HTTP 响应构建标准化的 Provider 错误
///
/// 异步读取响应体，清理敏感信息，并格式化为标准错误消息。
///
/// # 参数
///
/// - `provider`: Provider 名称，用于错误消息标识
/// - `response`: 失败的 HTTP 响应对象
///
/// # 返回值
///
/// 返回格式化的 `anyhow::Error`，包含 Provider 名称、HTTP 状态码和清理后的错误体。
///
/// # 错误格式
///
/// `{provider} API error ({status}): {sanitized_body}`
pub async fn api_error(provider: &str, response: reqwest::Response) -> anyhow::Error {
    let status = response.status();
    // 尝试读取响应体文本，失败时使用占位符
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<failed to read provider error body>".to_string());
    // 清理错误体中的敏感信息
    let sanitized = sanitize_api_error(&body);
    anyhow::anyhow!("{provider} API error ({status}): {sanitized}")
}

/// 判断给定的名称是否为 Moonshot 模型的别名
///
/// Moonshot AI 的模型有多个常用别名（如 Kimi），此函数用于统一识别。
///
/// # 参数
///
/// - `name`: Provider 或模型名称字符串
///
/// # 返回值
///
/// 如果名称是 Moonshot 的已知别名，返回 `true`；否则返回 `false`。
///
/// # 支持的别名
///
/// - `moonshot`
/// - `kimi`
/// - `kimi-k2`
/// - `kimi-k2-5`
pub fn is_moonshot_alias(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "moonshot" | "moonshot-intl" | "kimi" | "kimi-k2" | "kimi-k2-5"
    )
}

/// 工厂函数：根据配置创建 Provider 实例
///
/// 使用默认的运行时选项创建指定名称的 Provider。
///
/// # 参数
///
/// - `name`: Provider 名称（如 "openai"、"anthropic" 等）
/// - `api_key`: 可选的 API 密钥
///
/// # 返回值
///
/// 返回成功创建的 Provider 实例，封装在 `Box<dyn Provider>` 中。
///
/// # 错误
///
/// 如果 Provider 名称无效或初始化失败，返回 `anyhow::Error`。
pub fn create_provider(name: &str, api_key: Option<&str>) -> anyhow::Result<Box<dyn Provider>> {
    create_provider_with_options(name, api_key, &ProviderRuntimeOptions::default())
}

/// 工厂函数：使用运行时选项创建 Provider 实例
///
/// 创建 Provider 并应用指定的运行时配置选项。
///
/// # 参数
///
/// - `name`: Provider 名称
/// - `api_key`: 可选的 API 密钥
/// - `options`: 运行时配置选项
///
/// # 返回值
///
/// 返回成功创建的 Provider 实例。
///
/// # 实现说明
///
/// 当前实现使用 `session_llm::SessionLlmProvider` 作为统一的底层实现。
pub fn create_provider_with_options(
    name: &str,
    api_key: Option<&str>,
    options: &ProviderRuntimeOptions,
) -> anyhow::Result<Box<dyn Provider>> {
    validate_provider_name(name, &options.configured_provider_ids)?;
    let _ = api_key;
    Ok(Box::new(session_llm::SessionLlmProvider::with_options(options)))
}

fn validate_provider_name(name: &str, configured_provider_ids: &[String]) -> anyhow::Result<()> {
    let normalized = name.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        anyhow::bail!("Provider name must not be empty");
    }

    if let Some(url) = normalized.strip_prefix("custom:") {
        return validate_custom_provider_url("custom", url);
    }
    if let Some((prefix, url)) = normalized.split_once("-custom:") {
        if prefix.is_empty() {
            anyhow::bail!("Custom provider prefix must not be empty");
        }
        return validate_custom_provider_url(prefix, url);
    }

    let supported = matches!(
        normalized.as_str(),
        "shiju"
            | "openrouter"
            | "openai"
            | "openai-codex"
            | "anthropic"
            | "claude"
            | "google"
            | "gemini"
            | "deepseek"
            | "groq"
            | "xai"
            | "mistral"
            | "cohere"
            | "perplexity"
            | "ollama"
            | "lm-studio"
            | "moonshot"
            | "moonshot-intl"
            | "kimi"
            | "kimi-k2"
            | "kimi-k2-5"
            | "qwen"
            | "qwen-intl"
            | "dashscope"
            | "alibaba-cn"
            | "glm"
            | "glm-cn"
            | "zai"
            | "zai-cn"
            | "baidu"
            | "minimax"
            | "copilot"
    );
    if supported {
        return Ok(());
    }

    if configured_provider_ids
        .iter()
        .any(|provider_id| provider_id.trim().eq_ignore_ascii_case(&normalized))
    {
        return Ok(());
    }

    anyhow::bail!("Unknown provider: {name}");
}

fn validate_custom_provider_url(label: &str, url: &str) -> anyhow::Result<()> {
    let url = url.trim();
    if url.is_empty() {
        anyhow::bail!("{label} provider requires a URL");
    }
    match reqwest::Url::parse(url) {
        Ok(parsed) if matches!(parsed.scheme(), "http" | "https") => Ok(()),
        Ok(parsed) => {
            anyhow::bail!("custom provider URL must use http/https, got '{}'", parsed.scheme())
        }
        Err(error) => anyhow::bail!("invalid custom provider URL: {error}"),
    }
}

/// 工厂函数：使用自定义 URL 创建 Provider 实例
///
/// 创建 Provider 并指定自定义的 API 基础 URL。
///
/// # 参数
///
/// - `name`: Provider 名称
/// - `api_key`: 可选的 API 密钥
/// - `api_url`: 可选的自定义 API 基础 URL
///
/// # 返回值
///
/// 返回成功创建的 Provider 实例。
pub fn create_provider_with_url(
    name: &str,
    api_key: Option<&str>,
    api_url: Option<&str>,
) -> anyhow::Result<Box<dyn Provider>> {
    create_provider_with_url_and_options(name, api_key, api_url, &ProviderRuntimeOptions::default())
}

/// 工厂函数：使用自定义 URL 和运行时选项创建 Provider 实例
///
/// 创建 Provider 并同时指定自定义 URL 和运行时配置选项。
///
/// # 参数
///
/// - `name`: Provider 名称
/// - `api_key`: 可选的 API 密钥
/// - `api_url`: 可选的自定义 API 基础 URL
/// - `options`: 运行时配置选项
///
/// # 返回值
///
/// 返回成功创建的 Provider 实例。
///
/// # 实现说明
///
/// 当前实现中 `api_url` 参数暂未使用，委托给 `create_provider_with_options`。
pub fn create_provider_with_url_and_options(
    name: &str,
    api_key: Option<&str>,
    api_url: Option<&str>,
    options: &ProviderRuntimeOptions,
) -> anyhow::Result<Box<dyn Provider>> {
    let _ = api_url;
    create_provider_with_options(name, api_key, options)
}

/// 工厂函数：创建具有重试能力的 Provider 实例
///
/// 创建一个支持自动重试和故障恢复的 Provider。
/// 当前实现已迁移至 `session::llm` 模块处理可靠性逻辑。
///
/// # 参数
///
/// - `primary_name`: 主 Provider 名称
/// - `api_key`: 可选的 API 密钥
/// - `api_url`: 可选的自定义 API 基础 URL
/// - `reliability`: 可靠性配置（重试策略、超时等）
///
/// # 返回值
///
/// 返回成功创建的 Provider 实例。
pub fn create_resilient_provider(
    primary_name: &str,
    api_key: Option<&str>,
    api_url: Option<&str>,
    reliability: &crate::app::agent::config::ReliabilityConfig,
) -> anyhow::Result<Box<dyn Provider>> {
    create_resilient_provider_with_options(
        primary_name,
        api_key,
        api_url,
        reliability,
        &ProviderRuntimeOptions::default(),
    )
}

/// 工厂函数：创建具有重试能力的 Provider 实例（带运行时选项）
///
/// 创建支持自动重试的 Provider，并应用指定的运行时配置。
///
/// # 参数
///
/// - `primary_name`: 主 Provider 名称
/// - `api_key`: 可选的 API 密钥
/// - `api_url`: 可选的自定义 API 基础 URL
/// - `_reliability`: 可靠性配置（当前未使用，由 session::llm 处理）
/// - `options`: 运行时配置选项
///
/// # 返回值
///
/// 返回成功创建的 Provider 实例。
///
/// # 实现说明
///
/// 可靠性链式处理已迁移至 `session::llm` 模块内部实现。
pub fn create_resilient_provider_with_options(
    primary_name: &str,
    api_key: Option<&str>,
    api_url: Option<&str>,
    _reliability: &crate::app::agent::config::ReliabilityConfig,
    options: &ProviderRuntimeOptions,
) -> anyhow::Result<Box<dyn Provider>> {
    // 可靠性链式处理现在由 session::llm 内部处理
    create_provider_with_url_and_options(primary_name, api_key, api_url, options)
}

/// 工厂函数：创建支持模型路由的 Provider 实例
///
/// 创建一个能够根据配置将不同请求路由到不同模型的 Provider。
///
/// # 参数
///
/// - `primary_name`: 主 Provider 名称
/// - `api_key`: 可选的 API 密钥
/// - `api_url`: 可选的自定义 API 基础 URL
/// - `reliability`: 可靠性配置
/// - `model_routes`: 模型路由配置列表
/// - `default_model`: 默认模型名称
///
/// # 返回值
///
/// 返回成功创建的 Provider 实例。
pub fn create_routed_provider(
    primary_name: &str,
    api_key: Option<&str>,
    api_url: Option<&str>,
    reliability: &crate::app::agent::config::ReliabilityConfig,
    model_routes: &[crate::app::agent::config::ModelRouteConfig],
    default_model: &str,
) -> anyhow::Result<Box<dyn Provider>> {
    create_routed_provider_with_options(
        primary_name,
        api_key,
        api_url,
        reliability,
        model_routes,
        default_model,
        &ProviderRuntimeOptions::default(),
    )
}

/// 工厂函数：创建支持模型路由的 Provider 实例（带运行时选项）
///
/// 创建支持模型路由的 Provider，并应用指定的运行时配置。
///
/// # 参数
///
/// - `primary_name`: 主 Provider 名称
/// - `api_key`: 可选的 API 密钥
/// - `api_url`: 可选的自定义 API 基础 URL
/// - `_reliability`: 可靠性配置（当前未使用）
/// - `_model_routes`: 模型路由配置列表（当前未使用）
/// - `_default_model`: 默认模型名称（当前未使用）
/// - `options`: 运行时配置选项
///
/// # 返回值
///
/// 返回成功创建的 Provider 实例。
///
/// # 实现说明
///
/// 模型路由提示现在由 `session::llm` 的模型选择逻辑内部处理。
pub fn create_routed_provider_with_options(
    primary_name: &str,
    api_key: Option<&str>,
    api_url: Option<&str>,
    reliability: &crate::app::agent::config::ReliabilityConfig,
    _model_routes: &[crate::app::agent::config::ModelRouteConfig],
    _default_model: &str,
    options: &ProviderRuntimeOptions,
) -> anyhow::Result<Box<dyn Provider>> {
    // 模型路由提示现在由 session::llm 的模型选择逻辑解析
    create_resilient_provider_with_options(primary_name, api_key, api_url, reliability, options)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
