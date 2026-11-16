pub use tool_assembly::toolset::is_binary;
use super::context::ToolUseContext;
use super::discovery::ToolDiscoveryCatalog;
use super::tool_search::ToolSearchTool;
use super::*;
use crate::app::agent::config::{Config, DelegateAgentConfig};
use crate::app::agent::memory::Memory;
use runtime_types::RuntimeAdapter;
use crate::app::agent::security::SecurityPolicy;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

impl tool_git::GitToolHost for super::context::AgentContextHost {
    #[cfg(not(target_arch = "wasm32"))]
    fn initialize_worktree_host() {
        crate::worktree::initialize_host();
    }
}

pub type ToolRuntimeContext = tool_assembly::runtime::ToolRuntimeContext<super::context::AgentContextHost>;
pub use tool_assembly::toolset::boxed_registry_from_arcs;
pub use super::executor::{ExecutedToolCall, ToolCallError};
pub(crate) fn mcp_connections_for_context(
    ctx: &ToolRuntimeContext,
) -> Arc<crate::plugins::McpSessionConnections> { tool_assembly::runtime::mcp_connections_for_context::<super::context::AgentContextHost>(ctx) }

pub(crate) fn execution_tools_for_context(ctx: &ToolRuntimeContext) -> Vec<Box<dyn Tool>> { tool_assembly::runtime::execution_tools_for_context::<super::context::AgentContextHost>(ctx) }



pub(crate) fn agent_tool_parallel_limit_for_context(
    ctx: &ToolRuntimeContext,
    agent_name: &str,
) -> usize { tool_assembly::runtime::agent_tool_parallel_limit_for_context::<super::context::AgentContextHost>(ctx, agent_name) }

#[cfg(test)]
fn execution_environment_for_config(
    ctx: &ToolRuntimeContext,
    config: Arc<Config>,
) -> (Vec<Box<dyn Tool>>, Arc<ToolUseContext>) { tool_assembly::runtime::execution_environment_for_config::<super::context::AgentContextHost>(ctx, config) }



pub fn tool_specs_for_context(ctx: &ToolRuntimeContext) -> Vec<ToolSpec> { tool_assembly::runtime::tool_specs_for_context::<super::context::AgentContextHost>(ctx) }

pub fn execute_tool_call(
    requested_name: &str,
    input: &str,
    ctx: &ToolRuntimeContext,
) -> Result<ToolCallResult, ToolCallError> { tool_assembly::runtime::execute_tool_call::<super::context::AgentContextHost>(requested_name, input, ctx) }

pub(crate) fn execute_bound_app_tool(
    requested_name: &str,
    input: &str,
    ctx: &ToolRuntimeContext,
) -> Result<ToolCallResult, ToolCallError> { tool_assembly::runtime::execute_bound_app_tool::<super::context::AgentContextHost>(requested_name, input, ctx) }

pub(crate) fn execute_agent_tool_call(
    requested_name: &str,
    input: &str,
    ctx: &ToolRuntimeContext,
    callback: super::agent_tool::AgentToolChildSessionCreatedCallback,
) -> Result<ToolCallResult, ToolCallError> { tool_assembly::runtime::execute_agent_tool_call::<super::context::AgentContextHost>(requested_name, input, ctx, callback) }

pub async fn execute_tool_from_registry(
    tools: &[Box<dyn Tool>],
    requested_name: &str,
    input: Value,
    tool_use_context: Arc<ToolUseContext>,
) -> Result<ExecutedToolCall, ToolCallError> { tool_assembly::runtime::execute_tool_from_registry::<super::context::AgentContextHost>(tools, requested_name, input, tool_use_context).await }
fn block_on<F>(fut: F) -> F::Output
where
    F: std::future::Future + Send,
    F::Output: Send,
{
    #[cfg(target_arch = "wasm32")]
    panic!("block_on not supported on WASM");

    #[cfg(not(target_arch = "wasm32"))]
    {
        let context = super::current_tool_use_context();
        let tenant = crate::session::ui_store::current_tenant_scope();
        let fut = crate::session::ui_store::with_tenant_scope(tenant, async move {
            match context {
                Some(context) => super::scope_tool_use_context(context, fut).await,
                None => fut.await,
            }
        });
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

use tool_assembly::factory::ToolConstruction;
use tool_assembly::factory::{ConstructionContext, FactoryHost};
pub(crate) use tool_assembly::mcp::DiscoveredMcpTools;
pub(crate) use tool_assembly::mcp::discover_dynamic_mcp_tools as discover_dynamic_mcp_tools_for_host;
pub(crate) async fn discover_dynamic_mcp_tools(
    server: agent_client_protocol::McpServer,
    management: acp_config::McpServerManagement,
    existing_tool_ids: &[&str],
    connections: Arc<crate::plugins::McpSessionConnections>,
) -> anyhow::Result<DiscoveredMcpTools<super::context::AgentContextHost>> {
    discover_dynamic_mcp_tools_for_host::<super::context::AgentContextHost>(
        server, management, existing_tool_ids, connections,
    ).await
}

pub fn default_tools(security: Arc<SecurityPolicy>) -> Vec<Box<dyn Tool>> { tool_assembly::factory::default_tools::<super::context::AgentContextHost>(security) }

pub fn default_tools_with_runtime(
    security: Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
) -> Vec<Box<dyn Tool>> { tool_assembly::factory::default_tools_with_runtime::<super::context::AgentContextHost>(security, runtime) }

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
) -> Vec<Box<dyn Tool>> { tool_assembly::factory::all_tools::<super::context::AgentContextHost>(config, security, memory, browser_config, http_config, web_fetch_config, workspace_dir, agents, fallback_api_key, root_config, session_id_override) }

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
) -> Vec<Box<dyn Tool>> { tool_assembly::factory::all_tools_with_runtime::<super::context::AgentContextHost>(config, security, runtime, memory, browser_config, http_config, web_fetch_config, workspace_dir, agents, _fallback_api_key, root_config, session_id_override) }

