//! Context-owned tool assembly and execution orchestration.
use crate::{Tool, ToolSpec, ToolCallResult, ToolCallError, ExecutedToolCall};
use crate::toolset::{boxed_registry_from_arcs, parse_tool_input, finish_tool_call, classify_message};
use crate::discovery::ToolDiscoveryCatalog;
use crate::tool_search::TOOL_SEARCH_ID;
use crate::agents::{agent_tool_parallel_limit, enabled_delegate_agents};
use agent_config::runtime_schema::Config;
use agent_security::SecurityPolicy;
use agent_memory::Memory;
use agent_runtime::NativeRuntime;
use runtime_types::RuntimeAdapter;
use plugin_mcp::mcp_connections::McpSessionConnections;
use tool_context::{context::ToolUseContext, host::ContextHost};
use std::{path::Path, sync::Arc};
use serde::{Serialize, Deserialize};
use serde_json::Value;
pub type SessionToolset = session_port::SessionToolset<tool_catalog::catalog::ToolCatalogSnapshot, McpSessionConnections>;
pub trait RuntimeHost: crate::factory::FactoryHost + ContextHost<SecurityPolicy = SecurityPolicy> {
 fn load_project_config(tenant: &vw_api_transport::TenantScope, workspace: &Path) -> impl std::future::Future<Output = anyhow::Result<Config>> + Send;
 fn mcp_connections(session: &str, root: &Path) -> Arc<McpSessionConnections>;
 fn discovery_state(discovery: &Self::ToolDiscoverySession) -> &tool_catalog::tool_search::ToolDiscoverySession;
 fn register_app(context: &ToolRuntimeContext<Self>, name: &str, result: &mut ToolCallResult) -> anyhow::Result<()>;
}
/// Claude Tools V2 运行时上下文。
///
/// 对外仍保留 `session/root` 这两个轻量字段，避免影响现有调用点；真正的共享
/// 运行时能力则集中放在内部的 `ToolUseContext` 中，以便权限、审批、hook 与
/// read_state 在不同入口之间复用。
#[derive(Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct ToolRuntimeContext<H: RuntimeHost> {
    /// 当前会话标识。
    pub session: String,
    /// 当前工作区根目录。
    pub root: Option<String>,
    /// 厚上下文共享状态。
    #[serde(skip)]
    tool_use_context: Arc<ToolUseContext<H>>,
    /// 只属于当前会话的预发现工具。
    #[serde(skip)]
    session_tools: SessionToolset,
    #[cfg(any(test, feature = "test-support"))]
    #[serde(skip)]
    test_config: Option<Arc<Config>>,
}

impl<H: RuntimeHost> Default for ToolRuntimeContext<H> {
    fn default() -> Self {
        Self::new(
            "default",
            std::env::current_dir().ok().map(|path| path.to_string_lossy().to_string()),
        )
    }
}

