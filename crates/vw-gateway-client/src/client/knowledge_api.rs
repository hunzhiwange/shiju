//! 知识库网关客户端。
//!
//! 提供数据集、文档、导入任务、嵌入配置与检索接口，并保持查询参数与共享 API 类型一致。

use gateway_knowledge::GatewayKnowledgeClient;
use vw_api_knowledge::{
    KnowledgeChunkDto, KnowledgeDatasetCreateRequest, KnowledgeDatasetDto,
    KnowledgeDocumentCreateRequest, KnowledgeDocumentDto, KnowledgeDocumentImportQueuedResponse,
    KnowledgeDocumentImportRequest, KnowledgeDocumentImportResponse, KnowledgeDocumentListQuery,
    KnowledgeDocumentPatchRequest, KnowledgeDocumentReindexRequest,
    KnowledgeDocumentsBatchActionRequest, KnowledgeEmbeddingPresetDto, KnowledgeEmbeddingStatusDto,
    KnowledgeEmbeddingTestRequest, KnowledgeEmbeddingTestResponse, KnowledgeIndexJobDto,
    KnowledgeRetrieveRequest, KnowledgeRetrieveResponse, KnowledgeRuntimeStatus,
};

use super::GatewayClient;

impl GatewayClient {
    pub async fn knowledge_status(&self) -> Result<KnowledgeRuntimeStatus, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_status().await
    }

    pub async fn knowledge_datasets_list(&self) -> Result<Vec<KnowledgeDatasetDto>, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_datasets_list().await
    }

    pub async fn knowledge_dataset_get(
        &self,
        dataset_id: &str,
    ) -> Result<KnowledgeDatasetDto, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_dataset_get(dataset_id).await
    }

    pub async fn knowledge_dataset_create(
        &self,
        body: &KnowledgeDatasetCreateRequest,
    ) -> Result<KnowledgeDatasetDto, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_dataset_create(body).await
    }

    pub async fn knowledge_dataset_delete(
        &self,
        dataset_id: &str,
    ) -> Result<KnowledgeDatasetDto, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_dataset_delete(dataset_id).await
    }

    pub async fn knowledge_documents_list(
        &self,
        dataset_id: &str,
    ) -> Result<Vec<KnowledgeDocumentDto>, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_documents_list(dataset_id).await
    }

    pub async fn knowledge_documents_query(
        &self,
        dataset_id: &str,
        query: &KnowledgeDocumentListQuery,
    ) -> Result<Vec<KnowledgeDocumentDto>, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_documents_query(dataset_id, query).await
    }

    pub async fn knowledge_document_create(
        &self,
        dataset_id: &str,
        body: &KnowledgeDocumentCreateRequest,
    ) -> Result<KnowledgeDocumentDto, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_document_create(dataset_id, body).await
    }

    pub async fn knowledge_document_import(
        &self,
        dataset_id: &str,
        body: &KnowledgeDocumentImportRequest,
    ) -> Result<KnowledgeDocumentImportResponse, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_document_import(dataset_id, body).await
    }

    pub async fn knowledge_document_import_async(
        &self,
        dataset_id: &str,
        body: &KnowledgeDocumentImportRequest,
    ) -> Result<KnowledgeDocumentImportQueuedResponse, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_document_import_async(dataset_id, body).await
    }

    pub async fn knowledge_index_job_get(
        &self,
        job_id: &str,
    ) -> Result<KnowledgeIndexJobDto, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_index_job_get(job_id).await
    }

    pub async fn knowledge_document_delete(
        &self,
        document_id: &str,
    ) -> Result<KnowledgeDocumentDto, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_document_delete(document_id).await
    }

    pub async fn knowledge_document_update(
        &self,
        document_id: &str,
        body: &KnowledgeDocumentPatchRequest,
    ) -> Result<KnowledgeDocumentDto, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_document_update(document_id, body).await
    }

    pub async fn knowledge_documents_batch_action(
        &self,
        body: &KnowledgeDocumentsBatchActionRequest,
    ) -> Result<Vec<KnowledgeDocumentDto>, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_documents_batch_action(body).await
    }

    pub async fn knowledge_chunks_list(
        &self,
        document_id: &str,
    ) -> Result<Vec<KnowledgeChunkDto>, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_chunks_list(document_id).await
    }

    pub async fn knowledge_document_reindex(
        &self,
        document_id: &str,
        body: &KnowledgeDocumentReindexRequest,
    ) -> Result<KnowledgeDocumentDto, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_document_reindex(document_id, body).await
    }

    pub async fn knowledge_retrieve(
        &self,
        body: &KnowledgeRetrieveRequest,
    ) -> Result<KnowledgeRetrieveResponse, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_retrieve(body).await
    }

    pub async fn knowledge_retrieval_test(
        &self,
        body: &KnowledgeRetrieveRequest,
    ) -> Result<KnowledgeRetrieveResponse, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_retrieval_test(body).await
    }

    pub async fn knowledge_embedding_status(&self) -> Result<KnowledgeEmbeddingStatusDto, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_embedding_status().await
    }

    pub async fn knowledge_embedding_presets(
        &self,
    ) -> Result<Vec<KnowledgeEmbeddingPresetDto>, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_embedding_presets().await
    }

    pub async fn knowledge_embedding_test(
        &self,
        body: &KnowledgeEmbeddingTestRequest,
    ) -> Result<KnowledgeEmbeddingTestResponse, String> {
        GatewayKnowledgeClient::new(self.transport.clone()).knowledge_embedding_test(body).await
    }
}

#[cfg(test)]
#[path = "knowledge_facade_tests.rs"]
mod knowledge_facade_tests;
