//! 工作流执行能力；由宿主组合运行器与会话适配。

mod code_runner;
mod code_runner_http;

pub use code_runner::run_code_node;

#[cfg(not(target_arch = "wasm32"))]
mod native_permissions;
#[cfg(not(target_arch = "wasm32"))]
pub use native_permissions::check_allowed;

#[cfg(not(target_arch = "wasm32"))]
mod knowledge;

mod types;
pub use types::*;

#[cfg(not(target_arch = "wasm32"))]
pub mod native_video_progress;

mod runner;
pub use runner::{run_workflow, run_workflow_with_events, resume_workflow};

#[cfg(test)]
mod dsml_tests;

#[cfg(test)]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod native_agent;

#[cfg(not(target_arch = "wasm32"))]
pub mod native_tools;

#[cfg(not(target_arch = "wasm32"))]
pub mod native_command;

#[cfg(not(target_arch = "wasm32"))]
mod video_providers;