impl<H: RuntimeHost> ToolRuntimeContext<H> {
    /// 创建新的运行时上下文。
    pub fn new(session: impl Into<String>, root: Option<String>) -> Self {
        let session = session.into();
        let tool_use_context = ToolUseContext::new(session.clone(), root.clone());
        Self {
            session,
            root,
            tool_use_context: Arc::new(tool_use_context),
            session_tools: SessionToolset::default(),
            #[cfg(any(test, feature = "test-support"))]
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
    pub fn with_tool_use_context(mut self, tool_use_context: ToolUseContext<H>) -> Self {
        self.tool_use_context = Arc::new(tool_use_context.with_root(self.root.clone()));
        self
    }

    /// 获取当前共享运行时上下文。
    pub fn tool_use_context(&self) -> Arc<ToolUseContext<H>> {
        self.tool_use_context.clone()
    }

    /// 注入只属于当前会话的工具集合。
    pub fn with_session_tools(mut self, session_tools: SessionToolset) -> Self {
        self.session_tools = session_tools;
        self
    }

    /// Isolate config storage while exercising the real registry and permission pipeline.
    #[cfg(any(test, feature = "test-support"))]
    pub fn with_test_config(mut self, config: Config) -> Self {
        self.test_config = Some(Arc::new(config));
        self
    }

    /// Reuse native configuration, full-access inheritance and delegated restrictions
    /// for file-only entry points without constructing the executable tool registry.
    pub fn path_security_policy(&self) -> Arc<SecurityPolicy> {
        let config = runtime_config_for_context(self);
        let base = self.tool_use_context();
        let workspace = base.workspace_root().unwrap_or_else(|| config.workspace_dir.clone());
        security_for_tool_context(&config, &workspace, &base)
    }

    /// 返回当前会话的工具集合。
    pub fn session_tools(&self) -> &SessionToolset {
        &self.session_tools
    }
}

/// Resolve the session-owned refresh source; ACP leases remain isolated from native chats.
pub fn mcp_connections_for_context<H: RuntimeHost>(
    ctx: &ToolRuntimeContext<H>,
) -> Arc<McpSessionConnections> {
    ctx.session_tools.mcp_connections().unwrap_or_else(|| {
        let root = ctx
            .tool_use_context()
            .workspace_root()
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        H::mcp_connections(&ctx.session, &root)
    })
}





/// 根据当前上下文构建可执行工具集。
pub fn execution_tools_for_context<H: RuntimeHost>(ctx: &ToolRuntimeContext<H>) -> Vec<Box<dyn Tool>> {
    execution_environment_for_context(ctx).0
}

pub fn execution_environment_for_context<H: RuntimeHost>(
    ctx: &ToolRuntimeContext<H>,
) -> (Vec<Box<dyn Tool>>, Arc<ToolUseContext<H>>) {
    let config = runtime_config_for_context(ctx);
    execution_environment_for_config(ctx, config)
}

pub fn runtime_config_for_context<H: RuntimeHost>(ctx: &ToolRuntimeContext<H>) -> Arc<Config> {
    #[cfg(any(test, feature = "test-support"))]
    if let Some(config) = &ctx.test_config {
        return config.clone();
    }
    let workspace_dir = ctx.tool_use_context().workspace_root().unwrap_or_else(|| {
        ctx.root.as_ref().map(std::path::PathBuf::from).unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        })
    });

