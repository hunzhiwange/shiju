//! Concrete domain construction shared by real application factories.
use crate::factory::{ToolConstruction, ConstructionContext};
use crate::Tool;
use std::sync::Arc;
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
 Arc::new(PowerShellTool::<H>::<H>::new(security.clone())) },
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

_ => return None,
 }; Some(result)
}
