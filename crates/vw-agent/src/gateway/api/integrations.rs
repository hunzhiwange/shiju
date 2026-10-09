//! 仪表盘 AI 集成规范与设置管理模块
//!
//! 本模块负责管理 VibeWindow 仪表盘中可用的 AI 提供商集成规范。
//! 它提供了以下核心功能：
//!
//! - 定义支持的 AI 提供商集成规范（如 OpenRouter、Anthropic、OpenAI 等）
//! - 查询和匹配集成规范
//! - 构建集成设置的载荷数据结构
//! - 应用集成凭据更新
//! - 计算配置修订版本哈希
//!
//! ## 主要组件
//!
//! - [`DashboardAiIntegrationSpec`]: 单个 AI 集成规范的静态定义
//! - [`DASHBOARD_AI_INTEGRATION_SPECS`]: 所有支持的 AI 集成规范列表
//! - [`build_integration_settings_payload`]: 构建集成设置载荷供前端使用
//! - [`apply_integration_credentials_update`]: 应用集成凭据更新到配置
//!
//! ## 设计原则
//!
//! 本模块采用静态规范定义与动态配置查询相结合的方式：
//! - 规范定义使用 `&'static str` 避免运行时分配
//! - 配置查询实时从 `Config` 读取当前状态
//! - 提供商别名匹配支持多种常见命名方式

use super::types::{
    IntegrationCredentialsField, IntegrationSettingsEntry, IntegrationSettingsPayload,
};
use crate::app::agent::config::Config;
use crate::app::agent::config::schema::validate_config;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// 仪表盘 AI 集成规范结构体
///
/// 定义单个 AI 提供商在仪表盘中的展示和配置规范。
/// 该结构体使用静态字符串引用以优化内存使用和性能。
///
/// # 字段说明
///
/// - `id`: 集成的唯一标识符，用于 API 请求和内部查找
/// - `integration_name`: 集成在用户界面中的显示名称
/// - `provider_id`: 对应的 provider 实现标识符，用于路由请求
/// - `requires_api_key`: 该集成是否需要 API 密钥才能使用
/// - `supports_api_url`: 该集成是否支持自定义 API 端点 URL
/// - `model_options`: 该集成支持的模型选项列表
///
/// # 示例
///
/// ```
/// let spec = DashboardAiIntegrationSpec {
///     id: "openai",
///     integration_name: "OpenAI",
///     provider_id: "openai",
///     requires_api_key: true,
///     supports_api_url: false,
///     model_options: &["gpt-5.2", "gpt-4o"],
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct DashboardAiIntegrationSpec {
    /// 集成的唯一标识符（如 "openai"、"anthropic"）
    pub id: &'static str,
    /// 集成在用户界面中的显示名称（如 "OpenAI"、"Anthropic"）
    pub integration_name: &'static str,
    /// 对应的 provider 实现标识符，用于请求路由
    pub provider_id: &'static str,
    /// 该集成是否需要 API 密钥
    pub requires_api_key: bool,
    /// 该集成是否支持自定义 API 端点 URL
    pub supports_api_url: bool,
    /// 该集成支持的模型选项列表
    pub model_options: &'static [&'static str],
}

