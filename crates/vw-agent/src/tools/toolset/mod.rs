use super::context::ToolUseContext;
use super::*;
use crate::app::agent::config::schema::load_or_init_config;
use crate::app::agent::config::{Config, DelegateAgentConfig};
use crate::app::agent::memory::Memory;
use crate::app::agent::runtime::{NativeRuntime, RuntimeAdapter};
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use crate::session::runtime::SessionToolset;
use anyhow::Context;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use vw_config_types::agent::merged_agent_configs;

/// Claude Tools V2 运行时上下文。
///
/// 对外仍保留 `session/root` 这两个轻量字段，避免影响现有调用点；真正的共享
/// 运行时能力则集中放在内部的 `ToolUseContext` 中，以便权限、审批、hook 与
/// read_state 在不同入口之间复用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRuntimeContext {
    /// 当前会话标识。
    pub session: String,
    /// 当前工作区根目录。
    pub root: Option<String>,
    /// 厚上下文共享状态。
    #[serde(skip)]
    tool_use_context: Arc<ToolUseContext>,
    /// 只属于当前会话的预发现工具。
    #[serde(skip)]
    session_tools: SessionToolset,
    #[cfg(test)]
    #[serde(skip)]
    test_config: Option<Arc<Config>>,
}

impl Default for ToolRuntimeContext {
    fn default() -> Self {
        Self::new(
            "default",
            std::env::current_dir().ok().map(|path| path.to_string_lossy().to_string()),
        )
    }
}

impl ToolRuntimeContext {
    /// 创建新的运行时上下文。
    pub fn new(session: impl Into<String>, root: Option<String>) -> Self {
        let session = session.into();
        let tool_use_context = ToolUseContext::new(session.clone(), root.clone());
        Self {
            session,
            root,
            tool_use_context: Arc::new(tool_use_context),
            session_tools: SessionToolset::default(),
            #[cfg(test)]
            test_config: None,
        }
    }

    /// 构造用于工具规格枚举的默认上下文。
    pub fn for_specs() -> Self {
        Self::new(
            "specs",
            std::env::current_dir().ok().map(|path| path.to_string_lossy().to_string()),
        )
    }

    /// 用完整 ToolUseContext 覆盖内部共享状态。
    pub fn with_tool_use_context(mut self, tool_use_context: ToolUseContext) -> Self {
        self.tool_use_context = Arc::new(tool_use_context.with_root(self.root.clone()));
        self
    }

    /// 获取当前共享运行时上下文。
    pub fn tool_use_context(&self) -> Arc<ToolUseContext> {
        self.tool_use_context.clone()
    }

    /// 注入只属于当前会话的工具集合。
    pub fn with_session_tools(mut self, session_tools: SessionToolset) -> Self {
        self.session_tools = session_tools;
        self
    }

    /// Isolate config storage while exercising the real registry and permission pipeline.
    #[cfg(test)]
    pub(crate) fn with_test_config(mut self, config: Config) -> Self {
        self.test_config = Some(Arc::new(config));
        self
    }

    /// 返回当前会话的工具集合。
    pub fn session_tools(&self) -> &SessionToolset {
        &self.session_tools
    }
}

pub use super::executor::{ExecutedToolCall, ToolCallError};

#[derive(Clone)]
struct ArcDelegatingTool {
    inner: Arc<dyn Tool>,
}

impl ArcDelegatingTool {
    fn boxed(inner: Arc<dyn Tool>) -> Box<dyn Tool> {
        Box::new(Self { inner })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ArcDelegatingTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        self.inner.execute(args).await
    }

    fn validate_input(&self, input: Value) -> anyhow::Result<Value> {
        self.inner.validate_input(input)
    }

    async fn check_permissions(&self, input: &Value) -> anyhow::Result<()> {
        self.inner.check_permissions(input).await
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        self.inner.call(input).await
    }

    fn map_result_for_model(&self, result: &ToolCallResult) -> Value {
        self.inner.map_result_for_model(result)
    }

    fn render_hint(&self, result: &ToolCallResult) -> Option<ToolRenderHint> {
        self.inner.render_hint(result)
    }

    fn is_concurrency_safe(&self) -> bool {
        self.inner.is_concurrency_safe()
    }

    fn is_read_only(&self) -> bool {
        self.inner.is_read_only()
    }

