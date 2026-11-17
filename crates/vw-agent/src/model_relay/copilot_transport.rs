#[cfg(any(test, target_arch = "wasm32"))]
pub use model_transport::copilot_transport::*;
