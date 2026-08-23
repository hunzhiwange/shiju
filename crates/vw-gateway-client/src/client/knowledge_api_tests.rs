//! 验证知识库数据集、文档、导入任务及检索接口的完整路由契约。
//!
//! 所有请求均检查默认租户作用域，并覆盖同步导入、异步作业、批量操作和嵌入测试。

use serde_json::json;
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER};
use vw_api_types::knowledge::{
    KnowledgeCapabilitiesDto, KnowledgeChunkingMode, KnowledgeDatasetCreateRequest,
    KnowledgeDocumentCreateRequest, KnowledgeDocumentImportRequest, KnowledgeDocumentListQuery,
    KnowledgeDocumentPatchRequest, KnowledgeDocumentReindexRequest, KnowledgeDocumentStatus,
    KnowledgeDocumentsBatchAction, KnowledgeDocumentsBatchActionRequest,
    KnowledgeEmbeddingTestRequest, KnowledgeIndexJobStatus, KnowledgeIndexingMode,
    KnowledgeMatchType, KnowledgeRetrievalMode, KnowledgeRetrieveRequest,
};

use crate::client::test_support;

fn take_scoped_request(server: &test_support::TestServer) -> test_support::RecordedRequest {
    let request = server.take_request();
    assert_eq!(request.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(request.header(COMPANY_ID_HEADER), Some("100000000"));
    request
}

fn dataset() -> serde_json::Value {
    json!({
        "id": "ds",
        "name": "Docs",
        "description": "",
        "document_count": 1,
        "chunk_count": 2,
        "created_at_ms": 1,
        "updated_at_ms": 2
    })
}

fn document() -> serde_json::Value {
    json!({
        "id": "doc",
        "dataset_id": "ds",
        "name": "Readme",
        "metadata": {},
        "enabled": true,
        "chunk_count": 2,
        "created_at_ms": 1,
        "updated_at_ms": 2
    })
}

#[tokio::test]
async fn knowledge_api_routes_all_dataset_document_and_retrieval_calls() {
    let server = test_support::server(vec![
        (
            200,
            json!({"full_text": true, "vector": false, "hybrid": false, "rerank": false, "notes": {}}),
        ),
        (200, json!([dataset()])),
        (200, dataset()),
        (200, dataset()),
        (200, dataset()),
        (200, json!([document()])),
        (200, document()),
        (
            200,
            json!([{
                "id": "chunk",
                "dataset_id": "ds",
                "document_id": "doc",
                "title": "Readme",
                "content": "hello",
                "metadata": {},
                "ordinal": 0
            }]),
        ),
        (200, document()),
        (200, document()),
        (
            200,
            json!({"chunks": [{
                "id": "chunk",
                "dataset_id": "ds",
                "document_id": "doc",
                "title": "Readme",
                "content": "hello",
                "metadata": {},
                "score": 0.9
            }]}),
        ),
    ]);

    assert!(server.client().knowledge_status().await.expect("status").full_text);
    let datasets = server.client().knowledge_datasets_list().await.expect("datasets");
    assert_eq!(datasets.len(), 1);
    assert_eq!(datasets[0].capabilities, KnowledgeCapabilitiesDto::default());
    assert_eq!(server.client().knowledge_dataset_get("ds").await.expect("dataset").id, "ds");
    assert_eq!(
        server
            .client()
            .knowledge_dataset_create(&KnowledgeDatasetCreateRequest {
                name: "Docs".to_string(),
                description: String::new(),
                chunking_mode: KnowledgeChunkingMode::General,
                indexing_mode: KnowledgeIndexingMode::Economy,
                retrieval_mode: KnowledgeRetrievalMode::FullText,
                keyword_count: 10,
                top_k: 5,
                score_threshold_enabled: false,
                score_threshold: 0.15,
                rerank_enabled: false,
                embedding_model: None,
                rerank_model: None,
            })
            .await
            .expect("create")
            .name,
        "Docs"
    );
    assert_eq!(server.client().knowledge_dataset_delete("ds").await.expect("delete").id, "ds");
    let documents = server.client().knowledge_documents_list("ds").await.expect("docs");
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].status, KnowledgeDocumentStatus::Completed);
    assert_eq!(
        server
            .client()
            .knowledge_document_create(
                "ds",
                &KnowledgeDocumentCreateRequest {
                    name: "Readme".to_string(),
                    content: "hello".to_string(),
                    metadata: json!({}),
                    enabled: true,
                },
            )
            .await
            .expect("doc create")
            .id,
        "doc"
    );
    let chunks = server.client().knowledge_chunks_list("doc").await.expect("chunks");
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].id, "chunk");
    assert_eq!(
        server
            .client()
            .knowledge_document_reindex("doc", &KnowledgeDocumentReindexRequest { force: false })
            .await
            .expect("doc reindex")
            .id,
        "doc"
    );
    assert_eq!(
        server.client().knowledge_document_delete("doc").await.expect("doc delete").id,
        "doc"
    );
    let retrieved = server
        .client()
        .knowledge_retrieve(&KnowledgeRetrieveRequest {
            query: "hello".to_string(),
            dataset_ids: vec!["ds".to_string()],
            top_k: 5,
            score_threshold: Some(0.1),
            metadata_filter: None,
            include_context: true,
        })
        .await
        .expect("retrieve");
    assert_eq!(retrieved.chunks[0].id, "chunk");
    assert_eq!(retrieved.chunks[0].match_type, KnowledgeMatchType::FullText);

    assert_eq!(take_scoped_request(&server).path, "/v1/knowledge/status");
    assert_eq!(take_scoped_request(&server).path, "/v1/knowledge/datasets");
    assert_eq!(take_scoped_request(&server).path, "/v1/knowledge/datasets/ds");
    assert_eq!(take_scoped_request(&server).path, "/v1/knowledge/datasets");
    assert_eq!(take_scoped_request(&server).path, "/v1/knowledge/datasets/ds");
    assert_eq!(take_scoped_request(&server).path, "/v1/knowledge/datasets/ds/documents");
    assert_eq!(take_scoped_request(&server).path, "/v1/knowledge/datasets/ds/documents");
    assert_eq!(take_scoped_request(&server).path, "/v1/knowledge/documents/doc/chunks");
    let reindex_request = take_scoped_request(&server);
    assert_eq!(reindex_request.path, "/v1/knowledge/documents/doc/reindex");
    assert_eq!(reindex_request.body, json!({"force": false}));
    assert_eq!(take_scoped_request(&server).path, "/v1/knowledge/documents/doc");
    let retrieve_request = take_scoped_request(&server);
    assert_eq!(retrieve_request.path, "/v1/knowledge/retrieve");
    assert_eq!(retrieve_request.body["include_context"], json!(true));
    server.join();
}