    fn to_audit_input(&self, input: &Value) -> Value {
        self.inner.to_audit_input(input)
    }

    fn spec(&self) -> ToolSpec {
        self.inner.spec()
    }
}

fn boxed_registry_from_arcs(tools: Vec<Arc<dyn Tool>>) -> Vec<Box<dyn Tool>> {
    tools.into_iter().map(ArcDelegatingTool::boxed).collect()
}

/// 根据当前上下文构建可执行工具集。
pub(crate) fn execution_tools_for_context(ctx: &ToolRuntimeContext) -> Vec<Box<dyn Tool>> {
    execution_environment_for_context(ctx).0
}

fn execution_environment_for_context(
    ctx: &ToolRuntimeContext,
) -> (Vec<Box<dyn Tool>>, Arc<ToolUseContext>) {
    let config = runtime_config_for_context(ctx);
    execution_environment_for_config(ctx, config)
}

fn runtime_config_for_context(ctx: &ToolRuntimeContext) -> Arc<Config> {
    #[cfg(test)]
    if let Some(config) = &ctx.test_config {
        return config.clone();
    }
    let workspace_dir = ctx.tool_use_context().workspace_root().unwrap_or_else(|| {
        ctx.root.as_ref().map(std::path::PathBuf::from).unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        })
    });

    let config = block_on(async {
        match load_or_init_config().await {
            Ok(config) => Arc::new(config),
            Err(error) => {
                tracing::warn!(
                    "tool runtime: failed to load config, falling back to defaults: {error}"
                );
                let mut config = Config::default();
                config.workspace_dir = workspace_dir.clone();
                config.config_path = workspace_dir.join(".vibewindow.json");
                Arc::new(config)
            }
        }
    });

    config
}

const DEFAULT_AGENT_TOOL_PARALLEL_LIMIT: usize = 4;
const MAX_AGENT_TOOL_PARALLEL_LIMIT: usize = 32;

fn agent_tool_parallel_limit(
    agents: &HashMap<String, DelegateAgentConfig>,
    agent_name: &str,
) -> usize {
    agents
        .get(agent_name)
        .map(|config| config.max_parallel_subagents)
        .unwrap_or(DEFAULT_AGENT_TOOL_PARALLEL_LIMIT)
        .clamp(1, MAX_AGENT_TOOL_PARALLEL_LIMIT)
}

pub(crate) fn agent_tool_parallel_limit_for_context(
    ctx: &ToolRuntimeContext,
    agent_name: &str,
) -> usize {
    let config = runtime_config_for_context(ctx);
    agent_tool_parallel_limit(&config.agents, agent_name)
}

fn execution_environment_for_config(
    ctx: &ToolRuntimeContext,
    config: Arc<Config>,
) -> (Vec<Box<dyn Tool>>, Arc<ToolUseContext>) {
    let base_context = ctx.tool_use_context();
    let workspace_dir = base_context.workspace_root().unwrap_or_else(|| {
        ctx.root.as_ref().map(std::path::PathBuf::from).unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        })
    });
    let security = security_for_tool_context(&config, &workspace_dir, base_context.as_ref());
    let runtime: Arc<dyn RuntimeAdapter> = Arc::new(NativeRuntime::new());
    let mem_cfg = config.memory.clone();
    let memory: Arc<dyn Memory> = Arc::from(
        crate::app::agent::memory::create_memory(&mem_cfg, &workspace_dir, None).unwrap_or_else(
            |_| {
                crate::app::agent::memory::create_memory(
                    &crate::app::agent::config::MemoryConfig {
                        backend: "markdown".to_string(),
                        ..Default::default()
                    },
                    &workspace_dir,
                    None,
                )
                .expect("failed to create fallback memory")
            },
        ),
    );

    let browser_config = config.browser.clone();
    let http_config = config.http_request.clone();
    let web_fetch_config = config.web_fetch.clone();

    let tool_use_context = Arc::new(
        base_context.as_ref().clone().with_root(ctx.root.clone()).with_security(security.clone()),
    );

    let mut tools = all_tools_with_runtime(
        config.clone(),
        &security,
        runtime,
        memory,
        &browser_config,
        &http_config,
        &web_fetch_config,
        &workspace_dir,
        &config.agents,
        None,
        config.as_ref(),
        Some(&ctx.session),
    );

    if !ctx.session_tools.is_empty() {
        let session_tool_names = ctx
            .session_tools
            .tools()
            .iter()
            .map(|tool| tool.name())
            .collect::<std::collections::HashSet<_>>();
        tools.retain(|tool| !session_tool_names.contains(tool.name()));
        let mut scoped_tools = boxed_registry_from_arcs(ctx.session_tools.tools().to_vec());
        scoped_tools.extend(tools);
        tools = scoped_tools;
    }

    (tools, tool_use_context)
}

