//! 本地运行器兼容导出。
#[cfg(any(test, target_arch = "wasm32"))]
pub use task_scheduler::runner::workspace_claim::*;
