//! 跨平台出站格式的兼容导出。
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_support::outbound_format::*;
