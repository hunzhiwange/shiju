//! 验证知识库 DTO 的默认值、旧载荷兼容性、导入契约及敏感字段脱敏。

use super::knowledge::{
    KnowledgeCapabilitiesDto, KnowledgeChunkDto, KnowledgeChunkType, KnowledgeChunkingMode,
    KnowledgeDatasetCreateRequest, KnowledgeDatasetDto, KnowledgeDocumentCreateRequest,
    KnowledgeDocumentDto, KnowledgeDocumentImportLimitsDto, KnowledgeDocumentImportQueuedResponse,
    KnowledgeDocumentImportRequest, KnowledgeDocumentImportTypeDto, KnowledgeDocumentListQuery,
    KnowledgeDocumentPatchRequest, KnowledgeDocumentReindexRequest, KnowledgeDocumentStatus,
    KnowledgeDocumentsBatchAction, KnowledgeDocumentsBatchActionRequest,
    KnowledgeEmbeddingPresetDto, KnowledgeEmbeddingStatusDto, KnowledgeEmbeddingTestRequest,
    KnowledgeImportSupportDto, KnowledgeIndexJobDto, KnowledgeIndexJobStatus,
    KnowledgeIndexingMode, KnowledgeMatchType, KnowledgeRetrievalMode, KnowledgeRetrieveRequest,
};
use serde_json::json;

#[test]
fn knowledge_dataset_create_defaults_to_local_full_text() {
    let request: KnowledgeDatasetCreateRequest =
        serde_json::from_value(serde_json::json!({ "name": "Docs" })).expect("valid request");

    assert_eq!(request.indexing_mode, KnowledgeIndexingMode::Economy);
    assert_eq!(request.retrieval_mode, KnowledgeRetrievalMode::FullText);
    assert_eq!(request.chunking_mode, KnowledgeChunkingMode::General);
    assert_eq!(request.keyword_count, 10);
    assert_eq!(request.top_k, 10);
    assert!(!request.score_threshold_enabled);
    assert_eq!(request.score_threshold, 0.15);
    assert!(!request.rerank_enabled);
    assert_eq!(request.description, "");
}

#[test]
fn knowledge_retrieve_defaults_top_k() {
    let request: KnowledgeRetrieveRequest =
        serde_json::from_value(serde_json::json!({ "query": "refund policy" }))
            .expect("valid request");

    assert_eq!(request.top_k, 10);
    assert!(request.dataset_ids.is_empty());
    assert!(!request.include_context);

    let with_context: KnowledgeRetrieveRequest = serde_json::from_value(serde_json::json!({
        "query": "refund policy",
        "include_context": true
    }))
    .expect("valid request");
    assert!(with_context.include_context);
}

#[test]
fn knowledge_document_reindex_defaults_force_to_false() {
    let request: KnowledgeDocumentReindexRequest =
        serde_json::from_value(serde_json::json!({})).expect("valid request");

    assert!(!request.force);

    let forced: KnowledgeDocumentReindexRequest =
        serde_json::from_value(serde_json::json!({ "force": true })).expect("valid request");
    assert!(forced.force);
}

#[test]
fn knowledge_document_create_defaults_enabled_for_legacy_payloads() {
    let request: KnowledgeDocumentCreateRequest = serde_json::from_value(json!({
        "name": "Guide",
        "content": "hello"
    }))
    .expect("legacy create request");

    assert!(request.enabled);
    assert_eq!(request.metadata, json!(null));
}

#[test]
fn knowledge_dto_defaults_keep_legacy_payloads_compatible() {
    let dataset: KnowledgeDatasetDto = serde_json::from_value(json!({
        "id": "ds",
        "name": "Docs",
        "description": "",
        "document_count": 1,
        "chunk_count": 2,
        "created_at_ms": 1,
        "updated_at_ms": 2
    }))
    .expect("legacy dataset");
    assert_eq!(dataset.capabilities, KnowledgeCapabilitiesDto::default());
    assert!(dataset.capabilities.full_text);

    let document: KnowledgeDocumentDto = serde_json::from_value(json!({
        "id": "doc",
        "dataset_id": "ds",
        "name": "Readme",
        "metadata": {},
        "enabled": true,
        "chunk_count": 2,
        "created_at_ms": 1,
        "updated_at_ms": 2
    }))
    .expect("legacy document");
    assert_eq!(document.status, KnowledgeDocumentStatus::Completed);
    assert_eq!(document.error_message, "");
    assert_eq!(document.content_hash, "");

    let chunk: KnowledgeChunkDto = serde_json::from_value(json!({
        "id": "chunk",
        "dataset_id": "ds",
        "document_id": "doc",
        "title": "Readme",
        "content": "hello",
        "metadata": {},
        "score": 0.9
    }))
    .expect("legacy chunk");
    assert_eq!(chunk.ordinal, 0);
    assert_eq!(chunk.chunk_type, KnowledgeChunkType::Text);
    assert_eq!(chunk.start_at, 0);
    assert_eq!(chunk.end_at, 0);
    assert_eq!(chunk.match_type, KnowledgeMatchType::FullText);
    assert!(chunk.parent_chunk_id.is_none());
    assert!(chunk.prev_chunk_id.is_none());
    assert!(chunk.next_chunk_id.is_none());
    assert!(chunk.matched_content.is_none());
    assert!(chunk.context.is_none());
}

