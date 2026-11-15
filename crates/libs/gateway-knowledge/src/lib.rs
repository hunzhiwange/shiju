//! 知识库数据集、文档、嵌入与检索领域客户端。

#[derive(Debug, Clone)]
pub struct GatewayKnowledgeClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayKnowledgeClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod knowledge_api;

#[cfg(test)]
mod test_support;

#[cfg(not(target_arch = "wasm32"))]
mod handlers;
#[cfg(not(target_arch = "wasm32"))]
pub use handlers::{router, knowledge_store, knowledge_db_path};