#[tokio::test]
async fn knowledge_api_import_routes_use_typed_paths() {
    let server = test_support::server(vec![
        (
            200,
            json!({
                "document": document(),
                "filename": "guide.md",
                "mime_type": "text/markdown",
                "source_bytes": 19,
                "extracted_text_bytes": 18,
                "warnings": []
            }),
        ),
        (
            200,
            json!({
                "provider_id": "alibaba",
                "route_hint": "knowledge",
                "model": "text-embedding-v4",
                "dimensions": 1024,
                "api_key_configured": true,
                "vector_available": true,
                "hybrid_available": true,
                "retrieval_modes": ["full_text", "hybrid"],
                "notes": {}
            }),
        ),
        (
            200,
            json!([
                {
                    "id": "alibaba-intl",
                    "label": "Alibaba Bailian DashScope International",
                    "provider_id": "alibaba",
                    "model": "text-embedding-v4",
                    "dimensions": 1024,
                    "endpoint_url": "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/embeddings",
                    "api_key_env_vars": ["DASHSCOPE_API_KEY"],
                    "description": "OpenAI-compatible DashScope embedding endpoint outside China."
                },
                {
                    "id": "doubao-volcengine",
                    "label": "Doubao Volcengine OpenAI-compatible",
                    "provider_id": "custom:https://ark.cn-beijing.volces.com/api/v3",
                    "model": "doubao-embedding-text-240715",
                    "dimensions": 2048,
                    "endpoint_url": "https://ark.cn-beijing.volces.com/api/v3/embeddings",
                    "api_key_env_vars": ["ARK_API_KEY"],
                    "description": "Volcengine Ark embedding endpoint through the custom provider route."
                }
            ]),
        ),
        (
            200,
            json!({
                "ok": true,
                "provider_id": "alibaba",
                "model": "text-embedding-v4",
                "configured_dimensions": 1024,
                "observed_dimensions": 1024,
                "latency_ms": 120,
                "message": "embedding test succeeded"
            }),
        ),
    ]);

    let import = server
        .client()
        .knowledge_document_import(
            "ds",
            &KnowledgeDocumentImportRequest {
                filename: "guide.md".to_string(),
                mime_type: Some("text/markdown".to_string()),
                content_base64: "IyBoZWxsbyB2aWJl".to_string(),
                metadata: json!({"source": "desktop"}),
                enabled: true,
            },
        )
        .await
        .expect("import");
    assert_eq!(import.document.id, "doc");
    assert_eq!(import.extracted_text_bytes, 18);

    let status = server.client().knowledge_embedding_status().await.expect("embedding status");
    assert_eq!(status.provider_id, "alibaba");
    assert!(status.hybrid_available);

    let presets = server.client().knowledge_embedding_presets().await.expect("presets");
    assert_eq!(presets.len(), 2);
    assert!(presets[1].provider_id.starts_with("custom:https://"));

    let test = server
        .client()
        .knowledge_embedding_test(&KnowledgeEmbeddingTestRequest {
            provider_id: Some("alibaba".to_string()),
            model: Some("text-embedding-v4".to_string()),
            dimensions: Some(1024),
            api_key_override: Some("dashscope-secret".to_string()),
            sample_text: Some("hello".to_string()),
        })
        .await
        .expect("embedding test");
    assert!(test.ok);
    assert_eq!(test.observed_dimensions, Some(1024));

    let import_request = take_scoped_request(&server);
    assert_eq!(import_request.method, "POST");
    assert_eq!(import_request.path, "/v1/knowledge/datasets/ds/documents/import");
    assert_eq!(import_request.body["filename"], json!("guide.md"));
    assert_eq!(import_request.body["content_base64"], json!("IyBoZWxsbyB2aWJl"));

    let status_request = take_scoped_request(&server);
    assert_eq!(status_request.method, "GET");
    assert_eq!(status_request.path, "/v1/knowledge/embedding/status");

    let presets_request = take_scoped_request(&server);
    assert_eq!(presets_request.method, "GET");
    assert_eq!(presets_request.path, "/v1/knowledge/embedding/presets");

    let test_request = take_scoped_request(&server);
    assert_eq!(test_request.method, "POST");
    assert_eq!(test_request.path, "/v1/knowledge/embedding/test");
    assert_eq!(test_request.body["api_key_override"], json!("dashscope-secret"));
    server.join();
}