#[test]
fn knowledge_dto_accepts_explicit_capability_status_and_chunk_fields() {
    let dataset: KnowledgeDatasetDto = serde_json::from_value(json!({
        "id": "ds",
        "name": "Docs",
        "description": "",
        "capabilities": {
            "full_text": true,
            "vector": true,
            "hybrid": true,
            "rerank": false
        },
        "document_count": 1,
        "chunk_count": 2,
        "created_at_ms": 1,
        "updated_at_ms": 2
    }))
    .expect("dataset");
    assert!(dataset.capabilities.vector);
    assert!(dataset.capabilities.hybrid);

    let document: KnowledgeDocumentDto = serde_json::from_value(json!({
        "id": "doc",
        "dataset_id": "ds",
        "name": "Readme",
        "metadata": {},
        "enabled": false,
        "status": "disabled",
        "error_message": "manual pause",
        "content_hash": "abc",
        "chunk_count": 2,
        "created_at_ms": 1,
        "updated_at_ms": 2
    }))
    .expect("document");
    assert_eq!(document.status, KnowledgeDocumentStatus::Disabled);
    assert_eq!(document.error_message, "manual pause");
    assert_eq!(document.content_hash, "abc");

    let chunk: KnowledgeChunkDto = serde_json::from_value(json!({
        "id": "chunk",
        "dataset_id": "ds",
        "document_id": "doc",
        "title": "Readme",
        "content": "hello",
        "metadata": {},
        "score": 0.9,
        "ordinal": 3,
        "chunk_type": "qa",
        "start_at": 4,
        "end_at": 9,
        "parent_chunk_id": "parent",
        "prev_chunk_id": "prev",
        "next_chunk_id": "next",
        "matched_content": "hello",
        "match_type": "nearby",
        "context": "section"
    }))
    .expect("chunk");
    assert_eq!(chunk.ordinal, 3);
    assert_eq!(chunk.chunk_type, KnowledgeChunkType::Qa);
    assert_eq!(chunk.start_at, 4);
    assert_eq!(chunk.end_at, 9);
    assert_eq!(chunk.parent_chunk_id.as_deref(), Some("parent"));
    assert_eq!(chunk.prev_chunk_id.as_deref(), Some("prev"));
    assert_eq!(chunk.next_chunk_id.as_deref(), Some("next"));
    assert_eq!(chunk.matched_content.as_deref(), Some("hello"));
    assert_eq!(chunk.match_type, KnowledgeMatchType::Nearby);
    assert_eq!(chunk.context.as_deref(), Some("section"));
}

