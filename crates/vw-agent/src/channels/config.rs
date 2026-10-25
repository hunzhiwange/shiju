//! # 通道运行时配置模块
//!
//! 本模块提供通道（Channel）运行时配置的管理和动态更新功能。
//!
//! ## 主要功能
//!
//! - **消息超时配置**：计算和管理通道消息的有效超时时间和预算
//! - **运行时配置存储**：提供全局的运行时配置状态存储
//! - **默认配置解析**：解析默认的 Provider、Model 和其他运行时参数
//! - **项目覆盖加载**：从工作区加载项目覆盖并解密敏感信息
//! - **动态配置更新**：检测项目覆盖变化并热更新运行时配置
//!
//! ## 配置层次
//!
//! 配置按以下优先级加载：
//! 1. 环境变量覆盖
//! 2. 项目覆盖（`.vibewindow.json`）
//! 3. 默认值
//!
//! ## 安全考虑
//!
//! - API Key 等敏感信息支持加密存储
//! - 配置更新时自动解密敏感字段
//! - 配置文件变更检测基于文件元数据（修改时间和大小）

use super::*;

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;

/// 计算有效的通道消息超时时间（秒）
///
/// 确保消息超时时间不低于最小阈值，防止因超时时间过短导致的消息处理失败。
///
/// # 参数
///
/// - `configured`: 用户配置的消息超时时间（秒）
///
/// # 返回值
///
/// 返回有效的超时时间，取配置值和最小阈值的较大值
///
/// # 示例
///
/// ```ignore
/// // 配置值大于最小阈值，直接使用配置值
/// assert_eq!(effective_channel_message_timeout_secs(60), 60);
///
/// // 配置值小于最小阈值，使用最小阈值
/// assert_eq!(effective_channel_message_timeout_secs(5), MIN_CHANNEL_MESSAGE_TIMEOUT_SECS);
/// ```
pub(crate) fn effective_channel_message_timeout_secs(configured: u64) -> u64 {
    configured.max(MIN_CHANNEL_MESSAGE_TIMEOUT_SECS)
}

