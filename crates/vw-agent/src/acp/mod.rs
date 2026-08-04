//! VibeWindow 原生运行时的 ACP Agent 后端适配。

mod approval;
mod backend;
mod mcp;

pub use backend::VibeWindowBackend;

#[cfg(test)]
#[path = "backend_tests.rs"]
mod backend_tests;

#[cfg(test)]
#[path = "approval_tests.rs"]
mod approval_tests;

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod mcp_tests;
