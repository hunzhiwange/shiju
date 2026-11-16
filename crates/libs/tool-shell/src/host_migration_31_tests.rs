//! Domain regressions migrated from the agent; real registry/tenant bridges remain there.
use crate::shell::*;
use agent_security::{AutonomyLevel, SecurityPolicy};
use tool_types::Tool;
use serde_json::json;
use std::{sync::Arc, path::{Path, PathBuf}, time::Duration};
use crate::shell::compound::semantics::ExitInterpretation;
mod host;
use host::Host;
type ShellTool = crate::ShellTool<Host>;
type ToolUseContext = tool_context::context::ToolUseContext<Host>;
use tool_context::context::scope_tool_use_context;
fn global_env_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    &LOCK
}
mod tests;
mod environment_tests;
#[cfg(not(target_arch = "wasm32"))]
use crate::shell::processes::*;
#[cfg(not(target_arch = "wasm32"))]
mod processes_tests;
#[cfg(unix)]
mod termination_tests;
#[cfg(windows)]
mod termination_windows_tests;

#[cfg(target_os = "windows")]
mod powershell_35_tests;
