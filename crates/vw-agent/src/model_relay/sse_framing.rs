//! Shared SSE framing lives in model-protocol.
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use model_protocol::sse_framing::*;
