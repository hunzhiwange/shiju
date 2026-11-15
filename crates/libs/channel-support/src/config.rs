//! 不依赖通道执行器的超时及模型默认值解析。
use agent_config::schema::Config;

const MIN_CHANNEL_MESSAGE_TIMEOUT_SECS: u64 = 30;
const CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP: u64 = 4;

pub fn effective_channel_message_timeout_secs(configured: u64) -> u64 {
    configured.max(MIN_CHANNEL_MESSAGE_TIMEOUT_SECS)
}

pub fn channel_message_timeout_budget_secs(
    message_timeout_secs: u64,
    max_iterations: usize,
) -> u64 {
    // 确保迭代次数至少为 1，避免除零错误
    let iterations = max_iterations.max(1) as u64;
    // 限制缩放因子的最大值，防止预算过大
    let scale = iterations.min(CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP);
    // 饱和乘法，防止整数溢出
    message_timeout_secs.saturating_mul(scale)
}

pub fn resolved_default_provider(config: &Config) -> String {
    main_agent_model_binding(config)
        .map(|(provider, _)| provider.to_string())
        .or_else(|| config.default_provider.clone())
        .unwrap_or_else(|| "zhipuai-coding-plan".to_string())
}

pub fn resolved_default_model(config: &Config) -> String {
    main_agent_model_binding(config)
        .map(|(provider, model)| format!("{provider}/{model}"))
        .or_else(|| config.default_model.clone())
        .unwrap_or_else(|| "zhipuai-coding-plan/glm-5".to_string())
}

fn main_agent_model_binding(config: &Config) -> Option<(&str, &str)> {
    let main = config.agents.get("main")?;
    let provider = main.provider.trim();
    let model = main.model.trim();
    if provider.is_empty() || model.is_empty() { None } else { Some((provider, model)) }
}

use anyhow::{Context, Result};
use agent_config::schema::{NonCliNaturalLanguageApprovalMode};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

/// 通道运行时默认配置。
///
/// 包含通道启动时所需的默认值，如默认提供者、模型、
/// 温度参数以及可靠性配置。
#[derive(Debug, Clone)]
pub struct ChannelRuntimeDefaults {
    /// 默认的服务提供者名称
    pub default_provider: String,
    /// 默认使用的模型标识符
    pub model: String,
    /// 生成温度参数（0.0-2.0）
    pub temperature: f64,
    /// 可选的 API 密钥
    pub api_key: Option<String>,
    /// 可选的 API 端点 URL
    pub api_url: Option<String>,
    /// 可靠性配置（重试、超时等）
    pub reliability: agent_config::schema::ReliabilityConfig,
}

/// 配置文件时间戳。
///
/// 用于检测配置文件是否已更改，
/// 通过修改时间和文件长度共同判断。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigFileStamp {
    /// 文件最后修改时间
    pub modified: SystemTime,
    /// 文件字节长度
    pub len: u64,
}

/// 运行时配置状态。
///
/// 维护当前生效的配置及上次应用配置的时间戳，
/// 用于热重载时比较配置是否需要更新。
#[derive(Debug, Clone)]
pub struct RuntimeConfigState {
    /// 当前生效的运行时默认配置
    pub defaults: ChannelRuntimeDefaults,
    /// 上次应用配置时的文件时间戳
    pub last_applied_stamp: Option<ConfigFileStamp>,
}

/// 运行时自治策略。
///
/// 定义非 CLI 通道（如 Telegram、Discord）的工具审批行为，
/// 包括自动批准的工具列表、始终询问的工具列表，
/// 以及自然语言审批模式配置。
#[derive(Debug, Clone)]
pub struct RuntimeAutonomyPolicy {
    /// 自动批准的工具名称列表
    pub auto_approve: Vec<String>,
    /// 始终需要用户确认的工具名称列表
    pub always_ask: Vec<String>,
    /// 非 CLI 环境下排除的工具列表
    pub non_cli_excluded_tools: Vec<String>,
    /// 非 CLI 环境下有权审批的用户列表
    pub non_cli_approval_approvers: Vec<String>,
    /// 全局自然语言审批模式
    pub non_cli_natural_language_approval_mode: NonCliNaturalLanguageApprovalMode,
    /// 按通道细分的自然语言审批模式
    pub non_cli_natural_language_approval_mode_by_channel:
        HashMap<String, NonCliNaturalLanguageApprovalMode>,
}


