//! 可恢复 SSE 实现由独立领域库提供。
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use gateway_resumable_stream::*;