/// 所有支持的 AI 集成规范静态列表
///
/// 该常量定义了 VibeWindow 仪表盘中所有可用的 AI 提供商集成。
/// 每个规范包含提供商的基本信息和配置选项。
///
/// # 支持的集成
///
/// 1. **OpenRouter** - 多模型聚合平台
/// 2. **Anthropic** - Claude 系列模型
/// 3. **OpenAI** - GPT 系列模型
/// 4. **Google** - Gemini 系列模型
/// 5. **DeepSeek** - DeepSeek 系列模型
/// 6. **xAI** - Grok 系列模型
/// 7. **Mistral** - Mistral 系列模型
/// 8. **Ollama** - 本地模型运行平台
/// 9. **Perplexity** - Perplexity AI 搜索模型
/// 10. **Venice** - Venice AI 模型
/// 11. **Vercel AI** - Vercel AI SDK 模型
/// 12. **Cloudflare AI** - Cloudflare Workers AI 模型
///
/// # 使用场景
///
/// 该列表用于：
/// - 前端集成设置页面的渲染
/// - 集成 ID 到规范的查找
/// - 提供商激活状态的检测
pub const DASHBOARD_AI_INTEGRATION_SPECS: &[DashboardAiIntegrationSpec] = &[
    // OpenRouter - 多模型聚合平台，支持多种顶级 AI 模型
    DashboardAiIntegrationSpec {
        id: "openrouter",
        integration_name: "OpenRouter",
        provider_id: "openrouter",
        requires_api_key: true,
        supports_api_url: false,
        model_options: &["anthropic/claude-sonnet-4-6", "openai/gpt-5.2", "google/gemini-3.1-pro"],
    },
    // Anthropic - Claude 系列模型，擅长长文本理解和分析
    DashboardAiIntegrationSpec {
        id: "anthropic",
        integration_name: "Anthropic",
        provider_id: "anthropic",
        requires_api_key: true,
        supports_api_url: false,
        model_options: &["claude-sonnet-4-6", "claude-opus-4-6"],
    },
    // OpenAI - GPT 系列模型，业界标准的通用 AI 模型
    DashboardAiIntegrationSpec {
        id: "openai",
        integration_name: "OpenAI",
        provider_id: "openai",
        requires_api_key: true,
        supports_api_url: false,
        model_options: &["gpt-5.2", "gpt-5.2-codex", "gpt-4o"],
    },
    // Google - Gemini 系列模型，Google 最新的多模态 AI
    DashboardAiIntegrationSpec {
        id: "google",
        integration_name: "Google",
        provider_id: "gemini",
        requires_api_key: true,
        supports_api_url: false,
        model_options: &["google/gemini-3.1-pro", "google/gemini-3-flash"],
    },
    // DeepSeek - DeepSeek 系列模型，注重推理能力
    DashboardAiIntegrationSpec {
        id: "deepseek",
        integration_name: "DeepSeek",
        provider_id: "deepseek",
        requires_api_key: true,
        supports_api_url: false,
        model_options: &["deepseek/deepseek-reasoner", "deepseek/deepseek-chat"],
    },
    // xAI - Grok 系列模型，由 xAI 开发
    DashboardAiIntegrationSpec {
        id: "xai",
        integration_name: "xAI",
        provider_id: "xai",
        requires_api_key: true,
        supports_api_url: false,
        model_options: &["x-ai/grok-4", "x-ai/grok-3"],
    },
    // Mistral - Mistral 系列模型，欧洲开源 AI 公司
    DashboardAiIntegrationSpec {
        id: "mistral",
        integration_name: "Mistral",
        provider_id: "mistral",
        requires_api_key: true,
        supports_api_url: false,
        model_options: &["mistral-large-latest", "codestral-latest"],
    },
    // Ollama - 本地模型运行平台，无需 API 密钥，支持自定义 URL
    DashboardAiIntegrationSpec {
        id: "ollama",
        integration_name: "Ollama",
        provider_id: "ollama",
        requires_api_key: false,
        supports_api_url: true,
        model_options: &["llama3.2", "qwen2.5-coder:7b", "phi4"],
    },
    // Perplexity - Perplexity AI 搜索增强模型
    DashboardAiIntegrationSpec {
        id: "perplexity",
        integration_name: "Perplexity",
        provider_id: "perplexity",
        requires_api_key: true,
        supports_api_url: false,
        model_options: &["sonar-pro", "sonar-reasoning-pro", "sonar"],
    },
    // Venice - Venice AI 模型
    DashboardAiIntegrationSpec {
        id: "venice",
        integration_name: "Venice",
        provider_id: "venice",
        requires_api_key: true,
        supports_api_url: false,
        model_options: &["zai-org-glm-5", "venice-uncensored"],
    },
    // Vercel AI - Vercel AI SDK，支持多种模型
    DashboardAiIntegrationSpec {
        id: "vercel",
        integration_name: "Vercel AI",
        provider_id: "vercel",
        requires_api_key: true,
        supports_api_url: false,
        model_options: &["openai/gpt-5.2", "anthropic/claude-sonnet-4-6", "google/gemini-3.1-pro"],
    },
    // Cloudflare AI - Cloudflare Workers AI，边缘 AI 推理
    DashboardAiIntegrationSpec {
        id: "cloudflare",
        integration_name: "Cloudflare AI",
        provider_id: "cloudflare",
        requires_api_key: true,
        supports_api_url: false,
        model_options: &["@cf/meta/llama-3.3-70b-instruct-fp8-fast", "@cf/qwen/qwen3-32b"],
    },
];

