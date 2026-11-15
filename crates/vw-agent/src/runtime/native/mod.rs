//! Native runtime compatibility exports.
pub use agent_runtime::native::*;

#[cfg(test)]
mod shell_tool_tests;

#[cfg(all(test, windows))]
mod windows_commands_tests;
