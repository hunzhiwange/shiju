pub use tool_assembly::toolset::is_binary;
#[cfg(any(test, target_arch = "wasm32"))]
use super::context::ToolUseContext;
use super::discovery::ToolDiscoveryCatalog;
use super::tool_search::ToolSearchTool;
use super::*;
use crate::app::agent::config::{Config, DelegateAgentConfig};
use crate::app::agent::memory::Memory;
use runtime_types::RuntimeAdapter;
use crate::app::agent::security::SecurityPolicy;
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
pub(crate) use tool_assembly::runtime::mcp_connections_for_context;

pub(crate) use tool_assembly::runtime::execution_tools_for_context;



pub(crate) use tool_assembly::runtime::agent_tool_parallel_limit_for_context;

#[cfg(test)]
fn execution_environment_for_config(
    ctx: &ToolRuntimeContext,
    config: Arc<Config>,
) -> (Vec<Box<dyn Tool>>, Arc<ToolUseContext>) { tool_assembly::runtime::execution_environment_for_config::<super::context::AgentContextHost>(ctx, config) }



pub use tool_assembly::runtime::tool_specs_for_context;

pub use tool_assembly::runtime::execute_tool_call;

pub(crate) use tool_assembly::runtime::execute_bound_app_tool;

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use tool_assembly::runtime::execute_agent_tool_call;

pub use tool_assembly::runtime::execute_tool_from_registry;
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
  if let Some(tool) = tool_assembly::domain_construction::construct_domain_tool::<Self>(&tool, context) { return tool; }
  if let Some(tool) = tool_assembly::domain_construction::construct_thread_tool::<super::agent_hosts::ToolAgentHost>(&tool, context) { return tool; }
  match tool {
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
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::GitOperationsTool => { let security = context.security;
let workspace_dir = context.workspace_dir.expect("workspace");
 Arc::new(GitOperationsTool::new(security.clone(), workspace_dir.to_path_buf())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::EnterWorktreeTool => {
 Arc::new(EnterWorktreeTool::new()) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::ExitWorktreeTool => {
 Arc::new(ExitWorktreeTool::new()) },
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
ToolConstruction::QuestionTool => { let session_id = context.session_id.expect("session");
 Arc::new(QuestionTool::new(session_id.to_string())) },
ToolConstruction::McpStatusTool => {
 Arc::new(McpStatusTool::new()) },
_ => unreachable!("domain construction handled above"),
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
  session.map(|id| crate::plugins::mcp_session_connections(id, workspace)).unwrap_or_else(|| Arc::new(crate::plugins::McpSessionConnections::new(crate::plugins::compat::mcp_connections::host())))
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
 async fn load_project_config(tenant: &vw_api_transport::TenantScope, workspace: &Path) -> anyhow::Result<Config> { Ok(crate::config::get_project_for_tenant(tenant, workspace).await?) }
 fn mcp_connections(session: &str, root: &Path) -> Arc<crate::plugins::McpSessionConnections> { crate::plugins::mcp_session_connections(session, root) }
 fn discovery_state(discovery: &Self::ToolDiscoverySession) -> &tool_assembly::tool_search::ToolDiscoverySession { discovery }
 fn register_app(context: &ToolRuntimeContext, name: &str, result: &mut ToolCallResult) -> anyhow::Result<()> { crate::plugins::mcp_app_execution::register(context, name, result) }
}

#[cfg(test)]
use tool_assembly::agents::{agent_tool_parallel_limit, enabled_delegate_agents};