/// 计算通道消息的超时预算（秒）
///
/// 根据消息超时时间和最大工具迭代次数，计算总体的超时预算。
/// 该预算考虑了工具执行可能需要的多次迭代，确保有足够时间完成复杂任务。
///
/// # 参数
///
/// - `message_timeout_secs`: 单次消息的基本超时时间（秒）
/// - `max_iterations`: 最大工具迭代次数
///
/// # 返回值
///
/// 返回计算得到的超时预算（秒）
///
/// # 计算逻辑
///
/// 1. 确保迭代次数至少为 1
/// 2. 对迭代次数应用上限（防止预算过大）
/// 3. 将基本超时时间与缩放因子相乘
/// 4. 使用饱和乘法防止溢出
///
/// # 示例
///
/// ```ignore
/// // 基本超时 30 秒，最大迭代 3 次
/// let budget = channel_message_timeout_budget_secs(30, 3);
/// // budget = 30 * min(3, SCALE_CAP)
/// ```
pub(crate) fn channel_message_timeout_budget_secs(
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
pub(crate) fn runtime_config_store() -> &'static Mutex<HashMap<PathBuf, RuntimeConfigState>> {
    static STORE: OnceLock<Mutex<HashMap<PathBuf, RuntimeConfigState>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 解析配置中的默认 Provider
///
/// 优先使用主代理的完整模型绑定；未配置时回退到顶层默认 Provider，
/// 最后才使用系统默认值。
///
/// # 参数
///
/// - `config`: 配置对象的引用
///
/// # 返回值
///
/// 返回默认 Provider 的名称字符串
///
/// # 默认值
///
/// 如果配置中未指定，默认使用 `"zhipuai-coding-plan"`
pub(crate) fn resolved_default_provider(config: &Config) -> String {
    main_agent_model_binding(config)
        .map(|(provider, _)| provider.to_string())
        .or_else(|| config.default_provider.clone())
        .unwrap_or_else(|| "zhipuai-coding-plan".to_string())
}

/// 解析配置中的默认 Model
///
/// 优先使用主代理的完整模型绑定；未配置时回退到顶层默认 Model，
/// 最后才使用系统默认值。
///
/// # 参数
///
/// - `config`: 配置对象的引用
///
/// # 返回值
///
/// 返回默认 Model 的名称字符串
///
/// # 默认值
///
/// 如果配置中未指定，默认使用 `"zhipuai-coding-plan/glm-5"`
pub(crate) fn resolved_default_model(config: &Config) -> String {
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
pub(crate) fn runtime_defaults_from_config(config: &Config) -> ChannelRuntimeDefaults {
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
pub(crate) fn runtime_autonomy_policy_from_config(config: &Config) -> RuntimeAutonomyPolicy {
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

/// 计算运行时配置文件的路径
///
/// 根据 VibeWindow 目录位置，构建配置文件的完整路径。
///
/// # 参数
///
/// - `ctx`: 通道运行时上下文的引用
///
/// # 返回值
///
/// 如果存在 VibeWindow 目录，返回 `Some(配置文件路径)`
/// 否则返回 `None`
///
/// # 配置文件名
///
/// 项目覆盖文件固定命名为 `.vibewindow.json`
pub(crate) fn runtime_config_path(ctx: &ChannelRuntimeContext) -> Option<PathBuf> {
    ctx.provider_runtime_options.vibewindow_dir.as_ref().map(|dir| dir.join(".vibewindow.json"))
}

/// 获取运行时默认值的快照
///
/// 尝试从配置缓存中获取默认值，如果缓存不存在则从上下文构建。
///
/// # 参数
///
/// - `ctx`: 通道运行时上下文的引用
///
/// # 返回值
///
/// 返回 `ChannelRuntimeDefaults` 实例
///
/// # 工作流程
///
/// 1. 检查是否有配置文件路径
/// 2. 如果有，尝试从全局存储中获取缓存的配置
/// 3. 如果缓存不存在，从上下文参数构建默认值
///
/// # 性能考虑
///
/// 使用缓存避免重复加载和解析配置文件
pub(crate) fn runtime_defaults_snapshot(ctx: &ChannelRuntimeContext) -> ChannelRuntimeDefaults {
    // 尝试从配置文件路径获取缓存的默认值
    if let Some(config_path) = runtime_config_path(ctx) {
        // 锁定全局存储，处理可能的 poison 错误
        let store = runtime_config_store().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = store.get(&config_path) {
            return state.defaults.clone();
        }
    }

    // 缓存未命中，从上下文构建默认值
    ChannelRuntimeDefaults {
        default_provider: ctx.default_provider.as_str().to_string(),
        model: ctx.model.as_str().to_string(),
        temperature: ctx.temperature,
        api_key: ctx.api_key.clone(),
        api_url: ctx.api_url.clone(),
        reliability: (*ctx.reliability).clone(),
    }
}

/// 获取非 CLI 环境下排除工具的快照
///
/// 从上下文中获取当前的非 CLI 排除工具列表。
///
/// # 参数
///
/// - `ctx`: 通道运行时上下文的引用
///
/// # 返回值
///
/// 返回排除工具名称的列表
///
/// # 线程安全
///
/// 使用 `unwrap_or_else` 处理可能的锁 poison 情况
pub(crate) fn snapshot_non_cli_excluded_tools(ctx: &ChannelRuntimeContext) -> Vec<String> {
    ctx.non_cli_excluded_tools.lock().unwrap_or_else(|e| e.into_inner()).clone()
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
pub(crate) async fn config_file_stamp(path: &Path) -> Option<ConfigFileStamp> {
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
pub(crate) fn decrypt_optional_secret_for_runtime_reload(
    store: &crate::app::agent::security::SecretStore,
    value: &mut Option<String>,
    field_name: &str,
) -> Result<()> {
    // 检查字段是否有值
    if let Some(raw) = value.clone() {
        // 检查值是否被加密
        if crate::app::agent::security::SecretStore::is_encrypted(&raw) {
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
pub(crate) async fn load_runtime_defaults_from_config_file(
    path: &Path,
) -> Result<(
    ChannelRuntimeDefaults,
    RuntimeAutonomyPolicy,
    crate::app::agent::config::ChannelsConfig,
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
            let store = crate::app::agent::security::SecretStore::new(
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
        apply_env_overrides(&mut parsed);

        // 构建并返回默认值和策略
        Ok((
            runtime_defaults_from_config(&parsed),
            runtime_autonomy_policy_from_config(&parsed),
            parsed.channels_config,
        ))
    }
}

/// 检测并应用运行时配置更新
///
/// 检查配置文件是否有变化，如果有则重新加载配置并更新运行时上下文。
/// 这是一个热更新机制，允许在不重启服务的情况下更新配置。
///
/// # 参数
///
/// - `ctx`: 通道运行时上下文的引用
///
/// # 返回值
///
/// - 成功（包括无更新）：返回 `Ok(())`
/// - 失败：返回错误（配置加载失败、Provider 创建失败等）
///
/// # 工作流程
///
/// 1. 获取配置文件路径，如果没有则直接返回
/// 2. 获取文件时间戳，如果失败则直接返回
/// 3. 检查缓存中是否已有相同时间戳的配置，有则跳过
/// 4. 加载新的配置文件
/// 5. 创建新的 Provider 实例
/// 6. 预热 Provider（失败仅记录警告）
/// 7. 更新 Provider 缓存
/// 8. 更新配置存储
/// 9. 更新审批管理器的策略
/// 10. 更新非 CLI 排除工具列表
/// 11. 记录更新日志
///
/// # 热更新特性
///
/// - **Provider 切换**：支持动态切换 AI Provider
/// - **模型变更**：支持动态更改使用的模型
/// - **策略调整**：支持动态调整工具审批策略
/// - **工具配置**：支持动态增减可用工具
///
/// # 错误处理
///
/// - 配置文件读取/解析失败：返回错误
/// - Provider 创建失败：返回错误
/// - Provider 预热失败：仅记录警告，不中断流程
///
/// # 线程安全
///
/// 使用互斥锁保护共享状态的更新，处理可能的锁 poison 情况
pub(crate) async fn maybe_apply_runtime_config_update(ctx: &ChannelRuntimeContext) -> Result<()> {
    // 获取配置文件路径，不存在则跳过
    let Some(config_path) = runtime_config_path(ctx) else {
        return Ok(());
    };

    // 获取文件时间戳，失败则跳过
    let Some(stamp) = config_file_stamp(&config_path).await else {
        return Ok(());
    };

    // 检查缓存中是否已有最新配置
    {
        let store = runtime_config_store().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = store.get(&config_path) {
            // 时间戳相同，说明配置未变化，跳过更新
            if state.last_applied_stamp == Some(stamp) {
                return Ok(());
            }
        }
    }

    // 加载新的配置文件
    let (next_defaults, next_autonomy_policy, next_channels_config) =
        load_runtime_defaults_from_config_file(&config_path).await?;

    // 创建新的 Provider 实例（带弹性策略）
    let next_default_provider =
        crate::app::agent::providers::create_resilient_provider_with_options(
            &next_defaults.default_provider,
            next_defaults.api_key.as_deref(),
            next_defaults.api_url.as_deref(),
            &next_defaults.reliability,
            &ctx.provider_runtime_options,
        )?;
    let next_default_provider: Arc<dyn Provider> = Arc::from(next_default_provider);

    // 尝试预热 Provider（失败不影响整体流程）
    if let Err(err) = next_default_provider.warmup().await {
        tracing::warn!(
            provider = %next_defaults.default_provider,
            "Provider warmup failed after config reload: {err}"
        );
    }

    ctx.project_state.lock().unwrap_or_else(|e| e.into_inner()).config = next_channels_config;

    // 更新 Provider 缓存
    {
        let mut cache = ctx.provider_cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.clear();
        cache.insert(next_defaults.default_provider.clone(), Arc::clone(&next_default_provider));
    }

    // 更新配置存储
    {
        let mut store = runtime_config_store().lock().unwrap_or_else(|e| e.into_inner());
        store.insert(
            config_path.clone(),
            RuntimeConfigState { defaults: next_defaults.clone(), last_applied_stamp: Some(stamp) },
        );
    }

    // 更新审批管理器的运行时策略
    ctx.approval_manager.replace_runtime_non_cli_policy(
        &next_autonomy_policy.auto_approve,
        &next_autonomy_policy.always_ask,
        &next_autonomy_policy.non_cli_approval_approvers,
        next_autonomy_policy.non_cli_natural_language_approval_mode,
        &next_autonomy_policy.non_cli_natural_language_approval_mode_by_channel,
    );

    // 更新非 CLI 排除工具列表
    {
        let mut excluded = ctx.non_cli_excluded_tools.lock().unwrap_or_else(|e| e.into_inner());
        *excluded = next_autonomy_policy.non_cli_excluded_tools.clone();
    }

    // 记录配置更新日志
    tracing::info!(
        path = %config_path.display(),
        provider = %next_defaults.default_provider,
        model = %next_defaults.model,
        temperature = next_defaults.temperature,
        non_cli_approval_mode = %non_cli_natural_language_mode_label(
            next_autonomy_policy.non_cli_natural_language_approval_mode
        ),
        non_cli_excluded_tools_count = next_autonomy_policy.non_cli_excluded_tools.len(),
        "Applied updated channel runtime config from disk"
    );

    Ok(())
}
