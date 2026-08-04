//! CLI 代理环境初始化模块
//!
//! 本模块提供 CLI 模式下代理运行所需的所有组件的初始化逻辑。
//! 它是 CLI 代理启动流程的核心部分，负责组装和配置代理的各个子系统。
//!
//! # 主要职责
//!
//! - 初始化观测性系统（Observer），用于记录代理行为和事件
//! - 创建运行时适配器（Runtime Adapter），支持 native/docker/wasm 等执行环境
//! - 配置安全策略（Security Policy），控制代理的行为边界
//! - 设置内存系统（Memory），提供持久化存储能力
//! - 注册工具集（Tools Registry），为代理提供执行能力
//! - 创建 Provider 实例，连接 AI 模型服务
//! - 构建系统提示词（System Prompt），定义代理行为规范
//!
//! # 使用场景
//!
//! 此模块主要用于 `vibe-agent` CLI 入口，在交互模式和守护进程模式下都会被调用。
//! 根据模式不同，会启用或禁用某些功能（如审批管理器）。

use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::Config;
use crate::app::agent::memory::{self, Memory};
use crate::app::agent::observability::{self, Observer, ObserverEvent};
use crate::app::agent::providers::Provider;
use crate::app::agent::runtime;
use crate::app::agent::security::SecurityPolicy;
use anyhow::Result;
use std::sync::Arc;

use crate::app::agent::agent::loop_::instructions::{
    build_shell_policy_instructions, build_tool_instructions,
};

/// CLI 代理运行环境的初始化结果
///
/// 此结构体包含 CLI 模式下代理循环所需的所有已初始化组件。
/// 它是 `setup_cli` 函数的返回类型，作为代理主循环的输入。
///
/// # 生命周期
///
/// 此结构体的实例通常在代理启动时创建一次，并在整个代理生命周期中保持有效。
/// 所有字段都使用 `Arc` 或 `Box` 包装，以支持跨线程共享和 trait 对象。
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::config::Config;
/// use crate::app::agent::agent::loop_::cli::setup::{setup_cli, CliSetup};
///
/// let config = Config::load_default()?;
/// let setup = setup_cli(&config, true, None, None)?;
///
/// // 使用 setup.observer 记录事件
/// // 使用 setup.provider 发送模型请求
/// // 使用 setup.tools_registry 执行工具
/// ```
pub(crate) struct CliSetup {
    /// 观测性系统实例
    ///
    /// 用于记录代理生命周期中的各种事件，如启动、停止、错误等。
    /// 支持多种后端（日志、指标、追踪）。
    pub(crate) observer: Arc<dyn Observer>,

    /// 内存系统实例
    ///
    /// 提供持久化存储能力，允许代理保存和检索上下文信息、
    /// 用户偏好、历史决策等。支持多种后端（SQLite、PostgreSQL、向量数据库等）。
    pub(crate) mem: Arc<dyn Memory>,

    /// AI 模型提供者实例
    ///
    /// 负责与 AI 模型服务通信，处理请求和响应。
    /// 可以是 OpenAI、Anthropic、本地模型等任何实现了 `Provider` trait 的提供者。
    pub(crate) provider: Box<dyn Provider>,

    /// 已注册的工具集合
    ///
    /// 包含代理可调用的所有工具，如文件操作、Shell 命令、
    /// 网络请求、内存操作等。每个工具都实现了 `Tool` trait。
    pub(crate) tools_registry: Vec<Box<dyn crate::app::agent::tools::Tool>>,

    /// 提供者名称
    ///
    /// 标识正在使用的 AI 模型提供者，如 "openai"、"anthropic"、"zhipuai-coding-plan" 等。
    /// 用于日志记录和事件追踪。
    pub(crate) provider_name: String,

    /// 模型名称
    ///
    /// 标识正在使用的具体模型，如 "gpt-4"、"claude-3-opus"、"zhipuai-coding-plan/glm-4.7" 等。
    /// 用于日志记录和事件追踪。
    pub(crate) model_name: String,

    /// 系统提示词
    ///
    /// 定义代理的行为规范、能力和约束。包括：
    /// - 身份和角色定义
    /// - 工具使用指南
    /// - Shell 策略说明
    /// - 技能描述
    ///
    /// 系统提示词会在每次对话开始时发送给模型。
    pub(crate) system_prompt: String,

