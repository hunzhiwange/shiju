//! 通道共有支撑的兼容导出。
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_support::markdown_chunk::*;