    let config = H::block_on(async {
        match H::load_project_config(&ctx.tool_use_context().tenant_scope(), &workspace_dir).await {
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

pub fn agent_tool_parallel_limit_for_context<H: RuntimeHost>(
    ctx: &ToolRuntimeContext<H>,
    agent_name: &str,
) -> usize {
    let config = runtime_config_for_context(ctx);
    agent_tool_parallel_limit(&config.agents, agent_name)
}

pub fn execution_environment_for_config<H: RuntimeHost>(
    ctx: &ToolRuntimeContext<H>,
    config: Arc<Config>,
) -> (Vec<Box<dyn Tool>>, Arc<ToolUseContext<H>>) {
    tool_context::context::scope_tool_use_context_sync(ctx.tool_use_context().clone(), || {
        execution_environment_for_config_inner(ctx, config)
    })
}

pub fn execution_environment_for_config_inner<H: RuntimeHost>(
    ctx: &ToolRuntimeContext<H>,
    config: Arc<Config>,
) -> (Vec<Box<dyn Tool>>, Arc<ToolUseContext<H>>) {
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
        agent_memory::create_memory(&mem_cfg, &workspace_dir, None).unwrap_or_else(
            |_| {
                agent_memory::create_memory(
                    &agent_config::runtime_schema::MemoryConfig {
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

    let mut tools = crate::factory::all_tools_with_runtime::<H>(
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

    // Session tools may replace registry entries, but cannot supply a forged
    // search implementation or widen the caller's authorized catalog.
    tools.retain(|tool| tool.name() != TOOL_SEARCH_ID);
    if let Some(discovery) = tool_use_context.tool_discovery() {
        tools.retain(|tool| H::discovery_state(&discovery).authorizes(&tool.spec().id));
        if H::discovery_state(&discovery).authorizes(TOOL_SEARCH_ID) {
            // The description comes from the authorized snapshot, including
            // deferred tool namespaces rather than the currently exposed subset.
            tools.push(boxed_registry_from_arcs(vec![H::search_tool(&ToolDiscoveryCatalog::new(H::discovery_state(&discovery).authorized_specs()))]).remove(0));
        }
    } else {
        let catalog = ToolDiscoveryCatalog::new(tools.iter().map(|tool| tool.spec()).collect());
        if catalog.has_deferred_tools() {
            tools.push(boxed_registry_from_arcs(vec![H::search_tool(&catalog)]).remove(0));
        }
    }

    (tools, tool_use_context)
}

pub fn execution_environment_for_context_with_agent_callback<H: RuntimeHost>(
    ctx: &ToolRuntimeContext<H>,
    callback: tool_agents::agent_tool::AgentToolChildSessionCreatedCallback,
) -> (Vec<Box<dyn Tool>>, Arc<ToolUseContext<H>>) {
    let config = runtime_config_for_context(ctx);
    let (mut tools, tool_use_context) = execution_environment_for_config(ctx, config.clone());
    let delegate_agents = enabled_delegate_agents(&config.agents);
    if !delegate_agents.is_empty()
        && tool_use_context
            .tool_discovery()
            .is_none_or(|discovery| H::discovery_state(&discovery).authorizes("AgentTool"))
    {
        tools.retain(|tool| tool.name() != "AgentTool");
        let workspace_dir = tool_use_context.workspace_root().unwrap_or_else(|| {
            ctx.root.as_ref().map(std::path::PathBuf::from).unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            })
        });
        tools.extend(boxed_registry_from_arcs(vec![H::agent_tool(delegate_agents, Some(ctx.session.clone()), workspace_dir, Some(callback))]));
    }
    (tools, tool_use_context)
}

use crate::security::security_for_tool_context;

/// 枚举当前上下文下的所有工具规格。
pub fn tool_specs_for_context<H: RuntimeHost>(ctx: &ToolRuntimeContext<H>) -> Vec<ToolSpec> {
    if let Some(discovery) = ctx.tool_use_context().tool_discovery() {
        return H::discovery_state(&discovery).exposed_specs();
    }
    execution_environment_for_context(ctx).0.into_iter().map(|tool| tool.spec()).collect()
}

/// 执行一次工具调用。
pub fn execute_tool_call<H: RuntimeHost>(
    requested_name: &str,
    input: &str,
    ctx: &ToolRuntimeContext<H>,
) -> Result<ToolCallResult, ToolCallError> {
    let parsed = parse_tool_input(requested_name, input)?;
    let (tools, tool_use_context) = execution_environment_for_context(ctx);
    let executed = H::block_on(execute_tool_from_registry(
        &tools,
        requested_name,
        parsed,
        tool_use_context.clone(),
    ))?;
    let mut result = finish_tool_call(executed)?;
    if result.data.get("mcp_app").is_none() {
        return Ok(result);
    }
    let tools = tools.into_iter().map(Arc::from).collect();
    let app_context = ctx
        .clone()
        .with_tool_use_context((*tool_use_context).clone())
        .with_session_tools(SessionToolset::new(tools));
    H::register_app(&app_context, requested_name, &mut result)
        .map_err(|error| classify_message(error.to_string()))?;
    Ok(result)
}

pub fn execute_bound_app_tool<H: RuntimeHost>(
    requested_name: &str,
    input: &str,
    ctx: &ToolRuntimeContext<H>,
) -> Result<ToolCallResult, ToolCallError> {
    let parsed = parse_tool_input(requested_name, input)?;
    let (mut tools, tool_use_context) = execution_environment_for_context(ctx);
    tools.retain(|tool| tool.name() == requested_name);
    let executed =
        H::block_on(execute_tool_from_registry(&tools, requested_name, parsed, tool_use_context))?;
    finish_tool_call(executed)
}

/// 执行 AgentTool，并在真实子会话创建后报告其定位信息。
pub fn execute_agent_tool_call<H: RuntimeHost>(
    requested_name: &str,
    input: &str,
    ctx: &ToolRuntimeContext<H>,
    callback: tool_agents::agent_tool::AgentToolChildSessionCreatedCallback,
) -> Result<ToolCallResult, ToolCallError> {
    let parsed = parse_tool_input(requested_name, input)?;
    let (tools, tool_use_context) =
        execution_environment_for_context_with_agent_callback(ctx, callback);
    let executed = H::block_on(execute_tool_from_registry(
        &tools,
        requested_name,
        parsed,
        tool_use_context.clone(),
    ))?;
    finish_tool_call(executed)
}

/// 在给定注册表中执行工具调用。
pub async fn execute_tool_from_registry<H: RuntimeHost>(
    tools: &[Box<dyn Tool>],
    requested_name: &str,
    input: Value,
    tool_use_context: Arc<ToolUseContext<H>>,
) -> Result<ExecutedToolCall, ToolCallError> {
    tool_context::executor::execute_tool_from_registry(tools, requested_name, input, tool_use_context)
        .await
}


impl<H: RuntimeHost> std::fmt::Debug for ToolRuntimeContext<H> {
 fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
  f.debug_struct("ToolRuntimeContext").field("session", &self.session).field("root", &self.root).field("tool_use_context", &self.tool_use_context).field("session_tools", &self.session_tools).finish()
 }
}
