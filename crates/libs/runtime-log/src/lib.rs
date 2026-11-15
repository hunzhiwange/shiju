//! 运行时日志文件写入工具。

#![cfg(not(target_arch = "wasm32"))]

mod runtime_log;
pub use runtime_log::*;