#[test]
fn knowledge_document_contract_accepts_stats_archive_and_patch_requests() {
    let document: KnowledgeDocumentDto = serde_json::from_value(json!({
        "id": "doc",
        "dataset_id": "ds",
        "name": "Readme",
        "metadata": {},
        "enabled": false,
        "status": "archived",
        "error_message": "",
        "content_hash": "abc",
        "chunk_count": 2,
        "char_count": 120,
        "recall_count": 9,
        "archived_at_ms": 88,
        "created_at_ms": 1,
        "updated_at_ms": 2
    }))
    .expect("document with stats");
    assert_eq!(document.status, KnowledgeDocumentStatus::Archived);
    assert_eq!(document.char_count, 120);
    assert_eq!(document.recall_count, 9);
    assert_eq!(document.archived_at_ms, Some(88));

    let legacy_document: KnowledgeDocumentDto = serde_json::from_value(json!({
        "id": "doc",
        "dataset_id": "ds",
        "name": "Readme",
        "metadata": {},
        "enabled": true,
        "chunk_count": 2,
        "created_at_ms": 1,
        "updated_at_ms": 2
    }))
    .expect("legacy document");
    assert_eq!(legacy_document.char_count, 0);
    assert_eq!(legacy_document.recall_count, 0);
    assert_eq!(legacy_document.archived_at_ms, None);

    let query: KnowledgeDocumentListQuery =
        serde_json::from_value(json!({ "status": "archived" })).expect("status query");
    assert_eq!(query.status, Some(KnowledgeDocumentStatus::Archived));

    let patch: KnowledgeDocumentPatchRequest = serde_json::from_value(json!({
        "name": "Renamed",
        "enabled": true,
        "archived": false
    }))
    .expect("patch request");
    assert_eq!(patch.name.as_deref(), Some("Renamed"));
    assert_eq!(patch.enabled, Some(true));
    assert_eq!(patch.archived, Some(false));

    let batch: KnowledgeDocumentsBatchActionRequest = serde_json::from_value(json!({
        "document_ids": ["doc", "doc-2"],
        "action": "archive"
    }))
    .expect("batch request");
    assert_eq!(batch.document_ids, vec!["doc".to_string(), "doc-2".to_string()]);
    assert_eq!(batch.action, KnowledgeDocumentsBatchAction::Archive);

    assert_eq!(
        serde_json::to_value(KnowledgeDocumentStatus::Archived).expect("archived status"),
        json!("archived")
    );
}

