//! 识矩私有技能查询与远程安装客户端。
#[derive(Debug, Clone)]
pub struct GatewayShijuSkillClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayShijuSkillClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod types;
pub use types::*;
mod shiju_skill_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod shiju_skill_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
