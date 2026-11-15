//! 推理、JSON SSE 与 WebSocket chat 领域客户端。

#[derive(Debug, Clone)]
pub struct GatewayInferenceClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayInferenceClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod inference_api;
pub use inference_api::*;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