    /// 审批管理器（可选）
    ///
    /// 在交互模式下启用，用于在执行高风险操作前请求用户确认。
    /// 在守护进程模式下为 `None`，表示代理自主执行。
    pub(crate) approval_manager: Option<ApprovalManager>,

    /// 通道名称
    ///
    /// 标识代理的运行通道，用于日志和事件分类。
    /// - "cli": 交互模式，用户直接交互
    /// - "daemon": 守护进程模式，后台运行
    pub(crate) channel_name: &'static str,
}

pub(super) fn resolve_setup_name(
    override_value: Option<&str>,
    configured_value: Option<&str>,
    fallback: &str,
) -> String {
    override_value.or(configured_value).unwrap_or(fallback).to_string()
}

pub(super) fn composio_credentials<'a, 'b>(
    enabled: bool,
    api_key: Option<&'a str>,
    entity_id: &'b str,
) -> (Option<&'a str>, Option<&'b str>) {
    if enabled { (api_key, Some(entity_id)) } else { (None, None) }
}

pub(super) fn build_tool_descriptions(
    browser_enabled: bool,
    composio_enabled: bool,
    has_agent_tool: bool,
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
        (
            "cron_add",
            "Create a cron job. Supports schedule kinds: cron, at, every; and job types: shell or agent.",
        ),
        ("cron_list", "List all cron jobs with schedule, status, and metadata."),
        ("cron_remove", "Remove a cron job by job_id."),
        (
            "cron_update",
            "Patch a cron job (schedule, enabled, command/prompt, model, delivery, session_target).",
        ),
        ("cron_run", "Force-run a cron job immediately and record a run history entry."),
        ("cron_runs", "Show recent run history for a cron job."),
        (
            "screenshot",
            "Capture a screenshot of the current screen. Returns file path and base64-encoded PNG. Use when: visual verification, UI inspection, debugging displays.",
        ),
        (
            "image_info",
            "Read image file metadata (format, dimensions, size) and optionally base64-encode it. Use when: inspecting images, preparing visual data for analysis.",
        ),
    ];

    if browser_enabled {
        tool_descs.push((
            "browser_open",
            "Open approved HTTPS URLs in system browser (allowlist-only, no scraping)",
        ));
    }

    if composio_enabled {
        tool_descs.push((
            "composio",
            "Execute actions on 1000+ apps via Composio (Gmail, Notion, GitHub, Slack, etc.). Use action='list' to discover, 'execute' to run (optionally with connected_account_id), 'connect' to OAuth.",
        ));
    }

    tool_descs.push((
        "schedule",
        "Manage scheduled tasks (create/list/get/cancel/pause/resume). Supports recurring cron and one-shot delays.",
    ));
    tool_descs.push((
        "model_routing_config",
        "Configure default model, scenario routing, and delegate agents. Use for natural-language requests like: 'set conversation to kimi and coding to gpt-5.3-codex'.",
    ));

    if has_agent_tool {
        tool_descs.push((
            "AgentTool",
            "Delegate complex work to persistent sub-agents; skip only genuinely simple single-step tasks. Use launch (or spawn) to start independent work in the background and in parallel, run when the result is an immediate dependency, status to inspect tasks, wait for event-driven any/all completion with a bounded timeout, and stop to cancel owned tasks. Continue useful parent work after launch, then verify and synthesize required results before answering.",
        ));
    }

    tool_descs
}

pub(super) fn compact_context_max_chars(compact_context: bool) -> Option<usize> {
    if compact_context { Some(6000) } else { None }
}

pub(super) fn setup_channel_name(interactive: bool) -> &'static str {
    if interactive { "cli" } else { "daemon" }
}