#[tokio::test]
async fn knowledge_api_async_import_and_job_routes() {
    let server = test_support::server(vec![
        (
            200,
            json!({
                "document": {
                    "id": "doc",
                    "dataset_id": "ds",
                    "name": "guide.md",
                    "metadata": {"source": "desktop"},
                    "enabled": true,
                    "status": "pending",
                    "chunk_count": 0,
                    "created_at_ms": 10,
                    "updated_at_ms": 10
                },
                "job": {
                    "id": "job",
                    "dataset_id": "ds",
                    "document_id": "doc",
                    "status": "queued",
                    "created_at_ms": 10,
                    "updated_at_ms": 10
                },
                "filename": "guide.md",
                "mime_type": "text/markdown",
                "source_bytes": 19,
                "warnings": []
            }),
        ),
        (
            200,
            json!({
                "id": "job",
                "dataset_id": "ds",
                "document_id": "doc",
                "status": "completed",
                "created_at_ms": 10,
                "updated_at_ms": 30,
                "started_at_ms": 20,
                "finished_at_ms": 30
            }),
        ),
    ]);

    let queued = server
        .client()
        .knowledge_document_import_async(
            "ds",
            &KnowledgeDocumentImportRequest {
                filename: "guide.md".to_string(),
                mime_type: Some("text/markdown".to_string()),
                content_base64: "IyBoZWxsbyB2aWJl".to_string(),
                metadata: json!({"source": "desktop"}),
                enabled: true,
            },
        )
        .await
        .expect("async import");
    assert_eq!(queued.document.status, KnowledgeDocumentStatus::Pending);
    assert_eq!(queued.job.status, KnowledgeIndexJobStatus::Queued);
    assert_eq!(queued.source_bytes, 19);

    let job = server.client().knowledge_index_job_get("job").await.expect("job");
    assert_eq!(job.status, KnowledgeIndexJobStatus::Completed);
    assert_eq!(job.finished_at_ms, Some(30));

    let import_request = take_scoped_request(&server);
    assert_eq!(import_request.method, "POST");
    assert_eq!(import_request.path, "/v1/knowledge/datasets/ds/documents/import-async");
    assert_eq!(import_request.body["filename"], json!("guide.md"));
    assert_eq!(import_request.body["content_base64"], json!("IyBoZWxsbyB2aWJl"));

    let job_request = take_scoped_request(&server);
    assert_eq!(job_request.method, "GET");
    assert_eq!(job_request.path, "/v1/knowledge/index-jobs/job");
    server.join();
}

