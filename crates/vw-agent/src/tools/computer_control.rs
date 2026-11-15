//! Computer tools use the shared capability library.
pub use tool_computer::{NativeBackend, NativeControlRuntime};
pub type ComputerControlTool = tool_computer::ComputerControlTool<super::context::AgentContextHost>;

#[cfg(test)]
#[path = "../../../libs/tool-computer/src/computer_control/chrome_tests.rs"]
mod chrome_tests;
