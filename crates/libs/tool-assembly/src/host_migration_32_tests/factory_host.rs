//! Explicit construction port: records selected tool IDs; never executes a fake tool.
use super::host::Host;
use crate::{Tool, ToolResult, factory::*, runtime::*, discovery::ToolDiscoveryCatalog};
use std::{sync::Arc, path::{Path, PathBuf}, collections::HashMap};
use agent_config::runtime_schema::{Config, DelegateAgentConfig};
use plugin_mcp::mcp_connections::McpSessionConnections;
use serde_json::{Value, json};
struct Selected(&'static str);
#[async_trait::async_trait]
impl Tool for Selected {
 fn name(&self) -> &str { self.0 }
 fn description(&self) -> &str { "Construction port receipt" }
 fn parameters_schema(&self) -> Value { json!({"type":"object"}) }
 async fn execute(&self, _: Value) -> anyhow::Result<ToolResult> { anyhow::bail!("construction receipt cannot execute") }
}
impl FactoryHost for Host {
 fn block_on<F: Future + Send>(f: F) -> F::Output where F::Output: Send {
  std::thread::scope(|s| s.spawn(|| tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(f)).join().unwrap())
 }
 fn construct(tool: ToolConstruction, context: &ConstructionContext<'_>) -> Arc<dyn Tool> {
  if let Some(tool) = crate::domain_construction::construct_domain_tool::<Self>(&tool, context) { return tool; }
  use ToolConstruction::*;
  let id = match tool {
#[cfg(not(target_arch = "wasm32"))]
ShellTool => "bash",
#[cfg(not(target_arch = "wasm32"))]
BashWaitTool => "bash_wait",
#[cfg(not(target_arch = "wasm32"))]
BashCancelTool => "bash_cancel",
#[cfg(not(target_arch = "wasm32"))]
FileReadTool => "file_read",
#[cfg(not(target_arch = "wasm32"))]
FileEditTool => "file_edit",
#[cfg(not(target_arch = "wasm32"))]
FileWriteTool => "file_write",
#[cfg(not(target_arch = "wasm32"))]
ApplyPatchTool => "apply_patch",
#[cfg(not(target_arch = "wasm32"))]
LsTool => "ls",
#[cfg(not(target_arch = "wasm32"))]
GlobTool => "glob",
#[cfg(not(target_arch = "wasm32"))]
GrepTool => "grep",
ConfigTool => "config",
CronAddTool => "cron_add",
CronListTool => "cron_list",
CronRemoveTool => "cron_remove",
CronUpdateTool => "cron_update",
CronRunTool => "cron_run",
CronRunsTool => "cron_runs",
MemoryStoreTool => "memory_store",
MemoryRecallTool => "memory_recall",
MemoryForgetTool => "memory_forget",
ScheduleTool => "schedule",
#[cfg(not(target_arch = "wasm32"))]
ShellToolWithSyscallDetector => "bash",
#[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
PowerShellTool => "power_shell",
#[cfg(not(target_arch = "wasm32"))]
GitOperationsTool => "git_operations",
#[cfg(not(target_arch = "wasm32"))]
FileReadToolWithSession => "file_read",
#[cfg(not(target_arch = "wasm32"))]
EnterWorktreeTool => "enter_worktree",
#[cfg(not(target_arch = "wasm32"))]
ExitWorktreeTool => "exit_worktree",
#[cfg(not(target_arch = "wasm32"))]
BrowserOpenTool => "browser_open",
#[cfg(not(target_arch = "wasm32"))]
BrowserToolWithBackend => "browser",
HttpRequestTool => "http_request",
WebFetchTool => "web_fetch",
ExaWebSearchTool => "web_search_tool",
WebSearchTool => "web_search_tool",
#[cfg(not(target_arch = "wasm32"))]
PdfReadTool => "pdf_read",
#[cfg(not(target_arch = "wasm32"))]
ScreenshotTool => "screenshot",
ImageAssetReadTool => "image_asset_read",
SkillToolWithRuntimeConfig => "SkillTool",
SleepTool => "sleep",
TodoReadToolScoped => "todo_read",
TodoWriteToolScoped => "todo_write",
QuestionTool => "AskUserQuestion",
CreateThreadTool => "create_thread",
ForkThreadTool => "fork_thread",
HandoffThreadTool => "handoff_thread",
ListThreadsTool => "list_threads",
ReadThreadTool => "read_thread",
WaitThreadsTool => "wait_threads",
SendMessageToThreadTool => "send_message_to_thread",
SetThreadTitleTool => "set_thread_title",
SetThreadArchivedTool => "set_thread_archived",
SetThreadPinnedTool => "set_thread_pinned",
McpStatusTool => "mcp_status",
  }; Arc::new(Selected(id))
 }
 fn batch(_: Arc<Vec<Arc<dyn Tool>>>) -> Arc<dyn Tool> { Arc::new(Selected("batch")) }
 fn search_tool(catalog: &ToolDiscoveryCatalog) -> Arc<dyn Tool> { Arc::new(crate::tool_search::ContextToolSearchTool::<Self>::new(catalog)) }
 fn agent_tool(_: HashMap<String, DelegateAgentConfig>, _: Option<String>, _: PathBuf, _: Option<tool_agents::agent_tool::AgentToolChildSessionCreatedCallback>) -> Arc<dyn Tool> { Arc::new(Selected("AgentTool")) }
 fn project_scope(_: &Path) -> Option<String> { None }
 fn project_skills(workspace: &Path, config: &Config) -> Vec<String> { agent_skills::catalog::load_project_skills_with_config(workspace, &config.skills).into_iter().map(|skill| skill.name).collect() }
 fn connections_for_factory(_: &Path, _: Option<&str>) -> Arc<McpSessionConnections> { Arc::new(<Self as tool_mcp::McpToolHost>::new_connections()) }
 fn load_runtime_mcp_config(workspace: &Path) -> anyhow::Result<acp_config::ResolvedAcpxConfig> { Ok(Self::block_on(acp_config::load_resolved_config_from_paths(workspace.join("missing-global.json"), workspace.join(".vwacprc.json")))?) }
 #[cfg(unix)]
 fn native_tools(_: Option<&str>) -> Vec<Arc<dyn Tool>> { Vec::new() }
}
impl RuntimeHost for Host {
 async fn load_project_config(_: &vw_api_transport::TenantScope, workspace: &Path) -> anyhow::Result<Config> { Ok(Config { workspace_dir: workspace.into(), config_path: workspace.join(".vibewindow.json"), ..Config::default() }) }
 fn mcp_connections(_: &str, _: &Path) -> Arc<McpSessionConnections> { Arc::new(<Self as tool_mcp::McpToolHost>::new_connections()) }
 fn discovery_state(discovery: &Self::ToolDiscoverySession) -> &crate::tool_search::ToolDiscoverySession { discovery }
 fn register_app(_: &ToolRuntimeContext<Self>, _: &str, _: &mut crate::ToolCallResult) -> anyhow::Result<()> { anyhow::bail!("wire fixture has no apps") }
}
impl crate::tool_search::SearchHost for Host {
 fn search_session(context: &tool_context::context::ToolUseContext<Self>) -> Option<Arc<crate::tool_search::ToolDiscoverySession>> { context.tool_discovery() }
 fn record_search(_: &tool_context::context::ToolUseContext<Self>, _: Value) {}
}