/// 根据 ID 查找仪表盘集成规范
///
/// 在 `DASHBOARD_AI_INTEGRATION_SPECS` 中查找匹配指定 ID 的集成规范。
/// ID 匹配不区分大小写。
///
/// # 参数
///
/// - `id`: 要查找的集成 ID（如 "openai"、"anthropic"）
///
/// # 返回值
///
/// 返回 `Some(&DashboardAiIntegrationSpec)` 如果找到匹配的规范，
/// 否则返回 `None`。
///
/// # 示例
///
/// ```
/// let spec = find_dashboard_spec("openai");
/// assert!(spec.is_some());
/// assert_eq!(spec.unwrap().integration_name, "OpenAI");
///
/// let not_found = find_dashboard_spec("unknown");
/// assert!(not_found.is_none());
/// ```
pub fn find_dashboard_spec(id: &str) -> Option<&'static DashboardAiIntegrationSpec> {
    DASHBOARD_AI_INTEGRATION_SPECS.iter().find(|spec| spec.id.eq_ignore_ascii_case(id))
}

/// 检查提供商名称是否匹配集成的别名
///
/// 某些集成支持多个常见的别名。例如：
/// - "google" 集成支持 "google"、"google-gemini"、"gemini"
/// - "xai" 集成支持 "xai"、"grok"
///
/// # 参数
///
/// - `spec`: 要检查的集成规范
/// - `provider`: 用户配置中的提供商名称
///
/// # 返回值
///
/// 如果提供商名称匹配集成的任何别名，返回 `true`，否则返回 `false`。
///
/// # 内部逻辑
///
/// 1. 规范化提供商名称：去除首尾空白并转为小写
/// 2. 对特殊集成（google、xai、vercel、cloudflare）使用别名匹配
/// 3. 对其他集成直接比较规范化的名称与 provider_id
fn provider_alias_matches(spec: &DashboardAiIntegrationSpec, provider: &str) -> bool {
    // 规范化提供商名称：去除空白并转为小写
    let normalized = provider.trim().to_ascii_lowercase();

    // 根据集成 ID 使用不同的别名匹配策略
    match spec.id {
        // Google 集成支持多个常见别名
        "google" => matches!(normalized.as_str(), "google" | "google-gemini" | "gemini"),
        // xAI 集成支持 "xai" 和 "grok" 两种命名
        "xai" => matches!(normalized.as_str(), "xai" | "grok"),
        // Vercel AI 集成支持 "vercel" 和 "vercel-ai"
        "vercel" => matches!(normalized.as_str(), "vercel" | "vercel-ai"),
        // Cloudflare AI 集成支持 "cloudflare" 和 "cloudflare-ai"
        "cloudflare" => matches!(normalized.as_str(), "cloudflare" | "cloudflare-ai"),
        // 其他集成直接比较规范化名称与 provider_id
        _ => normalized == spec.provider_id,
    }
}

