//! 统一技能市场领域客户端与传输契约。
#[derive(Debug, Clone)]
pub struct GatewaySkillMarketClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewaySkillMarketClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod types;
pub use types::*;
mod skill_market_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod skill_market_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
