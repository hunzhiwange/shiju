//! 任务文件存储，保留原生默认拒绝与 WASM JSON 兼容行为。

mod store;
pub use store::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