#[test]
fn knowledge_import_contract_serializes_file_preset_status_and_redacts_key() {
    let request = KnowledgeDocumentImportRequest {
        filename: "guide.md".to_string(),
        mime_type: Some("text/markdown".to_string()),
        content_base64: "IyBIZWxsbyBWaWJlV2luZG93".to_string(),
        metadata: json!({"source": "desktop"}),
        enabled: true,
    };

    assert_eq!(
        serde_json::to_value(&request).expect("request"),
        json!({
            "filename": "guide.md",
            "mime_type": "text/markdown",
            "content_base64": "IyBIZWxsbyBWaWJlV2luZG93",
            "metadata": {"source": "desktop"},
            "enabled": true
        })
    );

    let support = KnowledgeImportSupportDto {
        limits: KnowledgeDocumentImportLimitsDto {
            max_file_bytes: 20 * 1024 * 1024,
            max_extracted_text_bytes: 5 * 1024 * 1024,
        },
        types: vec![
            KnowledgeDocumentImportTypeDto {
                id: "markdown".to_string(),
                label: "Markdown".to_string(),
                extensions: vec!["md".to_string(), "markdown".to_string()],
                mime_types: vec!["text/markdown".to_string()],
                supported: true,
                unsupported_reason: None,
            },
            KnowledgeDocumentImportTypeDto {
                id: "docx".to_string(),
                label: "Word document".to_string(),
                extensions: vec!["docx".to_string()],
                mime_types: vec![
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                        .to_string(),
                ],
                supported: false,
                unsupported_reason: Some("DOCX parsing is not available".to_string()),
            },
        ],
    };
    assert!(support.types[0].supported);
    assert_eq!(
        support.types[1].unsupported_reason.as_deref(),
        Some("DOCX parsing is not available")
    );

    let presets = [
        KnowledgeEmbeddingPresetDto {
            id: "alibaba-intl".to_string(),
            label: "Alibaba Bailian DashScope International".to_string(),
            provider_id: "alibaba".to_string(),
            model: "text-embedding-v4".to_string(),
            dimensions: 1024,
            endpoint_url: Some(
                "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/embeddings".to_string(),
            ),
            api_key_env_vars: vec!["DASHSCOPE_API_KEY".to_string()],
            description: "OpenAI-compatible DashScope embedding endpoint outside China."
                .to_string(),
        },
        KnowledgeEmbeddingPresetDto {
            id: "alibaba-cn".to_string(),
            label: "Alibaba Bailian DashScope China".to_string(),
            provider_id: "alibaba-cn".to_string(),
            model: "text-embedding-v4".to_string(),
            dimensions: 1024,
            endpoint_url: Some(
                "https://dashscope.aliyuncs.com/compatible-mode/v1/embeddings".to_string(),
            ),
            api_key_env_vars: vec!["DASHSCOPE_API_KEY".to_string()],
            description: "OpenAI-compatible DashScope embedding endpoint in China.".to_string(),
        },
        KnowledgeEmbeddingPresetDto {
            id: "doubao-volcengine".to_string(),
            label: "Doubao Volcengine OpenAI-compatible".to_string(),
            provider_id: "custom:https://ark.cn-beijing.volces.com/api/v3".to_string(),
            model: "doubao-embedding-text-240715".to_string(),
            dimensions: 2048,
            endpoint_url: Some("https://ark.cn-beijing.volces.com/api/v3/embeddings".to_string()),
            api_key_env_vars: vec!["ARK_API_KEY".to_string()],
            description: "Volcengine Ark embedding endpoint through the custom provider route."
                .to_string(),
        },
    ];
    assert_eq!(presets[0].provider_id, "alibaba");
    assert_eq!(presets[1].provider_id, "alibaba-cn");
    assert!(presets[2].provider_id.starts_with("custom:https://"));

    let status = KnowledgeEmbeddingStatusDto {
        provider_id: "alibaba".to_string(),
        route_hint: Some("knowledge".to_string()),
        model: "text-embedding-v4".to_string(),
        dimensions: 1024,
        api_key_configured: true,
        vector_available: true,
        hybrid_available: true,
        retrieval_modes: vec![KnowledgeRetrievalMode::FullText, KnowledgeRetrievalMode::Hybrid],
        notes: Default::default(),
    };
    assert_eq!(status.retrieval_modes[1], KnowledgeRetrievalMode::Hybrid);

    let test_request = KnowledgeEmbeddingTestRequest {
        provider_id: Some("alibaba".to_string()),
        model: Some("text-embedding-v4".to_string()),
        dimensions: Some(1024),
        api_key_override: Some("dashscope-secret".to_string()),
        sample_text: Some("hello".to_string()),
    };
    let debug = format!("{test_request:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("dashscope-secret"));
}

#[test]
fn knowledge_async_import_contract_serializes_job_statuses() {
    let statuses = [
        (KnowledgeIndexJobStatus::Queued, "queued"),
        (KnowledgeIndexJobStatus::Processing, "processing"),
        (KnowledgeIndexJobStatus::Completed, "completed"),
        (KnowledgeIndexJobStatus::Failed, "failed"),
    ];

    for (status, expected) in statuses {
        assert_eq!(serde_json::to_value(status).expect("status"), json!(expected));
    }

    let response = KnowledgeDocumentImportQueuedResponse {
        document: KnowledgeDocumentDto {
            id: "doc".to_string(),
            dataset_id: "ds".to_string(),
            name: "guide.md".to_string(),
            metadata: json!({"source": "desktop"}),
            enabled: true,
            status: KnowledgeDocumentStatus::Pending,
            error_message: String::new(),
            content_hash: String::new(),
            chunk_count: 0,
            char_count: 0,
            recall_count: 0,
            archived_at_ms: None,
            created_at_ms: 10,
            updated_at_ms: 10,
        },
        job: KnowledgeIndexJobDto {
            id: "job".to_string(),
            dataset_id: "ds".to_string(),
            document_id: "doc".to_string(),
            status: KnowledgeIndexJobStatus::Queued,
            error_message: String::new(),
            created_at_ms: 10,
            updated_at_ms: 10,
            started_at_ms: None,
            finished_at_ms: None,
        },
        filename: "guide.md".to_string(),
        mime_type: Some("text/markdown".to_string()),
        source_bytes: 19,
        warnings: Vec::new(),
    };

    assert_eq!(
        serde_json::to_value(&response).expect("response"),
        json!({
            "document": {
                "id": "doc",
                "dataset_id": "ds",
                "name": "guide.md",
                "metadata": {"source": "desktop"},
                "enabled": true,
                "status": "pending",
                "chunk_count": 0,
                "char_count": 0,
                "recall_count": 0,
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
        })
    );

    let job: KnowledgeIndexJobDto = serde_json::from_value(json!({
        "id": "job",
        "dataset_id": "ds",
        "document_id": "doc",
        "status": "failed",
        "error_message": "embedding failed",
        "created_at_ms": 10,
        "updated_at_ms": 30,
        "started_at_ms": 20,
        "finished_at_ms": 30
    }))
    .expect("job");
    assert_eq!(job.status, KnowledgeIndexJobStatus::Failed);
    assert_eq!(job.error_message, "embedding failed");
    assert_eq!(job.started_at_ms, Some(20));
    assert_eq!(job.finished_at_ms, Some(30));
}