fn execution_environment_for_context_with_agent_callback(
    ctx: &ToolRuntimeContext,
    callback: super::agent_tool::AgentToolChildSessionCreatedCallback,
) -> (Vec<Box<dyn Tool>>, Arc<ToolUseContext>) {
    let config = runtime_config_for_context(ctx);
    let (mut tools, tool_use_context) = execution_environment_for_config(ctx, config.clone());
    let delegate_agents = enabled_delegate_agents(&config.agents);
    if !delegate_agents.is_empty() {
        tools.retain(|tool| tool.name() != "AgentTool");
        let workspace_dir = tool_use_context.workspace_root().unwrap_or_else(|| {
            ctx.root.as_ref().map(std::path::PathBuf::from).unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            })
        });
        tools.push(Box::new(
            AgentTool::new(delegate_agents, Some(ctx.session.clone()), workspace_dir)
                .with_child_session_created_callback(callback),
        ));
    }
    (tools, tool_use_context)
}

fn security_for_tool_context(
    config: &Config,
    workspace_dir: &std::path::Path,
    base_context: &ToolUseContext,
) -> Arc<SecurityPolicy> {
    let mut security =
        SecurityPolicy::from_configs(&config.autonomy, &config.security, workspace_dir);
    if base_context.full_access_enabled() {
        security.bash_security_enabled = false;
        security.autonomy = AutonomyLevel::Full;
        security.workspace_only = false;
        security.allowed_commands = vec!["*".to_string()];
        security.forbidden_paths.clear();
        security.max_actions_per_hour = u32::MAX;
        security.require_approval_for_medium_risk = false;
        security.block_high_risk_commands = false;
        security.allow_unsafe_shell_patterns = true;
    }
    Arc::new(security)
}

/// 枚举当前上下文下的所有工具规格。
pub fn tool_specs_for_context(ctx: &ToolRuntimeContext) -> Vec<ToolSpec> {
    execution_environment_for_context(ctx).0.into_iter().map(|tool| tool.spec()).collect()
}

/// 执行一次工具调用。
pub fn execute_tool_call(
    requested_name: &str,
    input: &str,
    ctx: &ToolRuntimeContext,
) -> Result<ToolCallResult, ToolCallError> {
    let parsed = parse_tool_input(requested_name, input)?;
    let (tools, tool_use_context) = execution_environment_for_context(ctx);
    let executed = block_on(execute_tool_from_registry(
        &tools,
        requested_name,
        parsed,
        tool_use_context.clone(),
    ))?;

    finish_tool_call(executed)
}

/// 执行 AgentTool，并在真实子会话创建后报告其定位信息。
pub(crate) fn execute_agent_tool_call(
    requested_name: &str,
    input: &str,
    ctx: &ToolRuntimeContext,
    callback: super::agent_tool::AgentToolChildSessionCreatedCallback,
) -> Result<ToolCallResult, ToolCallError> {
    let parsed = parse_tool_input(requested_name, input)?;
    let (tools, tool_use_context) =
        execution_environment_for_context_with_agent_callback(ctx, callback);
    let executed = block_on(execute_tool_from_registry(
        &tools,
        requested_name,
        parsed,
        tool_use_context.clone(),
    ))?;
    finish_tool_call(executed)
}

fn finish_tool_call(executed: ExecutedToolCall) -> Result<ToolCallResult, ToolCallError> {
    // A managed process terminal snapshot is protocol data even when the command
    // failed or was cancelled. Flattening it would discard its handle and stderr.
    let shell_snapshot =
        matches!(executed.tool_name.as_str(), "bash" | "bash_wait" | "bash_cancel")
            && executed.result.data.get("process_id").and_then(Value::as_str).is_some()
            && matches!(
                executed.result.data.get("state").and_then(Value::as_str),
                Some("running" | "exited" | "cancelled" | "timed_out")
            );
    if executed.result.is_success() || shell_snapshot {
        return Ok(executed.result);
    }

    Err(classify_message(
        executed.result.error_text().unwrap_or_else(|| "tool execution failed".to_string()),
    ))
}

