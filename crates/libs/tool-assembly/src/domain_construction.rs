//! Concrete domain construction shared by real application factories.
use crate::factory::{ToolConstruction, ConstructionContext};
use crate::Tool;
use std::sync::Arc;
use tool_services::{config::ConfigTool, schedule::ScheduleTool, sleep::SleepTool, todo::{TodoReadTool, TodoWriteTool}};
#[cfg(not(target_arch = "wasm32"))]
use tool_services::screenshot::ScreenshotTool;
#[cfg(not(target_arch = "wasm32"))]
use tool_browser::{BrowserTool, ComputerUseConfig, browser_open::BrowserOpenTool};
use tool_shell::ShellTool;
use tool_shell::shell::session_tools::{BashWaitTool, BashCancelTool};
#[cfg(target_os = "windows")]
use tool_shell::PowerShellTool;
use tool_files::{file_read::FileReadTool, file_edit::FileEditTool, file_write::FileWriteTool, ls::LsTool, glob::GlobTool, grep::GrepTool, pdf_read::PdfReadTool};
use tool_web::{HttpRequestTool, WebFetchTool, WebSearchTool};
#[cfg(not(target_arch = "wasm32"))]
use tool_services::apply_patch::ApplyPatchTool;
/// None means this construction needs a concrete application port.
pub fn construct_domain_tool<H>(tool: &ToolConstruction, context: &ConstructionContext<'_>) -> Option<Arc<dyn Tool>>
where H: tool_files::FileToolHost<SecurityPolicy = agent_security::SecurityPolicy> {
 let result: Arc<dyn Tool> = match tool {
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::ShellTool => { let security = context.security;
let runtime = context.runtime;
 Arc::new(ShellTool::<H>::new(security.clone(), runtime.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::BashWaitTool => { let security = context.security;
 Arc::new(BashWaitTool::<H>::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::BashCancelTool => { let security = context.security;
 Arc::new(BashCancelTool::<H>::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::ShellToolWithSyscallDetector => { let security = context.security;
let runtime = context.runtime;
let syscall_detector = context.syscall_detector.expect("syscall detector");
 Arc::new(ShellTool::<H>::new_with_syscall_detector(
            security.clone(),
            runtime.clone(),
            Some(syscall_detector.clone()),
        )) },
#[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
ToolConstruction::PowerShellTool => { let security = context.security;
 Arc::new(PowerShellTool::<H>::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::FileReadTool => { let security = context.security;
 Arc::new(FileReadTool::<H>::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::FileEditTool => { let security = context.security;
 Arc::new(FileEditTool::<H>::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::FileWriteTool => { let security = context.security;
 Arc::new(FileWriteTool::<H>::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::ApplyPatchTool => { let security = context.security;
 Arc::new(ApplyPatchTool::<H>::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::LsTool => { let security = context.security;
 Arc::new(LsTool::<H>::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::GlobTool => { let security = context.security;
 Arc::new(GlobTool::<H>::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::GrepTool => { let security = context.security;
 Arc::new(GrepTool::<H>::new(security.clone())) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::FileReadToolWithSession => { let security = context.security;
let session_id = context.session_id.expect("session");
 Arc::new(FileReadTool::<H>::with_session(security.clone(), session_id)) },
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::PdfReadTool => { let security = context.security;
 Arc::new(PdfReadTool::new(security.clone())) },
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
 Arc::new(tool_web::websearch::WebSearchTool::new(
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
ToolConstruction::MemoryStoreTool => { let security = context.security;
let memory = context.memory.expect("full factory memory");
 Arc::new(tool_memory::MemoryStoreTool::new(memory.clone(), security.clone())) },
ToolConstruction::MemoryRecallTool => { let memory = context.memory.expect("full factory memory");
 Arc::new(tool_memory::MemoryRecallTool::new(memory.clone())) },
ToolConstruction::MemoryForgetTool => { let security = context.security;
let memory = context.memory.expect("full factory memory");
 Arc::new(tool_memory::MemoryForgetTool::new(memory.clone(), security.clone())) },

ToolConstruction::ConfigTool => { let security = context.security;
let config = context.config.expect("full factory config");
 Arc::new(ConfigTool::new(config.clone(), security.clone())) },
ToolConstruction::ScheduleTool => { let security = context.security;
let root_config = context.root_config.expect("root config");
 Arc::new(ScheduleTool::new(security.clone(), root_config.clone())) },
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
#[cfg(not(target_arch = "wasm32"))]
ToolConstruction::ScreenshotTool => { let security = context.security;
 Arc::new(ScreenshotTool::new(security.clone())) },

_ => return None,
 }; Some(result)
}

/// Thread metadata and execution keep the application's existing typed host.
pub fn construct_thread_tool<H: tool_agents::thread_tool::ThreadHost>(tool: &ToolConstruction, context: &ConstructionContext<'_>) -> Option<Arc<dyn Tool>> {
 use tool_agents::thread_tool::*;
 let result: Arc<dyn Tool> = match tool {
ToolConstruction::CreateThreadTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(CreateThreadTool::<H>::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::ForkThreadTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(ForkThreadTool::<H>::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::HandoffThreadTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(HandoffThreadTool::<H>::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::ListThreadsTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(ListThreadsTool::<H>::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::ReadThreadTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(ReadThreadTool::<H>::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::WaitThreadsTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(WaitThreadsTool::<H>::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::SendMessageToThreadTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(SendMessageToThreadTool::<H>::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::SetThreadTitleTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(SetThreadTitleTool::<H>::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::SetThreadArchivedTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(SetThreadArchivedTool::<H>::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },
ToolConstruction::SetThreadPinnedTool => { let workspace_dir = context.workspace_dir.expect("workspace");
let session_id = context.session_id.expect("session");
 Arc::new(SetThreadPinnedTool::<H>::new(
        Some(session_id.to_string()),
        workspace_dir.to_path_buf(),
    )) },

_ => return None,
 }; Some(result)
}
