//! Agent bindings for the tool-shell implementation.
pub use tool_shell::shell::{ast, compound, path, permissions, readonly, security, sed};
pub use tool_shell::shell::sandbox;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod processes;
#[cfg(not(target_arch = "wasm32"))]
pub use processes::shutdown_shell_processes;
#[cfg(not(target_arch = "wasm32"))]
pub mod session_tools;
pub type ShellTool = tool_shell::ShellTool<crate::tools::context::AgentContextHost>;
pub(crate) use tool_shell::shell::{apply_allowed_shell_environment, collect_allowed_shell_env_vars, allowed_shell_env_value};
#[cfg(test)]
use super::traits::Tool;
#[cfg(test)]
use agent_security::{AutonomyLevel, SecurityPolicy};
#[cfg(test)]
use serde_json::json;
#[cfg(test)]
use std::{sync::Arc, path::Path, time::Duration};
#[cfg(all(test, not(target_arch = "wasm32")))]
mod session_tools_tests;
#[cfg(all(test, unix))]
mod long_running_tests;