/// 获取运行时配置存储的全局单例
///
/// 提供一个全局的、线程安全的配置状态存储，用于缓存已加载的配置。
/// 使用 `OnceLock` 确保存储只初始化一次。
///
/// # 返回值
///
/// 返回配置存储的静态引用，类型为 `Mutex<HashMap<PathBuf, RuntimeConfigState>>`
///
/// # 线程安全
///
/// - 使用 `OnceLock` 保证单例初始化的线程安全
/// - 使用 `Mutex` 保护内部 HashMap 的并发访问
///
/// # 示例
///
/// ```ignore
/// let store = runtime_config_store();
/// let mut cache = store.lock().unwrap();
/// cache.insert(path, state);
/// ```
pub fn runtime_config_store() -> &'static Mutex<HashMap<PathBuf, RuntimeConfigState>> {
    static STORE: OnceLock<Mutex<HashMap<PathBuf, RuntimeConfigState>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 从配置对象构建通道运行时默认值
///
/// 将配置文件中的参数提取并构建为运行时默认值结构体。
///
/// # 参数
///
/// - `config`: 配置对象的引用
///
/// # 返回值
///
/// 返回构建的 `ChannelRuntimeDefaults` 实例
///
/// # 提取的字段
///
/// - `default_provider`: 默认 Provider
/// - `model`: 默认模型
/// - `temperature`: 温度参数
/// - `api_key`: API 密钥（可选）
/// - `api_url`: API 地址（可选）
/// - `reliability`: 可靠性配置
pub fn runtime_defaults_from_config(config: &Config) -> ChannelRuntimeDefaults {
    ChannelRuntimeDefaults {
        default_provider: resolved_default_provider(config),
        model: resolved_default_model(config),
        temperature: config.default_temperature,
        api_key: config.api_key.clone(),
        api_url: config.api_url.clone(),
        reliability: config.reliability.clone(),
    }
}

/// 从配置对象构建运行时自主策略
///
/// 提取配置中的自主性策略参数，用于控制代理的自动批准和交互行为。
///
/// # 参数
///
/// - `config`: 配置对象的引用
///
/// # 返回值
///
/// 返回构建的 `RuntimeAutonomyPolicy` 实例
///
/// # 策略字段
///
/// - `auto_approve`: 自动批准的工具列表
/// - `always_ask`: 总是询问的工具列表
/// - `non_cli_excluded_tools`: 非 CLI 环境下排除的工具
/// - `non_cli_approval_approvers`: 非 CLI 环境下的审批者配置
/// - `non_cli_natural_language_approval_mode`: 自然语言审批模式
/// - `non_cli_natural_language_approval_mode_by_channel`: 按通道的自然语言审批模式
pub fn runtime_autonomy_policy_from_config(config: &Config) -> RuntimeAutonomyPolicy {
    RuntimeAutonomyPolicy {
        auto_approve: config.autonomy.auto_approve.clone(),
        always_ask: config.autonomy.always_ask.clone(),
        non_cli_excluded_tools: config.autonomy.non_cli_excluded_tools.clone(),
        non_cli_approval_approvers: config.autonomy.non_cli_approval_approvers.clone(),
        non_cli_natural_language_approval_mode: config
            .autonomy
            .non_cli_natural_language_approval_mode,
        non_cli_natural_language_approval_mode_by_channel: config
            .autonomy
            .non_cli_natural_language_approval_mode_by_channel
            .clone(),
    }
}

/// 获取配置文件的时间戳信息
///
/// 读取文件的元数据以获取修改时间和大小，用于检测配置文件变化。
///
/// # 参数
///
/// - `path`: 配置文件路径的引用
///
/// # 返回值
///
/// - 成功：返回 `Some(ConfigFileStamp)`，包含修改时间和文件大小
/// - 失败或 WASM 环境：返回 `None`
///
/// # 平台差异
///
/// - 在非 WASM 环境：使用 `tokio::fs` 异步读取文件元数据
/// - 在 WASM 环境：直接返回 `None`（不支持文件系统访问）
///
/// # 错误处理
///
/// 文件访问或元数据读取失败时，静默返回 `None`
pub async fn config_file_stamp(path: &Path) -> Option<ConfigFileStamp> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // 异步读取文件元数据
        let metadata = tokio::fs::metadata(path).await.ok()?;
        // 获取文件修改时间
        let modified = metadata.modified().ok()?;
        Some(ConfigFileStamp { modified, len: metadata.len() })
    }
    #[cfg(target_arch = "wasm32")]
    None
}

