//! Tool selection and ordered factory assembly. The host only constructs concrete tools.
use crate::Tool;
use crate::toolset::boxed_registry_from_arcs;
use crate::discovery::ToolDiscoveryCatalog;
use agent_config::runtime_schema::{Config, DelegateAgentConfig, BrowserConfig, HttpRequestConfig, WebFetchConfig};
use agent_security::{SecurityPolicy, AutonomyLevel};
use agent_runtime::NativeRuntime;
use agent_memory::Memory;
use runtime_types::RuntimeAdapter;
use std::{collections::HashMap, path::{Path, PathBuf}, sync::Arc};
use tool_agents::agent_tool::AgentToolChildSessionCreatedCallback;

pub enum ToolConstruction {
#[cfg(not(target_arch = "wasm32"))]
ShellTool,
#[cfg(not(target_arch = "wasm32"))]
BashWaitTool,
#[cfg(not(target_arch = "wasm32"))]
BashCancelTool,
#[cfg(not(target_arch = "wasm32"))]
FileReadTool,
#[cfg(not(target_arch = "wasm32"))]
FileEditTool,
#[cfg(not(target_arch = "wasm32"))]
FileWriteTool,
#[cfg(not(target_arch = "wasm32"))]
ApplyPatchTool,
#[cfg(not(target_arch = "wasm32"))]
LsTool,
#[cfg(not(target_arch = "wasm32"))]
GlobTool,
#[cfg(not(target_arch = "wasm32"))]
GrepTool,
ConfigTool,
CronAddTool,
CronListTool,
CronRemoveTool,
CronUpdateTool,
CronRunTool,
CronRunsTool,
MemoryStoreTool,
MemoryRecallTool,
MemoryForgetTool,
ScheduleTool,
#[cfg(not(target_arch = "wasm32"))]
ShellToolWithSyscallDetector,
#[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
PowerShellTool,
#[cfg(not(target_arch = "wasm32"))]
GitOperationsTool,
#[cfg(not(target_arch = "wasm32"))]
FileReadToolWithSession,
#[cfg(not(target_arch = "wasm32"))]
EnterWorktreeTool,
#[cfg(not(target_arch = "wasm32"))]
ExitWorktreeTool,
#[cfg(not(target_arch = "wasm32"))]
BrowserOpenTool,
#[cfg(not(target_arch = "wasm32"))]
BrowserToolWithBackend,
HttpRequestTool,
WebFetchTool,
ExaWebSearchTool,
WebSearchTool,
#[cfg(not(target_arch = "wasm32"))]
PdfReadTool,
#[cfg(not(target_arch = "wasm32"))]
ScreenshotTool,
ImageAssetReadTool,
SkillToolWithRuntimeConfig,
SleepTool,
TodoReadToolScoped,
TodoWriteToolScoped,
QuestionTool,
CreateThreadTool,
ForkThreadTool,
HandoffThreadTool,
ListThreadsTool,
ReadThreadTool,
WaitThreadsTool,
SendMessageToThreadTool,
SetThreadTitleTool,
SetThreadArchivedTool,
SetThreadPinnedTool,
McpStatusTool,
}
#[derive(Clone)]
pub struct ConstructionContext<'a> {
 pub root_config: Option<&'a Config>,
 pub http_config: Option<&'a HttpRequestConfig>,
 pub web_fetch_config: Option<&'a WebFetchConfig>,
 pub api_key: Option<String>,
 pub cron_full_access: bool,
 pub config: Option<&'a Arc<Config>>,
 pub security: &'a Arc<SecurityPolicy>,
 pub runtime: &'a Arc<dyn RuntimeAdapter>,
 pub memory: Option<&'a Arc<dyn Memory>>,
 pub browser_config: Option<&'a BrowserConfig>,
 pub workspace_dir: Option<&'a Path>,
 pub session_id: Option<&'a str>,
 pub project_scope: Option<String>,
 pub syscall_detector: Option<&'a Arc<syscall_anomaly::SyscallAnomalyDetector>>,
}
pub trait FactoryHost: tool_mcp::McpToolHost<Connections = plugin_mcp::mcp_connections::McpSessionConnections> {
 fn block_on<F: std::future::Future + Send>(future: F) -> F::Output where F::Output: Send;
 fn construct(tool: ToolConstruction, context: &ConstructionContext<'_>) -> Arc<dyn Tool>;
 fn batch(tools: Arc<Vec<Arc<dyn Tool>>>) -> Arc<dyn Tool>;
 fn search_tool(catalog: &ToolDiscoveryCatalog) -> Arc<dyn Tool>;
 fn agent_tool(agents: HashMap<String, DelegateAgentConfig>, session: Option<String>, workspace: PathBuf, callback: Option<AgentToolChildSessionCreatedCallback>) -> Arc<dyn Tool>;
 fn project_scope(workspace: &Path) -> Option<String>;
 fn project_skills(workspace: &Path, config: &Config) -> Vec<String>;
 fn connections_for_factory(workspace: &Path, session: Option<&str>) -> Arc<plugin_mcp::mcp_connections::McpSessionConnections>;
 fn load_runtime_mcp_config(workspace: &Path) -> anyhow::Result<acp_config::ResolvedAcpxConfig>;
 #[cfg(unix)]
 fn native_tools(session: Option<&str>) -> Vec<Arc<dyn Tool>>;
}
use crate::agents::enabled_delegate_agents;
pub fn default_tools<H: FactoryHost>(security: Arc<SecurityPolicy>) -> Vec<Box<dyn Tool>> {
    default_tools_with_runtime::<H>(security, Arc::new(NativeRuntime::new()))
}

pub fn default_tools_with_runtime<H: FactoryHost>(
    security: Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
) -> Vec<Box<dyn Tool>> {
    let has_shell_access = runtime.has_shell_access();
    let has_filesystem_access = runtime.has_filesystem_access();
    let construction = ConstructionContext { root_config: None, http_config: None, web_fetch_config: None, api_key: None, cron_full_access: false, config: None, security: &security, runtime: &runtime, memory: None, browser_config: None, workspace_dir: None, session_id: None, project_scope: None, syscall_detector: None };
    let mut tool_arcs: Vec<Arc<dyn Tool>> = Vec::new();

    #[cfg(not(target_arch = "wasm32"))]
    if has_shell_access {
        tool_arcs.push(H::construct(ToolConstruction::ShellTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::BashWaitTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::BashCancelTool, &construction));
    }

    #[cfg(not(target_arch = "wasm32"))]
    if has_filesystem_access {
        tool_arcs.push(H::construct(ToolConstruction::FileReadTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::FileEditTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::FileWriteTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::ApplyPatchTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::LsTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::GlobTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::GrepTool, &construction));
    }

    if !tool_arcs.is_empty() {
        let batch_tools = Arc::new(tool_arcs.clone());
        tool_arcs.push(H::batch(batch_tools));
    }

    boxed_registry_from_arcs(tool_arcs)
}

#[allow(clippy::implicit_hasher, clippy::too_many_arguments)]
pub fn all_tools<H: FactoryHost>(
    config: Arc<Config>,
    security: &Arc<SecurityPolicy>,
    memory: Arc<dyn Memory>,
    browser_config: &agent_config::runtime_schema::BrowserConfig,
    http_config: &agent_config::runtime_schema::HttpRequestConfig,
    web_fetch_config: &agent_config::runtime_schema::WebFetchConfig,
    workspace_dir: &std::path::Path,
    agents: &HashMap<String, DelegateAgentConfig>,
    fallback_api_key: Option<&str>,
    root_config: &agent_config::runtime_schema::Config,
    session_id_override: Option<&str>,
) -> Vec<Box<dyn Tool>> {
    all_tools_with_runtime::<H>(
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
pub fn all_tools_with_runtime<H: FactoryHost>(
    config: Arc<Config>,
    security: &Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
    memory: Arc<dyn Memory>,
    browser_config: &agent_config::runtime_schema::BrowserConfig,
    http_config: &agent_config::runtime_schema::HttpRequestConfig,
    web_fetch_config: &agent_config::runtime_schema::WebFetchConfig,
    workspace_dir: &std::path::Path,
    agents: &HashMap<String, DelegateAgentConfig>,
    _fallback_api_key: Option<&str>,
    root_config: &agent_config::runtime_schema::Config,
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
    let project_scope = H::project_scope(workspace_dir);

    let syscall_detector = Arc::new(syscall_anomaly::SyscallAnomalyDetector::new(
        root_config.security.syscall_anomaly.clone(),
        &vibewindow_dir,
        root_config.security.audit.clone(),
    ));
    let cron_full_access = security.autonomy == AutonomyLevel::Full && !security.workspace_only;
    let construction = ConstructionContext { root_config: Some(root_config), http_config: Some(http_config), web_fetch_config: Some(web_fetch_config), api_key: None, cron_full_access, config: Some(&config), security, runtime: &runtime, memory: Some(&memory), browser_config: Some(browser_config), workspace_dir: Some(workspace_dir), session_id: Some(session_id), project_scope: project_scope.clone(), syscall_detector: Some(&syscall_detector) };
    let mut tool_arcs: Vec<Arc<dyn Tool>> = vec![
        H::construct(ToolConstruction::ConfigTool, &construction),
        H::construct(ToolConstruction::CronAddTool, &construction),
        H::construct(ToolConstruction::CronListTool, &construction),
        H::construct(ToolConstruction::CronRemoveTool, &construction),
        H::construct(ToolConstruction::CronUpdateTool, &construction),
        H::construct(ToolConstruction::CronRunTool, &construction),
        H::construct(ToolConstruction::CronRunsTool, &construction),
        H::construct(ToolConstruction::MemoryStoreTool, &construction),
        H::construct(ToolConstruction::MemoryRecallTool, &construction),
        H::construct(ToolConstruction::MemoryForgetTool, &construction),
        H::construct(ToolConstruction::ScheduleTool, &construction),
    ];

    #[cfg(not(target_arch = "wasm32"))]
    if has_shell_access {
        tool_arcs.push(H::construct(ToolConstruction::ShellToolWithSyscallDetector, &construction));
        tool_arcs.push(H::construct(ToolConstruction::BashWaitTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::BashCancelTool, &construction));
        #[cfg(target_os = "windows")]
        tool_arcs.push(H::construct(ToolConstruction::PowerShellTool, &construction));
        tool_arcs
            .push(H::construct(ToolConstruction::GitOperationsTool, &construction));
    }

    #[cfg(not(target_arch = "wasm32"))]
    if has_filesystem_access {
        tool_arcs.push(H::construct(ToolConstruction::FileReadToolWithSession, &construction));
        tool_arcs.push(H::construct(ToolConstruction::FileEditTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::FileWriteTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::ApplyPatchTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::LsTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::GlobTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::GrepTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::EnterWorktreeTool, &construction));
        tool_arcs.push(H::construct(ToolConstruction::ExitWorktreeTool, &construction));
    }

    #[cfg(not(target_arch = "wasm32"))]
    if browser_config.enabled {
        tracing::debug!(
            "all_tools_with_runtime: browser_config.enabled=true, browser_open={}",
            browser_config.browser_open
        );

        if !browser_config.browser_open.eq_ignore_ascii_case("disable") {
            tracing::debug!("all_tools_with_runtime: creating BrowserOpenTool");
            tool_arcs.push(H::construct(ToolConstruction::BrowserOpenTool, &construction));
        } else {
            tracing::debug!("all_tools_with_runtime: browser_open is disabled");
        }

        tracing::debug!("all_tools_with_runtime: creating BrowserTool");
        tool_arcs.push(H::construct(ToolConstruction::BrowserToolWithBackend, &construction));
    }

    if http_config.enabled {
        tool_arcs.push(H::construct(ToolConstruction::HttpRequestTool, &construction));
    }

    if web_fetch_config.enabled {
        tool_arcs.push(H::construct(ToolConstruction::WebFetchTool, &construction));
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

        let construction = ConstructionContext { api_key: api_key.clone(), ..construction.clone() };
        if provider == "exa" {
            tool_arcs.push(H::construct(ToolConstruction::ExaWebSearchTool, &construction));
        } else {
            tool_arcs.push(H::construct(ToolConstruction::WebSearchTool, &construction));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    tool_arcs.push(H::construct(ToolConstruction::PdfReadTool, &construction));

    #[cfg(not(target_arch = "wasm32"))]
    tool_arcs.push(H::construct(ToolConstruction::ScreenshotTool, &construction));
    tool_arcs.push(H::construct(ToolConstruction::ImageAssetReadTool, &construction));

    tool_arcs.push(H::construct(ToolConstruction::SkillToolWithRuntimeConfig, &construction));
    tool_arcs.push(H::construct(ToolConstruction::SleepTool, &construction));
    tool_arcs
        .push(H::construct(ToolConstruction::TodoReadToolScoped, &construction));
    tool_arcs.push(H::construct(ToolConstruction::TodoWriteToolScoped, &construction));
    tool_arcs.push(H::construct(ToolConstruction::QuestionTool, &construction));
    tool_arcs.push(H::construct(ToolConstruction::CreateThreadTool, &construction));
    tool_arcs.push(H::construct(ToolConstruction::ForkThreadTool, &construction));
    tool_arcs.push(H::construct(ToolConstruction::HandoffThreadTool, &construction));
    tool_arcs.push(H::construct(ToolConstruction::ListThreadsTool, &construction));
    tool_arcs.push(H::construct(ToolConstruction::ReadThreadTool, &construction));
    tool_arcs.push(H::construct(ToolConstruction::WaitThreadsTool, &construction));
    tool_arcs.push(H::construct(ToolConstruction::SendMessageToThreadTool, &construction));
    tool_arcs.push(H::construct(ToolConstruction::SetThreadTitleTool, &construction));
    tool_arcs.push(H::construct(ToolConstruction::SetThreadArchivedTool, &construction));
    tool_arcs.push(H::construct(ToolConstruction::SetThreadPinnedTool, &construction));

    let delegate_agents =
        enabled_delegate_agents_with_project_skills::<H>(agents, workspace_dir, root_config);
    if !delegate_agents.is_empty() {
        let agent_tool = H::agent_tool(delegate_agents.clone(), session_id_override.map(str::to_owned), workspace_dir.to_path_buf(), None);
        tool_arcs.push(agent_tool);
    }

    tool_arcs.push(H::construct(ToolConstruction::McpStatusTool, &construction));
    #[cfg(unix)]
    tool_arcs.extend(H::native_tools(session_id_override));

    let connections = H::connections_for_factory(workspace_dir, session_id_override);
    let mcp_config = H::load_runtime_mcp_config(workspace_dir);
    H::block_on(crate::mcp::register_dynamic_mcp_tools::<H>(&mut tool_arcs, mcp_config, connections));

    let catalog = ToolDiscoveryCatalog::new(tool_arcs.iter().map(|tool| tool.spec()).collect());
    if catalog.has_deferred_tools() {
        tool_arcs.push(H::search_tool(&catalog));
    }

    let batch_tools = Arc::new(tool_arcs.clone());
    tool_arcs.push(H::batch(batch_tools));
    boxed_registry_from_arcs(tool_arcs)
}

pub fn enabled_delegate_agents_with_project_skills<H: FactoryHost>(
    agents: &HashMap<String, DelegateAgentConfig>,
    workspace_dir: &Path,
    root_config: &Config,
) -> HashMap<String, DelegateAgentConfig> {
    let mut enabled = enabled_delegate_agents(agents);
    if !root_config.skills.auto_load_project_skills || enabled.is_empty() {
        return enabled;
    }

    let project_skill_names = H::project_skills(workspace_dir, root_config);

    crate::agents::extend_project_skills(&mut enabled, &project_skill_names);
    enabled
}
