//! 语音 HTTP 与原生双向流领域客户端。

#[derive(Debug, Clone)]
pub struct GatewayVoiceClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayVoiceClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod voice_api;
#[cfg(not(target_arch = "wasm32"))]
pub use voice_api::VoiceApiError;
#[cfg(not(target_arch = "wasm32"))]
mod voice_stream;
#[cfg(not(target_arch = "wasm32"))]
pub use voice_stream::VoiceStream;

#[cfg(not(target_arch = "wasm32"))]
pub mod voice_requests;
#[cfg(not(target_arch = "wasm32"))]
pub mod voice_upload;