/// 为运行时重载解密可选的敏感字段
///
/// 检查并解密配置字段中的加密值，用于配置重新加载时的敏感信息处理。
///
/// # 参数
///
/// - `store`: 密钥存储的引用
/// - `value`: 需要解密的字段值（可变引用）
/// - `field_name`: 字段名称（用于错误消息）
///
/// # 返回值
///
/// - 成功：返回 `Ok(())`
/// - 解密失败：返回带有上下文信息的错误
///
/// # 工作流程
///
/// 1. 克隆字段值进行检查
/// 2. 判断值是否被加密（通过 `is_encrypted` 方法）
/// 3. 如果加密，调用存储的 `decrypt` 方法解密
/// 4. 更新原始字段值为解密后的明文
pub fn decrypt_optional_secret_for_runtime_reload(
    store: &secret_store::secrets::SecretStore,
    value: &mut Option<String>,
    field_name: &str,
) -> Result<()> {
    // 检查字段是否有值
    if let Some(raw) = value.clone() {
        // 检查值是否被加密
        if secret_store::secrets::SecretStore::is_encrypted(&raw) {
            // 解密并更新字段值
            *value = Some(
                store.decrypt(&raw).with_context(|| format!("Failed to decrypt {field_name}"))?,
            );
        }
    }
    Ok(())
}

/// 从配置文件加载运行时默认值和自主策略
///
/// 读取并解析配置文件，解密敏感信息，应用环境变量覆盖，
/// 返回构建好的默认值和策略对象。
///
/// # 参数
///
/// - `path`: 配置文件路径的引用
///
/// # 返回值
///
/// - 成功：返回 `Ok((ChannelRuntimeDefaults, RuntimeAutonomyPolicy))`
/// - 失败：返回错误（文件读取失败、解析失败、解密失败等）
///
/// # 平台支持
///
/// - **非 WASM**：正常执行配置加载
/// - **WASM**：直接返回错误（不支持配置加载）
///
/// # 工作流程
///
/// 1. 异步读取文件内容
/// 2. 解析 TOML 格式的配置
/// 3. 设置配置路径
/// 4. 初始化密钥存储并解密 API Key
/// 5. 应用环境变量覆盖
/// 6. 构建并返回默认值和策略
///
/// # 安全考虑
///
/// - API Key 支持加密存储
/// - 使用专用的 SecretStore 处理解密
/// - 解密失败会返回明确的错误信息
pub async fn load_runtime_defaults_from_config_file(
    path: &Path,
) -> Result<(
    ChannelRuntimeDefaults,
    RuntimeAutonomyPolicy,
    agent_config::schema::ChannelsConfig,
)> {
    // WASM 环境不支持配置加载
    #[cfg(target_arch = "wasm32")]
    anyhow::bail!("Config loading not supported in WASM");

    #[cfg(not(target_arch = "wasm32"))]
    {
        // 异步读取配置文件内容
        let contents = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read {}", path.display()))?;

        // 解析 TOML 格式的配置
        let mut parsed: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        // 设置配置文件路径（用于后续的相对路径解析）
        parsed.config_path = path.to_path_buf();

        // 尝试解密配置中的敏感字段
        if let Some(vibewindow_dir) = path.parent() {
            // 初始化密钥存储
            let store = secret_store::secrets::SecretStore::new(
                vibewindow_dir,
                parsed.secrets.encrypt,
            );
            // 解密 API Key
            decrypt_optional_secret_for_runtime_reload(
                &store,
                &mut parsed.api_key,
                "config.api_key",
            )?;
        }

        // 应用环境变量覆盖（优先级最高）
        agent_config::runtime_schema::apply_env_overrides(&mut parsed);

        // 构建并返回默认值和策略
        Ok((
            runtime_defaults_from_config(&parsed),
            runtime_autonomy_policy_from_config(&parsed),
            parsed.channels_config,
        ))
    }
}

#[cfg(test)]
#[path = "timeout_budget_tests.rs"]
mod timeout_budget_tests;