/// 检查集成规范是否为当前活跃的提供商
///
/// 检查配置中的 `default_provider` 是否匹配指定的集成规范。
///
/// # 参数
///
/// - `config`: 当前配置对象
/// - `spec`: 要检查的集成规范
///
/// # 返回值
///
/// 如果配置中的 `default_provider` 匹配该集成规范，返回 `true`，否则返回 `false`。
///
/// # 内部逻辑
///
/// 1. 从配置中获取 `default_provider`
/// 2. 使用 `provider_alias_matches` 检查是否匹配
fn is_spec_active(config: &Config, spec: &DashboardAiIntegrationSpec) -> bool {
    config
        .default_provider
        .as_deref()
        .is_some_and(|provider| provider_alias_matches(spec, provider))
}

/// 检查字符串值是否非空
///
/// 检查 `Option<&str>` 是否包含非空字符串（去除空白后）。
///
/// # 参数
///
/// - `value`: 可选的字符串引用
///
/// # 返回值
///
/// 如果值存在且去除空白后不为空，返回 `true`，否则返回 `false`。
///
/// # 示例
///
/// ```
/// assert!(has_non_empty(Some("value")));
/// assert!(!has_non_empty(Some("  ")));
/// assert!(!has_non_empty(None));
/// ```
fn has_non_empty(value: Option<&str>) -> bool {
    value.is_some_and(|candidate| !candidate.trim().is_empty())
}

/// 计算配置的修订版本哈希
///
/// 使用 SHA256 算法计算配置的修订版本标识符。
/// 该哈希用于前端检测配置是否发生变化。
///
/// # 参数
///
/// - `config`: 要计算哈希的配置对象
///
/// # 返回值
///
/// 返回配置的 SHA256 哈希值的十六进制字符串表示。
///
/// # 内部逻辑
///
/// 1. 将配置序列化为 TOML 格式
/// 2. 计算 TOML 字符串的 SHA256 哈希
/// 3. 将哈希值格式化为十六进制字符串
///
/// # 示例
///
/// ```
/// let revision = config_revision(&config);
/// println!("Current config revision: {}", revision);
/// ```
pub fn config_revision(config: &Config) -> String {
    // 将配置序列化为 TOML 字符串
    let serialized = toml::to_string(config).unwrap_or_default();
    // 计算 SHA256 哈希
    let digest = Sha256::digest(serialized.as_bytes());
    // 格式化为十六进制字符串
    hex::encode(digest)
}

/// 获取当前活跃的仪表盘提供商 ID
///
/// 遍历所有集成规范，查找当前配置中活跃的提供商，
/// 并返回对应的集成 ID。
///
/// # 参数
///
/// - `config`: 当前配置对象
///
/// # 返回值
///
/// 如果找到活跃的提供商，返回 `Some(String)` 包含集成 ID，
/// 否则返回 `None`。
///
/// # 内部逻辑
///
/// 1. 遍历 `DASHBOARD_AI_INTEGRATION_SPECS`
/// 2. 对每个规范调用 `is_spec_active` 检查是否活跃
/// 3. 返回第一个匹配的集成 ID
fn active_dashboard_provider_id(config: &Config) -> Option<String> {
    DASHBOARD_AI_INTEGRATION_SPECS.iter().find_map(|spec| {
        if is_spec_active(config, spec) { Some(spec.id.to_string()) } else { None }
    })
}