/// 在给定注册表中执行工具调用。
pub async fn execute_tool_from_registry(
    tools: &[Box<dyn Tool>],
    requested_name: &str,
    input: Value,
    tool_use_context: Arc<ToolUseContext>,
) -> Result<ExecutedToolCall, ToolCallError> {
    super::executor::execute_tool_from_registry(tools, requested_name, input, tool_use_context)
        .await
}

/// 检测文件是否为二进制内容。
pub fn is_binary(path: &std::path::Path) -> bool {
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "zip" | "tar" | "gz" | "7z" | "exe" | "dll" | "so" | "class" | "jar" | "war" | "doc"
        | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp" | "bin" | "dat"
        | "obj" | "o" | "a" | "lib" | "wasm" | "pyc" | "pyo" => {
            return true;
        }
        _ => {}
    }

    use std::io::Read;

    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };

    let mut bytes = [0u8; 4096];
    let Ok(read_len) = file.read(&mut bytes) else {
        return false;
    };
    if read_len == 0 {
        return false;
    }

    let mut bad = 0usize;
    for byte in &bytes[..read_len] {
        if *byte == 0 {
            return true;
        }
        if *byte < 9 || (*byte > 13 && *byte < 32) {
            bad += 1;
        }
    }

    (bad as f32) / (read_len as f32) > 0.3
}

fn parse_tool_input(tool_id: &str, raw_input: &str) -> Result<Value, ToolCallError> {
    let raw = raw_input.trim();
    if raw.is_empty() {
        return Ok(serde_json::json!({}));
    }

    if raw.starts_with('{') {
        return serde_json::from_str::<Value>(raw)
            .map_err(|error| ToolCallError::Failed(format!("invalid JSON arguments: {error}")));
    }

    if let Some(default_key) = default_scalar_input_key(tool_id) {
        return Ok(serde_json::json!({ default_key: raw }));
    }

    Err(ToolCallError::Failed("invalid arguments: expected JSON object".to_string()))
}

fn default_scalar_input_key(tool_id: &str) -> Option<&'static str> {
    match tool_id {
        "file_read" | "file_write" => Some("path"),
        id if is_web_fetch_tool_id(id) => Some("url"),
        _ => None,
    }
}

fn classify_message(message: String) -> ToolCallError {
    if is_denied_error(&message) {
        ToolCallError::denied(message)
    } else {
        ToolCallError::Failed(message)
    }
}

fn is_denied_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("denied")
        || lower.contains("not allowed")
        || lower.contains("forbidden")
        || lower.contains("blocked")
}

