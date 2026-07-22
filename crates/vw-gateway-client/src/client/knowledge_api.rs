//! Knowledge base gateway client.

use vw_api_types::knowledge::{
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
        self.get_json("/v1/knowledge/status", &[]).await
    }

    pub async fn knowledge_datasets_list(&self) -> Result<Vec<KnowledgeDatasetDto>, String> {
        self.get_json("/v1/knowledge/datasets", &[]).await
    }

    pub async fn knowledge_dataset_get(
        &self,
        dataset_id: &str,
    ) -> Result<KnowledgeDatasetDto, String> {
        self.get_json(&format!("/v1/knowledge/datasets/{dataset_id}"), &[]).await
    }

    pub async fn knowledge_dataset_create(
        &self,
        body: &KnowledgeDatasetCreateRequest,
    ) -> Result<KnowledgeDatasetDto, String> {
        self.post_json("/v1/knowledge/datasets", &[], body).await
    }

    pub async fn knowledge_dataset_delete(
        &self,
        dataset_id: &str,
    ) -> Result<KnowledgeDatasetDto, String> {
        self.delete_json(
            &format!("/v1/knowledge/datasets/{dataset_id}"),
            &[],
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn knowledge_documents_list(
        &self,
        dataset_id: &str,
    ) -> Result<Vec<KnowledgeDocumentDto>, String> {
        self.knowledge_documents_query(dataset_id, &KnowledgeDocumentListQuery::default()).await
    }

    pub async fn knowledge_documents_query(
        &self,
        dataset_id: &str,
        query: &KnowledgeDocumentListQuery,
    ) -> Result<Vec<KnowledgeDocumentDto>, String> {
        self.get_json(
            &format!("/v1/knowledge/datasets/{dataset_id}/documents"),
            &knowledge_document_list_query(query),
        )
        .await
    }

    pub async fn knowledge_document_create(
        &self,
        dataset_id: &str,
        body: &KnowledgeDocumentCreateRequest,
    ) -> Result<KnowledgeDocumentDto, String> {
        self.post_json(&format!("/v1/knowledge/datasets/{dataset_id}/documents"), &[], body).await
    }

    pub async fn knowledge_document_import(
        &self,
        dataset_id: &str,
        body: &KnowledgeDocumentImportRequest,
    ) -> Result<KnowledgeDocumentImportResponse, String> {
        self.post_json(&format!("/v1/knowledge/datasets/{dataset_id}/documents/import"), &[], body)
            .await
    }

    pub async fn knowledge_document_import_async(
        &self,
        dataset_id: &str,
        body: &KnowledgeDocumentImportRequest,
    ) -> Result<KnowledgeDocumentImportQueuedResponse, String> {
        self.post_json(
            &format!("/v1/knowledge/datasets/{dataset_id}/documents/import-async"),
            &[],
            body,
        )
        .await
    }

    pub async fn knowledge_index_job_get(
        &self,
        job_id: &str,
    ) -> Result<KnowledgeIndexJobDto, String> {
        self.get_json(&format!("/v1/knowledge/index-jobs/{job_id}"), &[]).await
    }

    pub async fn knowledge_document_delete(
        &self,
        document_id: &str,
    ) -> Result<KnowledgeDocumentDto, String> {
        self.delete_json(
            &format!("/v1/knowledge/documents/{document_id}"),
            &[],
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn knowledge_document_update(
        &self,
        document_id: &str,
        body: &KnowledgeDocumentPatchRequest,
    ) -> Result<KnowledgeDocumentDto, String> {
        self.patch_json(&format!("/v1/knowledge/documents/{document_id}"), &[], body).await
    }

    pub async fn knowledge_documents_batch_action(
        &self,
        body: &KnowledgeDocumentsBatchActionRequest,
    ) -> Result<Vec<KnowledgeDocumentDto>, String> {
        self.post_json("/v1/knowledge/documents/batch", &[], body).await
    }

    pub async fn knowledge_chunks_list(
        &self,
        document_id: &str,
    ) -> Result<Vec<KnowledgeChunkDto>, String> {
        self.get_json(&format!("/v1/knowledge/documents/{document_id}/chunks"), &[]).await
    }

    pub async fn knowledge_document_reindex(
        &self,
        document_id: &str,
        body: &KnowledgeDocumentReindexRequest,
    ) -> Result<KnowledgeDocumentDto, String> {
        self.post_json(&format!("/v1/knowledge/documents/{document_id}/reindex"), &[], body).await
    }

    pub async fn knowledge_retrieve(
        &self,
        body: &KnowledgeRetrieveRequest,
    ) -> Result<KnowledgeRetrieveResponse, String> {
        self.post_json("/v1/knowledge/retrieve", &[], body).await
    }

    pub async fn knowledge_embedding_status(&self) -> Result<KnowledgeEmbeddingStatusDto, String> {
        self.get_json("/v1/knowledge/embedding/status", &[]).await
    }

    pub async fn knowledge_embedding_presets(
        &self,
    ) -> Result<Vec<KnowledgeEmbeddingPresetDto>, String> {
        self.get_json("/v1/knowledge/embedding/presets", &[]).await
    }

    pub async fn knowledge_embedding_test(
        &self,
        body: &KnowledgeEmbeddingTestRequest,
    ) -> Result<KnowledgeEmbeddingTestResponse, String> {
        self.post_json("/v1/knowledge/embedding/test", &[], body).await
    }
}

fn knowledge_document_list_query(query: &KnowledgeDocumentListQuery) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    if let Some(status) = query.status.as_ref() {
        pairs.push((
            "status".to_string(),
            serde_json::to_value(status)
                .expect("knowledge document status serializes")
                .as_str()
                .expect("knowledge document status is a string")
                .to_string(),
        ));
    }
    pairs
}

#[cfg(test)]
#[path = "knowledge_api_tests.rs"]
mod knowledge_api_tests;
