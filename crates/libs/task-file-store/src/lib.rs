//! 任务文件存储，保留原生默认拒绝与 WASM JSON 兼容行为。

pub mod bootstrap_task;
mod store;
pub use store::*;

#[cfg(not(target_arch = "wasm32"))]
pub mod task_quick_actions;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
