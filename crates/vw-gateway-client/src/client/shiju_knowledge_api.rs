//! 识矩知识兼容门面。
use super::{GatewayClient, ShijuCredential};
pub use gateway_shiju_knowledge::*;

impl GatewayClient {
    pub async fn shiju_knowledge_bases(
        &self,
        credential: &ShijuCredential,
        query: &ShijuKnowledgePageQuery,
    ) -> Result<ShijuKnowledgePageDto<ShijuKnowledgeBaseDto>, String> {
        gateway_shiju_knowledge::GatewayShijuKnowledgeClient::new(self.transport.clone())
            .shiju_knowledge_bases(credential, query).await
    }
    pub async fn shiju_knowledge_documents(
        &self,
        credential: &ShijuCredential,
        kb_zid: &str,
        query: &ShijuKnowledgePageQuery,
    ) -> Result<ShijuKnowledgePageDto<ShijuKnowledgeDocumentDto>, String> {
        gateway_shiju_knowledge::GatewayShijuKnowledgeClient::new(self.transport.clone())
            .shiju_knowledge_documents(credential, kb_zid, query).await
    }
    pub async fn shiju_knowledge_search(
        &self,
        credential: &ShijuCredential,
        request: &ShijuKnowledgeSearchRequest,
    ) -> Result<ShijuKnowledgeSearchDto, String> {
        gateway_shiju_knowledge::GatewayShijuKnowledgeClient::new(self.transport.clone())
            .shiju_knowledge_search(credential, request).await
    }
    pub async fn shiju_knowledge_read(
        &self,
        credential: &ShijuCredential,
        request: &ShijuKnowledgeReadRequest,
    ) -> Result<ShijuKnowledgeReadDto, String> {
        gateway_shiju_knowledge::GatewayShijuKnowledgeClient::new(self.transport.clone())
            .shiju_knowledge_read(credential, request).await
    }
}