fn block_on<F>(fut: F) -> F::Output
where
    F: std::future::Future + Send,
    F::Output: Send,
{
    #[cfg(target_arch = "wasm32")]
    panic!("block_on not supported on WASM");

    #[cfg(not(target_arch = "wasm32"))]
    {
        if tokio::runtime::Handle::try_current().is_ok() {
            return std::thread::scope(|scope| {
                scope
                    .spawn(move || {
                        tokio::runtime::Builder::new_multi_thread()
                            .worker_threads(1)
                            .enable_all()
                            .build()
                            .expect("failed to build tokio runtime")
                            .block_on(fut)
                    })
                    .join()
                    .expect("tokio bridge thread panicked")
            });
        }

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime")
            .block_on(fut)
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
mod tests;

pub fn default_tools(security: Arc<SecurityPolicy>) -> Vec<Box<dyn Tool>> {
    default_tools_with_runtime(security, Arc::new(NativeRuntime::new()))
}

pub fn default_tools_with_runtime(
    security: Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
) -> Vec<Box<dyn Tool>> {
    let has_shell_access = runtime.has_shell_access();
    let has_filesystem_access = runtime.has_filesystem_access();
    let mut tool_arcs: Vec<Arc<dyn Tool>> = Vec::new();

    #[cfg(not(target_arch = "wasm32"))]
    if has_shell_access {
        tool_arcs.push(Arc::new(ShellTool::new(security.clone(), runtime.clone())));
        tool_arcs.push(Arc::new(BashWaitTool::new(security.clone())));
        tool_arcs.push(Arc::new(BashCancelTool::new(security.clone())));
    }

    #[cfg(not(target_arch = "wasm32"))]
    if has_filesystem_access {
        tool_arcs.push(Arc::new(FileReadTool::new(security.clone())));
        tool_arcs.push(Arc::new(FileEditTool::new(security.clone())));
        tool_arcs.push(Arc::new(FileWriteTool::new(security.clone())));
        tool_arcs.push(Arc::new(ApplyPatchTool::new(security.clone())));
        tool_arcs.push(Arc::new(LsTool::new(security.clone())));
        tool_arcs.push(Arc::new(GlobTool::new(security.clone())));
        tool_arcs.push(Arc::new(GrepTool::new(security.clone())));
    }

    if !tool_arcs.is_empty() {
        let batch_tools = Arc::new(tool_arcs.clone());
        tool_arcs.push(Arc::new(BatchTool::new(batch_tools)));
    }

    boxed_registry_from_arcs(tool_arcs)
}

#[allow(clippy::implicit_hasher, clippy::too_many_arguments)]
pub fn all_tools(
    config: Arc<Config>,
    security: &Arc<SecurityPolicy>,
    memory: Arc<dyn Memory>,
    browser_config: &crate::app::agent::config::BrowserConfig,
    http_config: &crate::app::agent::config::HttpRequestConfig,
    web_fetch_config: &crate::app::agent::config::WebFetchConfig,
    workspace_dir: &std::path::Path,
    agents: &HashMap<String, DelegateAgentConfig>,
    fallback_api_key: Option<&str>,
    root_config: &crate::app::agent::config::Config,
    session_id_override: Option<&str>,
) -> Vec<Box<dyn Tool>> {
    all_tools_with_runtime(
        config,
        security,
        Arc::new(NativeRuntime::new()),
        memory,
        browser_config,
        http_config,
        web_fetch_config,
        workspace_dir,
        agents,
        fallback_api_key,
        root_config,
        session_id_override,
    )
}

#[allow(clippy::implicit_hasher, clippy::too_many_arguments)]
pub fn all_tools_with_runtime(
    config: Arc<Config>,
    security: &Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
    memory: Arc<dyn Memory>,
    browser_config: &crate::app::agent::config::BrowserConfig,
    http_config: &crate::app::agent::config::HttpRequestConfig,
    web_fetch_config: &crate::app::agent::config::WebFetchConfig,
    workspace_dir: &std::path::Path,
    agents: &HashMap<String, DelegateAgentConfig>,
    _fallback_api_key: Option<&str>,
    root_config: &crate::app::agent::config::Config,
    session_id_override: Option<&str>,
) -> Vec<Box<dyn Tool>> {
    let has_shell_access = runtime.has_shell_access();
    let has_filesystem_access = runtime.has_filesystem_access();

    let vibewindow_dir = root_config
        .config_path
        .parent()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| runtime.storage_path());

    let session_id = session_id_override
        .filter(|value| !value.trim().is_empty())
        .or_else(|| workspace_dir.file_name().and_then(|name| name.to_str()))
        .unwrap_or("default-session");
    let tenant = crate::session::ui_store::current_tenant_scope();
    let project_scope = block_on(crate::project::from_directory(&tenant, workspace_dir))
        .ok()
        .map(|(info, _)| info.id);

    let syscall_detector = Arc::new(crate::app::agent::security::SyscallAnomalyDetector::new(
        root_config.security.syscall_anomaly.clone(),
        &vibewindow_dir,
        root_config.security.audit.clone(),
    ));
    let cron_full_access = security.autonomy == AutonomyLevel::Full && !security.workspace_only;
    let mut tool_arcs: Vec<Arc<dyn Tool>> = vec![
        Arc::new(ConfigTool::new(config.clone(), security.clone())),
        Arc::new(
            CronAddTool::new(config.clone(), security.clone())
                .with_execution_defaults(workspace_dir, cron_full_access),
        ),
        Arc::new(CronListTool::new(config.clone())),
        Arc::new(CronRemoveTool::new(config.clone(), security.clone())),
        Arc::new(CronUpdateTool::new(config.clone(), security.clone())),
        Arc::new(CronRunTool::new(config.clone(), security.clone())),
        Arc::new(CronRunsTool::new(config.clone())),
        Arc::new(MemoryStoreTool::new(memory.clone(), security.clone())),
        Arc::new(MemoryRecallTool::new(memory.clone())),
        Arc::new(MemoryForgetTool::new(memory.clone(), security.clone())),
        Arc::new(ScheduleTool::new(security.clone(), root_config.clone())),
    ];

    #[cfg(not(target_arch = "wasm32"))]
    if has_shell_access {
        tool_arcs.push(Arc::new(ShellTool::new_with_syscall_detector(
            security.clone(),
            runtime.clone(),
            Some(syscall_detector.clone()),
        )));
        tool_arcs.push(Arc::new(BashWaitTool::new(security.clone())));
        tool_arcs.push(Arc::new(BashCancelTool::new(security.clone())));
        #[cfg(target_os = "windows")]
        tool_arcs.push(Arc::new(PowerShellTool::new(security.clone())));
        tool_arcs
            .push(Arc::new(GitOperationsTool::new(security.clone(), workspace_dir.to_path_buf())));
    }

    #[cfg(not(target_arch = "wasm32"))]
    if has_filesystem_access {
        tool_arcs.push(Arc::new(FileReadTool::with_session(security.clone(), session_id)));
        tool_arcs.push(Arc::new(FileEditTool::new(security.clone())));
        tool_arcs.push(Arc::new(FileWriteTool::new(security.clone())));
        tool_arcs.push(Arc::new(ApplyPatchTool::new(security.clone())));
        tool_arcs.push(Arc::new(LsTool::new(security.clone())));
        tool_arcs.push(Arc::new(GlobTool::new(security.clone())));
        tool_arcs.push(Arc::new(GrepTool::new(security.clone())));
        tool_arcs.push(Arc::new(EnterWorktreeTool::new()));
        tool_arcs.push(Arc::new(ExitWorktreeTool::new()));
    }

    #[cfg(not(target_arch = "wasm32"))]
    if browser_config.enabled {
        tracing::debug!(
            "all_tools_with_runtime: browser_config.enabled=true, browser_open={}",
            browser_config.browser_open
        );

        if !browser_config.browser_open.eq_ignore_ascii_case("disable") {
            tracing::debug!("all_tools_with_runtime: creating BrowserOpenTool");
            tool_arcs.push(Arc::new(BrowserOpenTool::new(
                security.clone(),
                browser_config.allowed_domains.clone(),
                browser_config.browser_open.clone(),
            )));
        } else {
            tracing::debug!("all_tools_with_runtime: browser_open is disabled");
        }

        tracing::debug!("all_tools_with_runtime: creating BrowserTool");
        tool_arcs.push(Arc::new(BrowserTool::new_with_backend(
            security.clone(),
            browser_config.allowed_domains.clone(),
            browser_config.session_name.clone(),
            browser_config.backend.clone(),
            browser_config.native_headless,
            browser_config.native_webdriver_url.clone(),
            browser_config.native_chrome_path.clone(),
            ComputerUseConfig {
                endpoint: browser_config.computer_use.endpoint.clone(),
                api_key: browser_config.computer_use.api_key.clone(),
                timeout_ms: browser_config.computer_use.timeout_ms,
                allow_remote_endpoint: browser_config.computer_use.allow_remote_endpoint,
                window_allowlist: browser_config.computer_use.window_allowlist.clone(),
                max_coordinate_x: browser_config.computer_use.max_coordinate_x,
                max_coordinate_y: browser_config.computer_use.max_coordinate_y,
            },
        )));
    }

    if http_config.enabled {
        tool_arcs.push(Arc::new(HttpRequestTool::new(
            security.clone(),
            http_config.allowed_domains.clone(),
            http_config.max_response_size,
            http_config.timeout_secs,
            http_config.user_agent.clone(),
        )));
    }

    if web_fetch_config.enabled {
        tool_arcs.push(Arc::new(WebFetchTool::new(
            security.clone(),
            web_fetch_config.provider.clone(),
            web_fetch_config.api_key.clone(),
            web_fetch_config.api_url.clone(),
            web_fetch_config.allowed_domains.clone(),
            web_fetch_config.blocked_domains.clone(),
            web_fetch_config.max_response_size,
            web_fetch_config.timeout_secs,
            web_fetch_config.user_agent.clone(),
        )));
    }

    if root_config.web_search.enabled {
        let provider = root_config.web_search.provider.trim().to_lowercase();
        let api_key = if provider == "brave" {
            root_config
                .web_search
                .brave_api_key
                .clone()
                .or_else(|| root_config.web_search.api_key.clone())
        } else {
            root_config.web_search.api_key.clone()
        };

        if provider == "exa" {
            tool_arcs.push(Arc::new(super::websearch::WebSearchTool::new(
                security.clone(),
                root_config.web_search.provider.clone(),
                api_key,
                root_config.web_search.api_url.clone(),
                root_config.web_search.max_results,
                root_config.web_search.timeout_secs,
                root_config.web_search.user_agent.clone(),
            )));
        } else {
            tool_arcs.push(Arc::new(WebSearchTool::new(
                security.clone(),
                root_config.web_search.provider.clone(),
                api_key,
                root_config.web_search.api_url.clone(),
                root_config.web_search.max_results,
                root_config.web_search.timeout_secs,
                root_config.web_search.user_agent.clone(),
            )));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    tool_arcs.push(Arc::new(PdfReadTool::new(security.clone())));

    #[cfg(not(target_arch = "wasm32"))]
    tool_arcs.push(Arc::new(ScreenshotTool::new(security.clone())));
    tool_arcs.push(Arc::new(ImageAssetReadTool::new()));

    tool_arcs.push(Arc::new(SkillTool::new_with_runtime_config(
        security.clone(),
        session_id.to_string(),
        false,
        workspace_dir.to_path_buf(),
        config.clone(),
    )));
    tool_arcs.push(Arc::new(SleepTool::new()));
    tool_arcs
        .push(Arc::new(TodoReadTool::new_scoped(session_id.to_string(), project_scope.clone())));
    tool_arcs.push(Arc::new(TodoWriteTool::new_scoped(
        session_id.to_string(),
        project_scope,
        security.clone(),
    )));
    tool_arcs.push(Arc::new(QuestionTool::new(session_id.to_string())));
    tool_arcs.push(Arc::new(CreateThreadTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )));
    tool_arcs.push(Arc::new(super::thread_tool::ForkThreadTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )));
    tool_arcs.push(Arc::new(super::thread_tool::HandoffThreadTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )));
    tool_arcs.push(Arc::new(super::thread_tool::ListThreadsTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )));
    tool_arcs.push(Arc::new(super::thread_tool::ReadThreadTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )));
    tool_arcs.push(Arc::new(super::thread_tool::WaitThreadsTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )));
    tool_arcs.push(Arc::new(super::thread_tool::SendMessageToThreadTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )));
    tool_arcs.push(Arc::new(super::thread_tool::SetThreadTitleTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )));
    tool_arcs.push(Arc::new(super::thread_tool::SetThreadArchivedTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )));
    tool_arcs.push(Arc::new(super::thread_tool::SetThreadPinnedTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )));

    let delegate_agents =
        enabled_delegate_agents_with_project_skills(agents, workspace_dir, root_config);
    if !delegate_agents.is_empty() {
        let agent_tool = Arc::new(
            AgentTool::new(
                delegate_agents.clone(),
                session_id_override.map(str::to_owned),
                workspace_dir.to_path_buf(),
            )
            .with_max_parallel_subagents(agent_tool_parallel_limit(agents, "main")),
        );
        tool_arcs.push(agent_tool);
    }

    tool_arcs.push(Arc::new(McpStatusTool::new()));
    register_dynamic_mcp_tools(&mut tool_arcs, workspace_dir);

    let batch_tools = Arc::new(tool_arcs.clone());
    tool_arcs.push(Arc::new(BatchTool::new(batch_tools)));
    boxed_registry_from_arcs(tool_arcs)
}

