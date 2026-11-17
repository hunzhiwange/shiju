//! 通道运行实现的兼容导出。
#[cfg(any(test, target_arch = "wasm32"))]
pub use channel_runtime::manager::listener::*;
