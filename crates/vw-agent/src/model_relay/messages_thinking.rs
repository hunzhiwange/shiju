#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use model_protocol::messages_thinking::*;