fn enabled_delegate_agents(
    agents: &HashMap<String, DelegateAgentConfig>,
) -> HashMap<String, DelegateAgentConfig> {
    merged_agent_configs(agents)
        .into_iter()
        .filter(|(_, config)| config.enabled && config.allow_as_subagent)
        .collect()
}

fn enabled_delegate_agents_with_project_skills(
    agents: &HashMap<String, DelegateAgentConfig>,
    workspace_dir: &Path,
    root_config: &Config,
) -> HashMap<String, DelegateAgentConfig> {
    let mut enabled = enabled_delegate_agents(agents);
    if !root_config.skills.auto_load_project_skills || enabled.is_empty() {
        return enabled;
    }

    let project_skill_names =
        crate::app::agent::skills::load_project_skills_with_config(workspace_dir, root_config)
            .into_iter()
            .map(|skill| skill.name)
            .collect::<Vec<_>>();

    for config in enabled.values_mut() {
        config.allowed_skills.extend(project_skill_names.iter().cloned());
        config.allowed_skills.sort();
        config.allowed_skills.dedup();
    }

    enabled
}

fn register_dynamic_mcp_tools(tool_arcs: &mut Vec<Arc<dyn Tool>>, workspace_dir: &Path) {
    let config = match block_on(super::mcp_common::load_resolved_acp_config_from(workspace_dir)) {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!("MCP toolset: failed to load ACP config for dynamic tools: {error}");
            return;
        }
    };

    for server in &config.mcp_servers {
        let server_label = super::mcp_common::server_name(server).to_string();
        let management = super::mcp_common::management_for_server(&config, server);
        if !management.enabled {
            tracing::debug!(
                server = server_label.as_str(),
                "MCP toolset: skipped disabled MCP server"
            );
            continue;
        }
        let existing_ids = tool_arcs.iter().map(|tool| tool.spec().id).collect::<Vec<_>>();
        let existing_refs = existing_ids.iter().map(String::as_str).collect::<Vec<_>>();

        match block_on(discover_dynamic_mcp_tools(
            server.clone(),
            management.clone(),
            &existing_refs,
        )) {
            Ok(tools) => {
                for tool in tools {
                    tracing::debug!(
                        server = server_label.as_str(),
                        tool_id = tool.name(),
                        "MCP toolset: registered dynamic tool"
                    );
                    tool_arcs.push(Arc::new(tool));
                }
            }
            Err(error) => {
                if management.required {
                    tracing::error!(
                        server = server_label.as_str(),
                        "MCP toolset: required server failed dynamic tool discovery: {error}"
                    );
                } else {
                    tracing::warn!(
                        server = server_label.as_str(),
                        "MCP toolset: failed to discover dynamic tools: {error}"
                    );
                }
            }
        }
    }
}