#[tokio::test]
async fn knowledge_api_document_update_and_batch_routes() {
    let server = test_support::server(vec![
        (
            200,
            json!([{
                "id": "doc",
                "dataset_id": "ds",
                "name": "Archived guide",
                "metadata": {},
                "enabled": false,
                "status": "archived",
                "chunk_count": 2,
                "char_count": 120,
                "recall_count": 9,
                "archived_at_ms": 88,
                "created_at_ms": 1,
                "updated_at_ms": 2
            }]),
        ),
        (
            200,
            json!({
                "id": "doc",
                "dataset_id": "ds",
                "name": "Renamed guide",
                "metadata": {},
                "enabled": true,
                "status": "completed",
                "chunk_count": 2,
                "char_count": 120,
                "recall_count": 9,
                "created_at_ms": 1,
                "updated_at_ms": 3
            }),
        ),
        (
            200,
            json!([{
                "id": "doc",
                "dataset_id": "ds",
                "name": "Renamed guide",
                "metadata": {},
                "enabled": false,
                "status": "archived",
                "chunk_count": 2,
                "char_count": 120,
                "recall_count": 9,
                "archived_at_ms": 90,
                "created_at_ms": 1,
                "updated_at_ms": 4
            }]),
        ),
    ]);

    let filtered = server
        .client()
        .knowledge_documents_query(
            "ds",
            &KnowledgeDocumentListQuery { status: Some(KnowledgeDocumentStatus::Archived) },
        )
        .await
        .expect("filtered documents");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].status, KnowledgeDocumentStatus::Archived);
    assert_eq!(filtered[0].archived_at_ms, Some(88));

    let updated = server
        .client()
        .knowledge_document_update(
            "doc",
            &KnowledgeDocumentPatchRequest {
                name: Some("Renamed guide".to_string()),
                enabled: Some(true),
                archived: Some(false),
            },
        )
        .await
        .expect("document update");
    assert_eq!(updated.name, "Renamed guide");
    assert_eq!(updated.status, KnowledgeDocumentStatus::Completed);

    let batch = server
        .client()
        .knowledge_documents_batch_action(&KnowledgeDocumentsBatchActionRequest {
            document_ids: vec!["doc".to_string()],
            action: KnowledgeDocumentsBatchAction::Archive,
        })
        .await
        .expect("batch archive");
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0].status, KnowledgeDocumentStatus::Archived);
    assert_eq!(batch[0].archived_at_ms, Some(90));

    let list_request = take_scoped_request(&server);
    assert_eq!(list_request.method, "GET");
    assert_eq!(list_request.path, "/v1/knowledge/datasets/ds/documents?status=archived");

    let patch_request = take_scoped_request(&server);
    assert_eq!(patch_request.method, "PATCH");
    assert_eq!(patch_request.path, "/v1/knowledge/documents/doc");
    assert_eq!(
        patch_request.body,
        json!({
            "name": "Renamed guide",
            "enabled": true,
            "archived": false
        })
    );

    let batch_request = take_scoped_request(&server);
    assert_eq!(batch_request.method, "POST");
    assert_eq!(batch_request.path, "/v1/knowledge/documents/batch");
    assert_eq!(
        batch_request.body,
        json!({
            "document_ids": ["doc"],
            "action": "archive"
        })
    );
    server.join();
}
