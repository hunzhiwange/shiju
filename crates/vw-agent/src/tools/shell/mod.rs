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
use tool_shell::shell::{is_valid_env_var_name, extract_command_argument, extract_description_argument, parse_timeout_ms, parse_yield_time_ms, normalize_line_endings, merge_shell_output, dedupe_paths, tool_result_from_interpretation, append_note, SAFE_ENV_VARS, PLATFORM_SAFE_ENV_VARS};
#[cfg(test)]
use super::traits::Tool;
#[cfg(test)]
use agent_security::{AutonomyLevel, SecurityPolicy};
#[cfg(test)]
use serde_json::json;
#[cfg(test)]
use std::{sync::Arc, path::{Path, PathBuf}, time::Duration};
#[cfg(test)]
use tool_shell::shell::compound::semantics::ExitInterpretation;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
mod environment_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "session_tools_tests.rs"]
mod session_tools_tests;

#[cfg(all(test, unix))]
mod long_running_tests;