async fn discover_dynamic_mcp_tools(
    server: agent_client_protocol::McpServer,
    management: vw_acp::McpServerManagement,
    existing_tool_ids: &[&str],
) -> anyhow::Result<Vec<super::mcp_tool::McpTool>> {
    let server_label = super::mcp_common::server_name(&server).to_string();
    let existing_tool_ids =
        existing_tool_ids.iter().map(|id| (*id).to_string()).collect::<Vec<_>>();
    let configured_server = server.clone();

    let timeout = super::mcp_common::management_timeout(&management);

    super::mcp_common::with_server_peer_timeout(&server, timeout, move |peer| {
        let server_label = server_label.clone();
        let existing_tool_ids = existing_tool_ids.clone();
        let configured_server = configured_server.clone();
        let management = management.clone();
        Box::pin(async move {
            let tools = peer.list_all_tools().await.context("failed to list MCP tools")?;
            let server_instructions =
                peer.peer_info().and_then(|info| info.instructions.as_deref());
            let mut allowed_tools = Vec::new();
            for tool in tools {
                let tool_id = super::mcp_tool::mcp_tool_id(&server_label, tool.name.as_ref())?;
                if super::mcp_common::mcp_tool_allowed(&management, tool.name.as_ref(), &tool_id) {
                    allowed_tools.push(tool);
                }
            }
            let existing_refs = existing_tool_ids.iter().map(String::as_str).collect::<Vec<_>>();
            let specs = super::mcp_tool::mcp_tools_to_specs(
                &server_label,
                server_instructions,
                &allowed_tools,
                &existing_refs,
            )?;

            Ok(allowed_tools
                .into_iter()
                .zip(specs.into_iter())
                .map(|(tool, spec)| {
                    super::mcp_tool::McpTool::from_configured_server(
                        configured_server.clone(),
                        tool.name.to_string(),
                        spec,
                    )
                })
                .collect())
        })
    })
    .await
}
