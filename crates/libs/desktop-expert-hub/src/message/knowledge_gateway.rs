//! 知识库网关请求，保持每次请求的宿主客户端获取时机。
use super::*;
use gateway_knowledge::GatewayKnowledgeClient;

pub(super) async fn status_async(gateway: crate::GatewayFactory) -> Result<KnowledgeRuntimeStatus, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_status().await
}

pub(super) async fn embedding_status_async(gateway: crate::GatewayFactory) -> Result<KnowledgeEmbeddingStatusDto, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_embedding_status().await
}

pub(super) async fn embedding_presets_async(gateway: crate::GatewayFactory) -> Result<Vec<KnowledgeEmbeddingPresetDto>, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_embedding_presets().await
}

pub(super) async fn list_datasets_async(gateway: crate::GatewayFactory) -> Result<Vec<KnowledgeDatasetDto>, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_datasets_list().await
}

pub(super) async fn list_documents_async(gateway: crate::GatewayFactory, dataset_id: String) -> Result<Vec<KnowledgeDocumentDto>, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_documents_list(&dataset_id).await
}

pub(super) async fn list_chunks_async(gateway: crate::GatewayFactory, document_id: String) -> Result<Vec<KnowledgeChunkDto>, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_chunks_list(&document_id).await
}

pub(super) async fn create_dataset_async(gateway: crate::GatewayFactory,
    body: KnowledgeDatasetCreateRequest,
) -> Result<KnowledgeDatasetDto, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_dataset_create(&body).await
}

pub(super) async fn delete_dataset_async(gateway: crate::GatewayFactory, dataset_id: String) -> Result<KnowledgeDatasetDto, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_dataset_delete(&dataset_id).await
}

pub(super) async fn create_document_async(gateway: crate::GatewayFactory,
    dataset_id: String,
    body: KnowledgeDocumentCreateRequest,
) -> Result<KnowledgeDocumentDto, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_document_create(&dataset_id, &body).await
}

pub(super) async fn import_documents_async(gateway: crate::GatewayFactory,
    dataset_id: String,
    files: Vec<KnowledgeFileImportDraft>,
) -> Result<Vec<KnowledgeDocumentImportQueuedResponse>, String> {
    let client = GatewayKnowledgeClient::new(gateway()?);
    let mut responses = Vec::with_capacity(files.len());
    for file in files {
        let filename = file.filename.clone();
        let body = import_request_from_file(file);
        let response = client
            .knowledge_document_import_async(&dataset_id, &body)
            .await
            .map_err(|error| format!("{filename} 入队失败：{error}"))?;
        responses.push(response);
    }
    Ok(responses)
}

pub(super) async fn get_index_job_async(gateway: crate::GatewayFactory, job_id: String) -> Result<KnowledgeIndexJobDto, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_index_job_get(&job_id).await
}

pub(super) async fn delete_document_async(gateway: crate::GatewayFactory, document_id: String) -> Result<KnowledgeDocumentDto, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_document_delete(&document_id).await
}

pub(super) async fn update_document_async(gateway: crate::GatewayFactory,
    document_id: String,
    body: KnowledgeDocumentPatchRequest,
) -> Result<KnowledgeDocumentDto, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_document_update(&document_id, &body).await
}

pub(super) async fn batch_update_documents_async(gateway: crate::GatewayFactory,
    body: KnowledgeDocumentsBatchActionRequest,
) -> Result<Vec<KnowledgeDocumentDto>, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_documents_batch_action(&body).await
}

pub(super) async fn reindex_document_async(gateway: crate::GatewayFactory, document_id: String) -> Result<KnowledgeDocumentDto, String> {
    GatewayKnowledgeClient::new(gateway()?)
        .knowledge_document_reindex(&document_id, &KnowledgeDocumentReindexRequest { force: false })
        .await
}

pub(super) async fn retrieve_async(gateway: crate::GatewayFactory,
    body: KnowledgeRetrieveRequest,
) -> Result<KnowledgeRetrieveResponse, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_retrieve(&body).await
}

pub(super) async fn embedding_test_async(gateway: crate::GatewayFactory,
    body: KnowledgeEmbeddingTestRequest,
) -> Result<KnowledgeEmbeddingTestResponse, String> {
    GatewayKnowledgeClient::new(gateway()?).knowledge_embedding_test(&body).await
}