impl FactoryHost for super::context::AgentContextHost {
 fn block_on<F: std::future::Future + Send>(future: F) -> F::Output where F::Output: Send { block_on(future) }
 fn construct(tool: ToolConstruction, context: &ConstructionContext<'_>) -> Arc<dyn Tool> {
  match tool {
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::ShellTool => { let security = context.security;
let runtime = context.runtime;
 Arc::new(ShellTool::new(security.clone(), runtime.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::BashWaitTool => { let security = context.security;
 Arc::new(BashWaitTool::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::BashCancelTool => { let security = context.security;
 Arc::new(BashCancelTool::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::FileReadTool => { let security = context.security;
 Arc::new(FileReadTool::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::FileEditTool => { let security = context.security;
 Arc::new(FileEditTool::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::FileWriteTool => { let security = context.security;
 Arc::new(FileWriteTool::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::ApplyPatchTool => { let security = context.security;
 Arc::new(ApplyPatchTool::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::LsTool => { let security = context.security;
 Arc::new(LsTool::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::GlobTool => { let security = context.security;
 Arc::new(GlobTool::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::GrepTool => { let security = context.security;
 Arc::new(GrepTool::new(security.clone())) },
ToolConstruction::ConfigTool => { let security = context.security;
let config = context.config.expect("full factory config");
 Arc::new(ConfigTool::new(config.clone(), security.clone())) },
ToolConstruction::CronAddTool => { let cron_full_access = context.cron_full_access;
let security = context.security;
let config = context.config.expect("full factory config");
let workspace_dir = context.workspace_dir.expect("workspace");
 Arc::new(
            CronAddTool::new(config.clone(), security.clone())
                .with_execution_defaults(workspace_dir, cron_full_access),
        ) },
ToolConstruction::CronListTool => { let config = context.config.expect("full factory config");
 Arc::new(CronListTool::new(config.clone())) },
ToolConstruction::CronRemoveTool => { let security = context.security;
let config = context.config.expect("full factory config");
 Arc::new(CronRemoveTool::new(config.clone(), security.clone())) },
ToolConstruction::CronUpdateTool => { let security = context.security;
let config = context.config.expect("full factory config");
 Arc::new(CronUpdateTool::new(config.clone(), security.clone())) },
ToolConstruction::CronRunTool => { let security = context.security;
let config = context.config.expect("full factory config");
 Arc::new(CronRunTool::new(
            config.clone(),
            security.clone(),
            super::cron_run::execute_job_now,
        )) },
ToolConstruction::CronRunsTool => { let config = context.config.expect("full factory config");
 Arc::new(CronRunsTool::new(config.clone())) },
ToolConstruction::MemoryStoreTool => { let security = context.security;
let memory = context.memory.expect("full factory memory");
 Arc::new(tool_memory::MemoryStoreTool::new(memory.clone(), security.clone())) },
ToolConstruction::MemoryRecallTool => { let memory = context.memory.expect("full factory memory");
 Arc::new(tool_memory::MemoryRecallTool::new(memory.clone())) },
ToolConstruction::MemoryForgetTool => { let security = context.security;
let memory = context.memory.expect("full factory memory");
 Arc::new(tool_memory::MemoryForgetTool::new(memory.clone(), security.clone())) },
ToolConstruction::ScheduleTool => { let security = context.security;
let root_config = context.root_config.expect("root config");
 Arc::new(ScheduleTool::new(security.clone(), root_config.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::ShellToolWithSyscallDetector => { let security = context.security;
let runtime = context.runtime;
let syscall_detector = context.syscall_detector.expect("syscall detector");
 Arc::new(ShellTool::new_with_syscall_detector(
            security.clone(),
            runtime.clone(),
            Some(syscall_detector.clone()),
        )) },
#[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
ToolConstruction::PowerShellTool => { let security = context.security;
 Arc::new(PowerShellTool::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::GitOperationsTool => { let security = context.security;
let workspace_dir = context.workspace_dir.expect("workspace");
 Arc::new(GitOperationsTool::new(security.clone(), workspace_dir.to_path_buf())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::FileReadToolWithSession => { let security = context.security;
let session_id = context.session_id.expect("session");
 Arc::new(FileReadTool::with_session(security.clone(), session_id)) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::EnterWorktreeTool => {
 Arc::new(EnterWorktreeTool::new()) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::ExitWorktreeTool => {
 Arc::new(ExitWorktreeTool::new()) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::BrowserOpenTool => { let security = context.security;
let browser_config = context.browser_config.expect("browser config");
 Arc::new(BrowserOpenTool::new(
                security.clone(),
                browser_config.allowed_domains.clone(),
                browser_config.browser_open.clone(),
            )) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::BrowserToolWithBackend => {
let security = context.security;
let browser_config = context.browser_config.expect("browser config");
 Arc::new(BrowserTool::new_with_backend(
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
        )) },
ToolConstruction::HttpRequestTool => { let http_config = context.http_config.expect("http config");
let security = context.security;
 Arc::new(HttpRequestTool::new(
            security.clone(),
            http_config.allowed_domains.clone(),
            http_config.max_response_size,
            http_config.timeout_secs,
            http_config.user_agent.clone(),
        )) },
ToolConstruction::WebFetchTool => { let web_fetch_config = context.web_fetch_config.expect("web fetch config");
let security = context.security;
 Arc::new(WebFetchTool::new(
            security.clone(),
            web_fetch_config.provider.clone(),
            web_fetch_config.api_key.clone(),
            web_fetch_config.api_url.clone(),
            web_fetch_config.allowed_domains.clone(),
            web_fetch_config.blocked_domains.clone(),
            web_fetch_config.max_response_size,
            web_fetch_config.timeout_secs,
            web_fetch_config.user_agent.clone(),
        )) },
ToolConstruction::ExaWebSearchTool => { let api_key = context.api_key.clone();
let security = context.security;
let root_config = context.root_config.expect("root config");
 Arc::new(super::websearch::WebSearchTool::new(
                security.clone(),
                root_config.web_search.provider.clone(),
                api_key,
                root_config.web_search.api_url.clone(),
                root_config.web_search.max_results,
                root_config.web_search.timeout_secs,
                root_config.web_search.user_agent.clone(),
            )) },
ToolConstruction::WebSearchTool => { let api_key = context.api_key.clone();
let security = context.security;
let root_config = context.root_config.expect("root config");
 Arc::new(WebSearchTool::new(
                security.clone(),
                root_config.web_search.provider.clone(),
                api_key,
                root_config.web_search.api_url.clone(),
                root_config.web_search.max_results,
                root_config.web_search.timeout_secs,
                root_config.web_search.user_agent.clone(),
            )) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::PdfReadTool => { let security = context.security;
 Arc::new(PdfReadTool::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::ScreenshotTool => { let security = context.security;
 Arc::new(ScreenshotTool::new(security.clone())) },
ToolConstruction::ImageAssetReadTool => {
 Arc::new(ImageAssetReadTool::new()) },
ToolConstruction::SkillToolWithRuntimeConfig => { let security = context.security;
let config = context.config.expect("full factory config");
let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(SkillTool::new_with_runtime_config(
        security.clone(),
        session_id.to_string(),
        false,
        workspace_dir.to_path_buf(),
        config.clone(),
    )) },
ToolConstruction::SleepTool => {
 Arc::new(SleepTool::new()) },
ToolConstruction::TodoReadToolScoped => { let session_id = context.session_id.expect("session");
let project_scope = context.project_scope.clone();
 Arc::new(TodoReadTool::new_scoped(session_id.to_string(), project_scope.clone())) },
ToolConstruction::TodoWriteToolScoped => { let security = context.security;
let session_id = context.session_id.expect("session");
let project_scope = context.project_scope.clone();
 Arc::new(TodoWriteTool::new_scoped(
        session_id.to_string(),
        project_scope,
        security.clone(),
    )) },
ToolConstruction::QuestionTool => { let session_id = context.session_id.expect("session");
 Arc::new(QuestionTool::new(session_id.to_string())) },
ToolConstruction::CreateThreadTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(CreateThreadTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::ForkThreadTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(super::thread_tool::ForkThreadTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::HandoffThreadTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(super::thread_tool::HandoffThreadTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::ListThreadsTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(super::thread_tool::ListThreadsTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::ReadThreadTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(super::thread_tool::ReadThreadTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::WaitThreadsTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(super::thread_tool::WaitThreadsTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::SendMessageToThreadTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(super::thread_tool::SendMessageToThreadTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::SetThreadTitleTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(super::thread_tool::SetThreadTitleTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::SetThreadArchivedTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(super::thread_tool::SetThreadArchivedTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::SetThreadPinnedTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(super::thread_tool::SetThreadPinnedTool::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::McpStatusTool => {
 Arc::new(McpStatusTool::new()) },
  }
 }
 fn batch(tools: Arc<Vec<Arc<dyn Tool>>>) -> Arc<dyn Tool> { Arc::new(BatchTool::new(tools)) }
 fn search_tool(catalog: &ToolDiscoveryCatalog) -> Arc<dyn Tool> { Arc::new(ToolSearchTool::new(catalog)) }
 fn agent_tool(agents: HashMap<String, DelegateAgentConfig>, session: Option<String>, workspace: std::path::PathBuf, callback: Option<super::agent_tool::AgentToolChildSessionCreatedCallback>) -> Arc<dyn Tool> {
  let tool = AgentTool::new(agents, session, workspace);
  Arc::new(match callback { Some(callback) => tool.with_child_session_created_callback(callback), None => tool })
 }
 fn project_scope(workspace: &Path) -> Option<String> {
  let tenant = crate::session::ui_store::current_tenant_scope();
  block_on(crate::project::from_directory(&tenant, workspace)).ok().map(|(info, _)| info.id)
 }
 fn project_skills(workspace: &Path, config: &Config) -> Vec<String> {
  crate::app::agent::skills::load_project_skills_with_config(workspace, config).into_iter().map(|skill| skill.name).collect()
 }
 fn connections_for_factory(workspace: &Path, session: Option<&str>) -> Arc<crate::plugins::McpSessionConnections> {
  session.map(|id| crate::plugins::mcp_session_connections(id, workspace)).unwrap_or_default()
 }
 fn load_runtime_mcp_config(workspace: &Path) -> anyhow::Result<acp_config::ResolvedAcpxConfig> {
  block_on(super::mcp_common::load_resolved_runtime_mcp_config_from(workspace))
 }
 #[cfg(unix)]
 fn native_tools(session: Option<&str>) -> Vec<Arc<dyn Tool>> {
  let Some(host) = super::computer_control::NativeControlRuntime::shared() else { return Vec::new() };
  vec![Arc::new(super::computer_control::ComputerControlTool::new(host.clone(), session.map(str::to_owned))), Arc::new(tool_office::WordControlTool::<Self>::new(host.clone(), session.map(str::to_owned))), Arc::new(tool_office::ExcelControlTool::<Self>::new(host, session.map(str::to_owned)))]
 }
}

impl tool_assembly::runtime::RuntimeHost for super::context::AgentContextHost {
 async fn load_project_config(tenant: &vw_api_transport::TenantScope, workspace: &Path) -> anyhow::Result<Config> { crate::config::get_project_for_tenant(tenant, workspace).await }
 fn mcp_connections(session: &str, root: &Path) -> Arc<crate::plugins::McpSessionConnections> { crate::plugins::mcp_session_connections(session, root) }
 fn discovery_state(discovery: &Self::ToolDiscoverySession) -> &tool_assembly::tool_search::ToolDiscoverySession { discovery }
 fn register_app(context: &ToolRuntimeContext, name: &str, result: &mut ToolCallResult) -> anyhow::Result<()> { crate::plugins::mcp_app_execution::register(context, name, result) }
}

#[cfg(test)]
use tool_assembly::toolset::parse_tool_input;
#[cfg(test)]
use tool_assembly::agents::{agent_tool_parallel_limit, enabled_delegate_agents};
#[cfg(test)]
fn enabled_delegate_agents_with_project_skills(agents: &HashMap<String, DelegateAgentConfig>, workspace: &Path, config: &Config) -> HashMap<String, DelegateAgentConfig> {
 tool_assembly::factory::enabled_delegate_agents_with_project_skills::<super::context::AgentContextHost>(agents, workspace, config)
}

#[cfg(test)]
use super::tool_search::TOOL_SEARCH_ID;
