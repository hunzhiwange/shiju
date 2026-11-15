//! 识矩云端知识库分页、检索与正文读取客户端。
#[derive(Debug, Clone)]
pub struct GatewayShijuKnowledgeClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayShijuKnowledgeClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod shiju_knowledge_api;
pub use shiju_knowledge_api::*;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod shiju_knowledge_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