/// 构建集成设置载荷
///
/// 为前端仪表盘构建完整的集成设置数据结构。
/// 该载荷包含所有支持的集成、它们的当前状态和配置字段。
///
/// # 参数
///
/// - `config`: 当前配置对象
///
/// # 返回值
///
/// 返回 `IntegrationSettingsPayload` 包含：
/// - `revision`: 配置的修订版本哈希
/// - `active_default_provider_integration_id`: 当前活跃的提供商集成 ID
/// - `integrations`: 所有集成的设置条目列表
///
/// # 内部逻辑
///
/// 1. 获取所有集成注册信息
/// 2. 遍历每个集成规范：
///    a. 查找对应的注册条目
///    b. 获取集成状态
///    c. 检查是否为活跃提供商
///    d. 构建凭据字段（API Key、默认模型、API URL）
///    e. 确定配置完成状态
/// 3. 构建并返回载荷
///
/// # 示例
///
/// ```
/// let payload = build_integration_settings_payload(&config);
/// // payload.revision - 配置版本
/// // payload.active_default_provider_integration_id - 活跃提供商
/// // payload.integrations - 所有集成列表
/// ```
pub fn build_integration_settings_payload(config: &Config) -> IntegrationSettingsPayload {
    // 获取所有集成注册信息
    let all_integrations = crate::app::agent::integrations::registry::all_integrations();
    let mut entries = Vec::new();

    // 遍历每个仪表盘集成规范
    for spec in DASHBOARD_AI_INTEGRATION_SPECS {
        // 查找对应的注册条目；静态仪表盘规范必须与集成注册表保持同步。
        let registry_entry = all_integrations
            .iter()
            .find(|entry| entry.name == spec.integration_name)
            .expect("dashboard AI integration spec missing from registry");

        // 获取集成状态
        let status = (registry_entry.status_fn)(config);

        // 检查是否为当前活跃的提供商
        let is_active_provider = is_spec_active(config, spec);

        // 检查是否已配置 API Key
        let has_key = has_non_empty(config.api_key.as_deref());

        // 检查是否已配置默认模型（仅在活跃提供商时有效）
        let has_model = is_active_provider && has_non_empty(config.default_model.as_deref());

        // 检查是否已配置 API URL（仅在活跃提供商时有效）
        let has_api_url = is_active_provider && has_non_empty(config.api_url.as_deref());

        // 构建凭据字段列表
        let mut fields = vec![
            // API Key 字段
            IntegrationCredentialsField {
                key: "api_key".to_string(),
                label: "API Key".to_string(),
                required: spec.requires_api_key,
                has_value: has_key,
                input_type: "secret",
                options: Vec::new(),
                current_value: None,
                // 如果有值则显示掩码
                masked_value: has_key.then(|| "••••••••".to_string()),
            },
            // 默认模型字段
            IntegrationCredentialsField {
                key: "default_model".to_string(),
                label: "Default Model".to_string(),
                required: false,
                has_value: has_model,
                input_type: "select",
                // 提供模型选项列表
                options: spec.model_options.iter().map(|value| (*value).to_string()).collect(),
                // 仅在活跃提供商时显示当前值
                current_value: if is_active_provider {
                    config
                        .default_model
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                        .map(std::string::ToString::to_string)
                } else {
                    None
                },
                masked_value: None,
            },
        ];

        // 如果集成支持自定义 API URL，添加该字段
        if spec.supports_api_url {
            fields.push(IntegrationCredentialsField {
                key: "api_url".to_string(),
                label: "Base URL".to_string(),
                required: false,
                has_value: has_api_url,
                input_type: "text",
                options: Vec::new(),
                // 仅在活跃提供商时显示当前值
                current_value: if is_active_provider {
                    config
                        .api_url
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                        .map(std::string::ToString::to_string)
                } else {
                    None
                },
                masked_value: None,
            });
        }

        // 确定配置完成状态
        // 如果需要 API Key，则必须在活跃提供商且有 Key 的情况下才算配置完成
        // 否则，只要提供商活跃即算配置完成
        let configured =
            if spec.requires_api_key { is_active_provider && has_key } else { is_active_provider };

        // 构建集成设置条目
        entries.push(IntegrationSettingsEntry {
            id: spec.id.to_string(),
            name: registry_entry.name.to_string(),
            description: registry_entry.description.to_string(),
            category: registry_entry.category,
            status,
            configured,
            activates_default_provider: true,
            fields,
        });
    }

    // 构建并返回完整载荷
    IntegrationSettingsPayload {
        revision: config_revision(config),
        active_default_provider_integration_id: active_dashboard_provider_id(config),
        integrations: entries,
    }
}