/// 初始化 CLI 代理运行环境
///
/// 此函数是 CLI 代理启动流程的核心，负责组装代理所需的所有组件。
/// 它会根据配置和参数创建观测系统、内存、安全策略、工具集、提供者等。
///
/// # 参数
///
/// * `config` - 代理配置对象，包含所有配置信息（路径、密钥、策略等）
/// * `interactive` - 是否为交互模式
///   - `true`: 启用审批管理器，高风险操作需用户确认
///   - `false`: 守护进程模式，代理自主执行
/// * `provider_override` - 可选的提供者名称覆盖，优先级高于配置文件
/// * `model_override` - 可选的模型名称覆盖，优先级高于配置文件
///
/// # 返回值
///
/// 返回 `Result<CliSetup>`：
/// - `Ok(CliSetup)`: 初始化成功，包含所有已配置的组件
/// - `Err(anyhow::Error)`: 初始化失败，可能原因包括：
///   - 配置错误（无效的提供者名称、模型名称等）
///   - 资源创建失败（无法创建内存后端、运行时适配器等）
///   - 权限问题（无法访问工作目录、密钥文件等）
///
/// # 工作流程
///
/// 1. **初始化观测系统**: 创建 Observer 实例，用于记录事件
/// 2. **创建运行时适配器**: 根据配置选择执行环境（native/docker/wasm）
/// 3. **加载安全策略**: 从配置构建安全边界，控制代理行为
/// 4. **初始化内存系统**: 创建持久化存储后端
/// 5. **注册工具集**: 根据配置和运行时创建所有可用工具
/// 6. **确定 Provider 和模型**: 应用覆盖值或使用配置默认值
/// 7. **创建 Provider 实例**: 初始化模型服务连接
/// 8. **记录启动事件**: 通知观测系统代理已启动
/// 9. **加载技能**: 从配置目录加载可用技能
/// 10. **构建工具描述**: 为系统提示词准备工具使用指南
/// 11. **生成系统提示词**: 组合身份、工具、技能、策略等
/// 12. **配置审批管理器**: 交互模式下启用用户确认机制
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::config::Config;
///
/// // 从文件加载配置
/// let config = Config::load_default()?;
///
/// // 交互模式初始化（使用默认 provider 和 model）
/// let setup = setup_cli(&config, true, None, None)?;
///
/// // 覆盖 provider 和 model
/// let setup = setup_cli(&config, false, Some("openai"), Some("gpt-4-turbo"))?;
///
/// // 守护进程模式（自主执行，无需审批）
/// let setup = setup_cli(&config, false, None, None)?;
/// ```
///
/// # 安全考虑
///
/// - API 密钥和敏感信息不会记录到日志
/// - 安全策略会限制工具的执行范围
/// - 在非交互模式下，某些高风险操作可能被禁止
///
/// # 性能考虑
///
/// - 工具注册是懒加载的，只在首次使用时初始化
/// - 内存系统使用连接池优化性能
/// - Provider 使用重试和超时机制提高可靠性
pub(crate) fn setup_cli(
    config: &Config,
    interactive: bool,
    provider_override: Option<&str>,
    model_override: Option<&str>,
) -> Result<CliSetup> {
    // ========================================
    // 第一步：初始化核心基础设施
    // ========================================

    // 创建观测系统实例，用于记录代理行为和事件
    let base_observer = observability::create_observer(&config.observability);
    let observer: Arc<dyn Observer> = Arc::from(base_observer);

    // 创建运行时适配器，支持不同的执行环境（native/docker/wasm）
    let runtime: Arc<dyn runtime::RuntimeAdapter> =
        Arc::from(runtime::create_runtime(&config.runtime)?);

    // 从配置构建安全策略，定义代理的行为边界
    let security = Arc::new(SecurityPolicy::from_configs(
        &config.autonomy,
        &config.security,
        &config.workspace_dir,
    ));

    // ========================================
    // 第二步：初始化内存系统
    // ========================================

    // 创建内存系统实例，使用配置指定的存储后端
    let mem: Arc<dyn Memory> = Arc::from(memory::create_memory_with_storage(
        &config.memory,
        Some(&config.storage.provider.config),
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);
    tracing::info!(backend = mem.name(), "Memory initialized");

    // ========================================
    // 第三步：配置并注册工具集
    // ========================================

    // 解析 Composio 集成配置（用于第三方应用集成）
    let (composio_key, composio_entity_id) = composio_credentials(
        config.composio.enabled,
        config.composio.api_key.as_deref(),
        config.composio.entity_id.as_str(),
    );

    // 注册所有可用工具，传入必要的依赖项
    let tools_registry = crate::app::agent::tools::all_tools_with_runtime(
        Arc::new(config.clone()),  // 配置克隆
        &security,                 // 安全策略
        runtime,                   // 运行时适配器
        mem.clone(),               // 内存系统
        composio_key,              // Composio API 密钥
        composio_entity_id,        // Composio 实体 ID
        &config.browser,           // 浏览器配置
        &config.http_request,      // HTTP 请求配置
        &config.web_fetch,         // Web 抓取配置
        &config.workspace_dir,     // 工作目录
        &config.agents,            // 代理配置
        config.api_key.as_deref(), // API 密钥
        config,                    // 完整配置引用
        Some("cli"),               // 来源标识
    );

    // ========================================
    // 第四步：确定 Provider 和模型
    // ========================================

    // 确定提供者名称，优先级：命令行覆盖 > 配置文件默认值 > 硬编码默认值
    let provider_name = resolve_setup_name(
        provider_override,
        config.default_provider.as_deref(),
        "zhipuai-coding-plan",
    );

    // 确定模型名称，优先级：命令行覆盖 > 配置文件默认值 > 硬编码默认值
    let model_name = resolve_setup_name(
        model_override,
        config.default_model.as_deref(),
        "zhipuai-coding-plan/glm-4.7",
    );

    // ========================================
    // 第五步：创建 Provider 实例
    // ========================================

    // 构建 Provider 运行时选项
    let provider_runtime_options = crate::app::agent::providers::ProviderRuntimeOptions {
        auth_profile_override: None,              // 认证配置文件覆盖
        provider_api_url: config.api_url.clone(), // Provider API URL
        vibewindow_dir: config.config_path.parent().map(std::path::PathBuf::from), // 配置目录
        secrets_encrypt: config.secrets.encrypt,  // 密钥加密设置
        reasoning_enabled: config.runtime.reasoning_enabled, // 推理能力开关
        reasoning_level: config.effective_provider_reasoning_level(), // 推理级别
        custom_provider_api_mode: config.provider_api.map(|mode| mode.as_compatible_mode()), // 自定义 API 模式
        max_tokens_override: None, // 最大 token 数覆盖
        model_support_vision: config.model_support_vision, // 模型视觉支持
    };

    // 创建 Provider 实例，支持路由和重试机制
    let provider: Box<dyn Provider> =
        crate::app::agent::providers::create_routed_provider_with_options(
            &provider_name,
            config.api_key.as_deref(),
            config.api_url.as_deref(),
            &config.reliability,
            &config.model_routes,
            &model_name,
            &provider_runtime_options,
        )?;

    // 记录代理启动事件到观测系统
    observer.record_event(&ObserverEvent::AgentStart {
        provider: provider_name.to_string(),
        model: model_name.to_string(),
    });

    // ========================================
    // 第六步：构建系统提示词
    // ========================================

    // 加载技能定义
    let skills = crate::app::agent::skills::load_skills_with_config(&config.workspace_dir, config);

    // 构建工具描述列表，用于系统提示词
    // 每个工具包含名称和使用指南
    let has_agent_tool = tools_registry.iter().any(|tool| tool.name() == "AgentTool");
    let tool_descs =
        build_tool_descriptions(config.browser.enabled, config.composio.enabled, has_agent_tool);

    // 确定上下文压缩设置
    // 启用时会限制初始上下文大小以优化性能
    let bootstrap_max_chars = compact_context_max_chars(config.agent.compact_context);

    // 检查 Provider 是否支持原生工具调用
    let native_tools = provider.supports_native_tools();
    let timezone =
        vw_agent::util::locale::parse_timezone(&config.app_ui.system_settings.timezone)?;

    // 构建基础系统提示词
    let mut system_prompt = crate::app::agent::channels::build_system_prompt_with_mode(
        &config.workspace_dir,
        &model_name,
        &tool_descs,
        &skills,
        Some(&config.identity),
        bootstrap_max_chars,
        native_tools,
        config.skills.prompt_injection_mode,
        timezone,
    );

    // 如果 Provider 不支持原生工具，添加工具使用说明
    if !native_tools {
        system_prompt.push_str(&build_tool_instructions(&tools_registry));
    }

    // 添加 Shell 策略说明，定义命令执行规则
    system_prompt.push_str(&build_shell_policy_instructions(&config.autonomy));

    // ========================================
    // 第七步：配置审批和通道
    // ========================================

    // 在交互模式下创建审批管理器
    let approval_manager =
        if interactive { Some(ApprovalManager::from_config(&config.autonomy)) } else { None };

    // 确定通道名称（用于日志和事件分类）
    let channel_name = setup_channel_name(interactive);

    // ========================================
    // 返回初始化结果
    // ========================================

    Ok(CliSetup {
        observer,
        mem,
        provider,
        tools_registry,
        provider_name,
        model_name,
        system_prompt,
        approval_manager,
        channel_name,
    })
}
