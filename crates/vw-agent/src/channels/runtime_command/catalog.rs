//! 通道命令目录的兼容导出。
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_runtime::manager::catalog::*;