/// 应用集成凭据更新
///
/// 根据前端提交的字段值更新配置对象。
/// 该函数验证字段有效性、应用更新并确保配置有效。
///
/// # 参数
///
/// - `config`: 当前配置对象（不会被修改）
/// - `integration_id`: 要更新的集成 ID
/// - `fields`: 字段名到值的映射
///
/// # 返回值
///
/// 成功时返回 `Ok(Config)` 包含更新后的配置对象，
/// 失败时返回 `Err(String)` 包含错误信息。
///
/// # 支持的字段
///
/// - `api_key`: API 密钥
/// - `default_model`: 默认模型
/// - `api_url`: API 端点 URL（仅对支持自定义 URL 的集成有效）
///
/// # 错误
///
/// 该函数在以下情况返回错误：
/// - 集成 ID 未知
/// - 字段不被该集成支持
/// - 更新后的配置验证失败
///
/// # 内部逻辑
///
/// 1. 查找集成规范
/// 2. 克隆当前配置
/// 3. 应用字段更新：
///    - `api_key`: 更新或清空 API 密钥
///    - `default_model`: 更新或清空默认模型
///    - `api_url`: 验证支持性并更新或清空
/// 4. 设置 `default_provider` 为该集成的 provider_id
/// 5. 如果是首次激活，设置默认模型为第一个选项
/// 6. 清理不支持的 `api_url` 设置
/// 7. 验证更新后的配置
///
/// # 示例
///
/// ```
/// let mut fields = BTreeMap::new();
/// fields.insert("api_key".to_string(), "sk-xxx".to_string());
/// fields.insert("default_model".to_string(), "gpt-5.2".to_string());
///
/// match apply_integration_credentials_update(&config, "openai", &fields) {
///     Ok(updated_config) => println!("Config updated successfully"),
///     Err(e) => eprintln!("Failed to update config: {}", e),
/// }
/// ```
pub fn apply_integration_credentials_update(
    config: &Config,
    integration_id: &str,
    fields: &BTreeMap<String, String>,
) -> Result<Config, String> {
    // 查找集成规范，如果找不到则返回错误
    let Some(spec) = find_dashboard_spec(integration_id) else {
        return Err(format!("Unknown integration id: {integration_id}"));
    };

    // 记录是否之前就是活跃提供商
    let was_active_provider = is_spec_active(config, spec);

    // 克隆配置以进行修改
    let mut updated = config.clone();

    // 遍历所有提交的字段并应用更新
    for (key, value) in fields {
        let trimmed = value.trim();
        match key.as_str() {
            // 更新 API Key
            "api_key" => {
                updated.api_key = if trimmed.is_empty() { None } else { Some(trimmed.to_string()) };
            }
            // 更新默认模型
            "default_model" => {
                updated.default_model =
                    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) };
            }
            // 更新 API URL（需要验证支持性）
            "api_url" => {
                // 检查集成是否支持自定义 API URL
                if !spec.supports_api_url {
                    return Err(format!(
                        "Integration '{}' does not support api_url",
                        spec.integration_name
                    ));
                }
                updated.api_url = if trimmed.is_empty() { None } else { Some(trimmed.to_string()) };
            }
            // 不支持的字段
            _ => {
                return Err(format!(
                    "Unsupported field '{key}' for integration '{integration_id}'"
                ));
            }
        }
    }

    // 设置默认提供商为该集成的 provider_id
    updated.default_provider = Some(spec.provider_id.to_string());

    // 如果是首次激活且未指定默认模型，使用第一个选项
    if !fields.contains_key("default_model") && !was_active_provider {
        updated.default_model = spec.model_options.first().map(|value| (*value).to_string());
    }

    // 清理 API URL 设置
    // 如果集成不支持自定义 URL，且之前不是活跃提供商，清空 api_url
    // 如果支持但未在本次更新中指定且之前不是活跃提供商，也清空
    if !spec.supports_api_url && !was_active_provider {
        updated.api_url = None;
    } else if spec.supports_api_url && !fields.contains_key("api_url") && !was_active_provider {
        updated.api_url = None;
    }

    // 验证更新后的配置
    validate_config(&updated).map_err(|err| format!("Invalid integration config update: {err}"))?;

    Ok(updated)
}

#[cfg(test)]
#[path = "integrations_tests.rs"]
mod integrations_tests;
