//! # 通道启动模块
//!
//! 本模块负责启动所有已配置的通信通道（Channel），并建立消息路由机制，将来自各通道的消息
//! 分发给代理核心进行处理。
//!
//! ## 主要功能
//!
//! - **通道初始化**：根据配置加载并初始化所有通信通道（如 Telegram、Discord、Slack 等）
//! - **Provider 设置**：创建并配置 AI 模型提供者实例，包括连接池预热
//! - **工具注册**：构建完整的工具注册表，供代理在处理消息时调用
//! - **系统提示构建**：从工作区身份文件和技能列表构建系统提示
//! - **消息分发循环**：运行消息分发主循环，协调所有通道的消息处理
//!
//! ## 架构设计
//!
//! 该模块采用以下架构模式：
//! 1. **单消息总线模式**：所有通道将消息发送到统一的消息总线（MPSC 通道）
//! 2. **监督监听器模式**：每个通道都有独立的监督监听器任务，具备重试和退避机制
//! 3. **运行时上下文共享**：通过 `ChannelRuntimeContext` 共享所有必要的运行时资源
//!
//! ## 使用场景
//!
//! 此模块通常在 VibeWindow 的通道服务器模式下被调用，用于启动多通道消息监听服务。

use super::*;

/// 启动所有已配置的通道并将消息路由到代理核心
///
/// 该函数是通道管理器的核心入口点，负责：
/// 1. 设置项目目录覆盖存储
/// 2. 创建并配置默认的 Provider 实例
/// 3. 预热 Provider 连接池（执行 TLS 握手、DNS 解析、HTTP/2 设置）
/// 4. 初始化运行时配置状态
/// 5. 创建观测性观察器和运行时适配器
/// 6. 配置安全策略和内存存储
/// 7. 构建工具注册表和技能列表
/// 8. 生成系统提示
/// 9. 收集并初始化所有配置的通道
/// 10. 启动监督监听器任务
/// 11. 运行消息分发循环
///
/// # 参数
///
/// * `config` - 完整的应用配置，包含通道、Provider、内存、安全等所有配置项
///
/// # 返回值
///
/// * `Result<()>` - 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 错误
///
/// 该函数可能在以下情况下返回错误：
/// - Provider 创建失败
/// - 内存存储初始化失败
/// - 所有配置的通道在初始化时都失败
/// - 运行时适配器创建失败
///
/// # 示例
///
/// ```rust,no_run
/// use vibewindow::app::agent::config::Config;
/// use vibewindow::app::agent::channels::manager::start::start_channels;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let config = Config::load("config.toml").await?;
///     start_channels(config).await?;
///     Ok(())
/// }
/// ```
///
/// # 内部流程
///
/// 1. **配置存储初始化**：将项目目录存储到全局覆盖存储中
/// 2. **Provider 创建**：根据配置创建带路由功能的 Provider 实例
/// 3. **连接预热**：预热 Provider 连接池，避免首次消息超时
/// 4. **运行时状态初始化**：将初始配置状态存储到运行时配置存储中
/// 5. **核心组件创建**：创建观察器、运行时适配器、安全策略、内存存储
/// 6. **工具注册**：构建包含所有可用工具的注册表
/// 7. **技能加载**：从工作区加载所有已配置的技能
/// 8. **工具描述收集**：收集所有工具的描述信息，用于构建系统提示
/// 9. **系统提示构建**：结合工作区身份、模型信息、工具描述和技能构建系统提示
/// 10. **通道收集**：收集所有配置的通道，包括 Nostr 通道（如果可用）
/// 11. **消息总线创建**：创建单消息总线用于所有通道的消息传输
/// 12. **监听器启动**：为每个通道启动监督监听器任务
/// 13. **分发循环运行**：运行消息分发循环，处理来自所有通道的消息
#[allow(clippy::too_many_lines)]
pub async fn start_channels(config: Config) -> Result<()> {
    let timezone =
        crate::app::agent::util::locale::parse_timezone(&config.app_ui.system_settings.timezone)?;

    // 设置项目目录覆盖存储，允许通道访问项目目录配置
    *channel_project_dir_override_store().lock().unwrap_or_else(|e| e.into_inner()) =
        config.channels_config.project_dir.clone();

    // 解析默认 Provider 名称和模型
    let provider_name = resolved_default_provider(&config);
    let model = resolved_default_model(&config);

    // 构建 Provider 运行时选项
    let provider_runtime_options = build_provider_runtime_options(&config);

    // 创建带路由功能的 Provider 实例
    let provider: Arc<dyn Provider> = Arc::from(
        create_routed_provider_nonblocking(
            &provider_name,
            config.api_key.clone(),
            config.api_url.clone(),
            config.reliability.clone(),
            config.model_routes.clone(),
            model.clone(),
            provider_runtime_options.clone(),
        )
        .await?,
    );

    // 预热 Provider 连接池（TLS 握手、DNS 解析、HTTP/2 设置）
    // 确保第一条真实消息不会因冷启动而超时
    if let Err(e) = provider.warmup().await {
        tracing::warn!("Provider warmup failed (non-fatal): {e}");
    }

    // 项目覆盖仅从工作区加载，系统基础配置由共享数据库提供。
    let project_config_path = config.workspace_dir.join(".vibewindow.json");
    let initial_stamp = config_file_stamp(&project_config_path).await;

    // 初始化运行时配置状态存储
    {
        let mut store = runtime_config_store().lock().unwrap_or_else(|e| e.into_inner());
        store.insert(
            project_config_path,
            RuntimeConfigState {
                defaults: runtime_defaults_from_config(&config),
                last_applied_stamp: initial_stamp,
            },
        );
    }

    // 创建核心组件实例
    let observer: Arc<dyn Observer> =
        Arc::from(observability::create_observer(&config.observability));
    let runtime: Arc<dyn runtime::RuntimeAdapter> =
        Arc::from(runtime::create_runtime(&config.runtime)?);
    let security = Arc::new(SecurityPolicy::from_configs(
        &config.autonomy,
        &config.security,
        &config.workspace_dir,
    ));
    let temperature = config.default_temperature;
    let mem: Arc<dyn Memory> = Arc::from(memory::create_memory_with_storage(
        &config.memory,
        Some(&config.storage.provider.config),
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);

    // 构建工具注册表，包含所有可用工具和运行时配置
    let workspace = config.workspace_dir.clone();
    let tools_registry = Arc::new(crate::app::agent::tools::all_tools_with_runtime(
        Arc::new(config.clone()),
        &security,
        runtime,
        Arc::clone(&mem),
        &config.browser,
        &config.http_request,
        &config.web_fetch,
        &workspace,
        &config.agents,
        config.api_key.as_deref(),
        &config,
        None,
    ));

    // 从工作区加载所有已配置的技能
    let skills = skills::load_skills_with_config(&workspace, &config);

    // 收集工具描述信息，用于构建系统提示
    // 这些描述将帮助 AI 理解每个工具的用途和使用场景
    let excluded = &config.autonomy.non_cli_excluded_tools;
    let agent_tool_registered = tools_registry.iter().any(|tool| tool.name() == "AgentTool");
    let tool_descs = build_channel_tool_descriptions(&config, agent_tool_registered);

    // 构建系统提示
    let bootstrap_max_chars = bootstrap_max_chars(&config);
    let native_tools = provider.supports_native_tools();
    let mut system_prompt = build_system_prompt_with_mode(
        &workspace,
        &model,
        &tool_descs,
        &skills,
        Some(&config.identity),
        bootstrap_max_chars,
        native_tools,
        config.skills.prompt_injection_mode,
        timezone,
    );

    // 如果 Provider 不支持原生工具，添加工具指令
    if !native_tools {
        let filtered_specs = filtered_tool_specs_for_runtime(tools_registry.as_ref(), excluded);
        system_prompt.push_str(&build_tool_instructions_from_specs(&filtered_specs));
    }
    system_prompt.push_str(&build_shell_policy_instructions(&config.autonomy));

    // 显示已加载的技能信息
    if !skills.is_empty() {
        println!(
            "  🧩 Skills:   {}",
            skills.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ")
        );
    }

    // 从共享构建器收集活动通道，保持启动和诊断的奇偶性
    let mut configured_channels = collect_configured_channels(&config, "runtime startup");
    let mut init_failures = Vec::new();

    // 尝试附加 Nostr 通道（如果可用）
    if let Some(reason) =
        append_nostr_channel_if_available(&config, &mut configured_channels, "runtime startup")
            .await
    {
        init_failures.push(reason);
    }

    // 检查是否有配置的通道
    if configured_channels.is_empty() && init_failures.is_empty() {
        println!("No channels configured.");
        return Ok(());
    }

    // 如果所有通道初始化都失败，返回错误
    if configured_channels.is_empty() && !init_failures.is_empty() {
        for failure in &init_failures {
            println!("  ❌ {failure}");
        }
        anyhow::bail!("All configured channels failed during initialization.");
    }

    // 显示初始化失败的警告
    if !init_failures.is_empty() {
        for failure in &init_failures {
            println!("  ⚠️  {failure}");
        }
        println!();
    }

    // 提取通道实例
    let channels: Vec<Arc<dyn Channel>> =
        configured_channels.into_iter().map(|configured| configured.channel).collect();

    // 显示启动信息
    println!("VibeWindow Channel Server");
    println!(" Model:    {model}");
    let effective_backend = memory::effective_memory_backend_name(
        &config.memory.backend,
        Some(&config.storage.provider.config),
    );
    println!(
        "  💡 Memory:   {} (auto-save: {})",
        effective_backend,
        if config.memory.auto_save { "on" } else { "off" }
    );
    println!("  Channels: {}", channels.iter().map(|c| c.name()).collect::<Vec<_>>().join(", "));
    println!();
    println!("  Listening for messages... (Ctrl+C to stop)");
    println!();

    // 标记通道组件健康状态为正常
    crate::app::agent::health::mark_component_ok("channels");

    // 计算退避时间参数
    let (initial_backoff_secs, max_backoff_secs) = channel_listener_backoff_secs(&config);

    // 创建单消息总线 - 所有通道将消息发送到这里
    let (tx, rx) = tokio::sync::mpsc::channel::<traits::ChannelMessage>(100);

    // 为每个通道启动监督监听器任务
    let mut handles = Vec::new();
    for ch in &channels {
        handles.push(spawn_supervised_listener(
            ch.clone(),
            tx.clone(),
            initial_backoff_secs,
            max_backoff_secs,
        ));
    }
    drop(tx); // 释放我们的副本，以便在所有通道停止时关闭 rx

    // 构建通道名称到通道实例的映射
    let channels_by_name = build_channels_by_name(&channels);

    // 计算最大并发消息数
    let max_in_flight_messages = compute_max_in_flight_messages(channels.len());

    println!(" In-flight message limit: {max_in_flight_messages}");

    // 初始化 Provider 缓存
    let provider_cache_seed = seed_provider_cache(&provider_name, &provider);
    let message_timeout_secs =
        effective_channel_message_timeout_secs(config.channels_config.message_timeout_secs);
    let interrupt_on_new_message = channel_interrupt_on_new_message(&config);
    let main_agent = vw_config_types::agent::effective_main_agent_config(&config.agents);

    // 构建通道运行时上下文
    let runtime_ctx = Arc::new(ChannelRuntimeContext {
        channels_by_name,
        provider: Arc::clone(&provider),
        default_provider: Arc::new(provider_name),
        memory: Arc::clone(&mem),
        tools_registry: Arc::clone(&tools_registry),
        observer,
        system_prompt: Arc::new(system_prompt),
        model: Arc::new(model.clone()),
        temperature,
        auto_save_memory: config.memory.auto_save,
        max_iterations: main_agent.max_iterations,
        min_relevance_score: config.memory.min_relevance_score,
        conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        provider_cache: Arc::new(Mutex::new(provider_cache_seed)),
        route_overrides: Arc::new(Mutex::new(HashMap::new())),
        api_key: config.api_key.clone(),
        api_url: config.api_url.clone(),
        reliability: Arc::new(config.reliability.clone()),
        provider_runtime_options,
        workspace_dir: Arc::new(config.workspace_dir.clone()),
        message_timeout_secs,
        interrupt_on_new_message,
        multimodal: config.multimodal.clone(),
        hooks: build_channel_hooks(&config)?,
        non_cli_excluded_tools: Arc::new(Mutex::new(
            config.autonomy.non_cli_excluded_tools.clone(),
        )),
        query_classification: config.query_classification.clone(),
        model_routes: config.model_routes.clone(),
        approval_manager: Arc::new(ApprovalManager::from_config(&config.autonomy)),
    });

    // 运行消息分发循环
    run_message_dispatch_loop(rx, runtime_ctx, max_in_flight_messages).await;

    // 等待所有通道任务完成
    for h in handles {
        let _ = h.await;
    }

    Ok(())
}

fn build_provider_runtime_options(
    config: &Config,
) -> crate::app::agent::providers::ProviderRuntimeOptions {
    crate::app::agent::providers::ProviderRuntimeOptions {
        configured_provider_ids: config.providers.keys().cloned().collect(),
        auth_profile_override: None,
        provider_api_url: config.api_url.clone(),
        vibewindow_dir: Some(config.workspace_dir.clone()),
        secrets_encrypt: config.secrets.encrypt,
        reasoning_enabled: config.runtime.reasoning_enabled,
        reasoning_level: config.effective_provider_reasoning_level(),
        custom_provider_api_mode: config.provider_api.map(|mode| mode.as_compatible_mode()),
        max_tokens_override: None,
        model_support_vision: config.model_support_vision,
    }
}

fn build_channel_tool_descriptions(
    config: &Config,
    agent_tool_registered: bool,
) -> Vec<(&'static str, &'static str)> {
    let mut tool_descs = vec![
        (
            "shell",
            "Execute terminal commands. Use when: running local checks, build/test commands, diagnostics. Don't use when: a safer dedicated tool exists, or command is destructive without approval.",
        ),
        (
            "file_read",
            "Read file contents. Use when: inspecting project files, configs, logs. Don't use when: a targeted search is enough.",
        ),
        (
            "file_write",
            "Write file contents. Use when: applying focused edits, scaffolding files, updating docs/code. Don't use when: side effects are unclear or file ownership is uncertain.",
        ),
        (
            "memory_store",
            "Save to memory. Use when: preserving durable preferences, decisions, key context. Don't use when: information is transient/noisy/sensitive without need.",
        ),
        (
            "memory_recall",
            "Search memory. Use when: retrieving prior decisions, user preferences, historical context. Don't use when: answer is already in current context.",
        ),
        (
            "memory_forget",
            "Delete a memory entry. Use when: memory is incorrect/stale or explicitly requested for removal. Don't use when: impact is uncertain.",
        ),
    ];

    if config.browser.enabled {
        if !config.browser.browser_open.eq_ignore_ascii_case("disable") {
            tool_descs.push((
                "BrowserOpen",
                "Open approved HTTPS URLs in system browser (allowlist-only, no scraping)",
            ));
        }
        tool_descs.push((
            "Browser",
            "Drive a browser for navigation, DOM inspection, and page interaction",
        ));
    }
    if config.web_fetch.enabled {
        tool_descs.push((
            "WebFetch",
            "Fetch approved web pages and convert them to markdown, text, or html",
        ));
    }
    if config.web_search.enabled {
        tool_descs.push((
            "WebSearch",
            "Search the web and return structured search results with sources",
        ));
    }
    tool_descs.push((
        "schedule",
        "Manage scheduled tasks (create/list/get/cancel/pause/resume). Supports recurring cron and one-shot delays.",
    ));
    if agent_tool_registered {
        tool_descs.push((
            "AgentTool",
            "Delegate complex work to persistent sub-agents; skip only genuinely simple single-step tasks. Use launch (or spawn) to start independent work in the background and in parallel, run when the result is an immediate dependency, status to inspect tasks, wait for event-driven any/all completion with a bounded timeout, and stop to cancel owned tasks. Continue useful parent work after launch, then verify and synthesize required results before answering.",
        ));
    }

    let excluded = &config.autonomy.non_cli_excluded_tools;
    if !excluded.is_empty() {
        tool_descs.retain(|(name, _)| !excluded.iter().any(|ex| ex == name));
    }

    tool_descs
}

fn bootstrap_max_chars(config: &Config) -> Option<usize> {
    vw_config_types::agent::effective_main_agent_config(&config.agents)
        .compact_context
        .then_some(6000)
}

fn channel_listener_backoff_secs(config: &Config) -> (u64, u64) {
    (
        config.reliability.channel_initial_backoff_secs.max(DEFAULT_CHANNEL_INITIAL_BACKOFF_SECS),
        config.reliability.channel_max_backoff_secs.max(DEFAULT_CHANNEL_MAX_BACKOFF_SECS),
    )
}

fn build_channels_by_name(channels: &[Arc<dyn Channel>]) -> Arc<HashMap<String, Arc<dyn Channel>>> {
    Arc::new(
        channels
            .iter()
            .map(|ch| (ch.name().to_string(), Arc::clone(ch)))
            .collect::<HashMap<_, _>>(),
    )
}

fn seed_provider_cache(
    provider_name: &str,
    provider: &Arc<dyn Provider>,
) -> HashMap<String, Arc<dyn Provider>> {
    let mut provider_cache_seed: HashMap<String, Arc<dyn Provider>> = HashMap::new();
    provider_cache_seed.insert(provider_name.to_string(), Arc::clone(provider));
    provider_cache_seed
}

fn channel_interrupt_on_new_message(config: &Config) -> bool {
    config.channels_config.telegram.as_ref().is_some_and(|tg| tg.interrupt_on_new_message)
}

fn build_channel_hooks(
    config: &Config,
) -> Result<Option<Arc<crate::app::agent::hooks::HookRunner>>> {
    Ok(crate::app::agent::hooks::build_hook_runtime(config)?.runner_snapshot())
}

#[cfg(test)]
#[path = "start_tests.rs"]
mod start_tests;
