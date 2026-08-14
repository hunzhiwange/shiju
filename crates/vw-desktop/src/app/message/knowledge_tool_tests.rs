//! 覆盖知识库工具消息处理的本地状态转换与参数校验。

use super::{
    KnowledgeToolMessage, build_batch_document_action_request, build_document_patch_request,
    document_status_from_index_job, import_batch_success_message, import_request_from_file,
    import_success_message, looks_sensitive_token, mime_type_for_filename, parse_score_threshold,
    parse_usize_input, sanitize_document_page_size, sanitize_sensitive_display_text,
    selected_dataset_embedding_model, trim_to_option, update,
};
use crate::app::{
    App,
    state::{
        KnowledgeCreateDatasetTab, KnowledgeDatasetEmbeddingSelection, KnowledgeDetailTab,
        KnowledgeDocumentInputTab,
    },
};
use iced::widget::text_editor;
use serde_json::Value;
use std::collections::BTreeMap;
use vw_gateway_client::vw_api_types::knowledge::{
    KnowledgeDocumentPatchRequest, KnowledgeDocumentStatus, KnowledgeDocumentsBatchAction,
    KnowledgeDocumentsBatchActionRequest,
};
use vw_gateway_client::{
    KnowledgeCapabilitiesDto, KnowledgeChunkDto, KnowledgeChunkType, KnowledgeChunkingMode,
    KnowledgeDatasetDto, KnowledgeDocumentDto, KnowledgeDocumentImportQueuedResponse,
    KnowledgeEmbeddingPresetDto, KnowledgeEmbeddingStatusDto, KnowledgeEmbeddingTestResponse,
    KnowledgeIndexJobDto, KnowledgeIndexJobStatus, KnowledgeIndexingMode, KnowledgeMatchType,
    KnowledgeRetrievalMode, KnowledgeRetrieveResponse, KnowledgeRuntimeStatus,
};

fn test_app() -> App {
    App::new().0
}

fn dataset(id: &str, top_k: usize, threshold_enabled: bool, threshold: f64) -> KnowledgeDatasetDto {
    KnowledgeDatasetDto {
        id: id.to_string(),
        name: format!("dataset {id}"),
        description: "desc".to_string(),
        capabilities: KnowledgeCapabilitiesDto::default(),
        chunking_mode: KnowledgeChunkingMode::General,
        indexing_mode: KnowledgeIndexingMode::HighQuality,
        retrieval_mode: KnowledgeRetrievalMode::Hybrid,
        keyword_count: 12,
        top_k,
        score_threshold_enabled: threshold_enabled,
        score_threshold: threshold,
        rerank_enabled: false,
        embedding_model: None,
        rerank_model: None,
        document_count: 0,
        chunk_count: 0,
        created_at_ms: 1,
        updated_at_ms: 2,
    }
}

fn document(id: &str, dataset_id: &str) -> KnowledgeDocumentDto {
    KnowledgeDocumentDto {
        id: id.to_string(),
        dataset_id: dataset_id.to_string(),
        name: format!("doc {id}"),
        metadata: Value::Null,
        enabled: true,
        status: KnowledgeDocumentStatus::Completed,
        error_message: String::new(),
        content_hash: String::new(),
        chunk_count: 1,
        char_count: 0,
        recall_count: 0,
        archived_at_ms: None,
        created_at_ms: 1,
        updated_at_ms: 2,
    }
}

fn index_job(id: &str, document_id: &str, status: KnowledgeIndexJobStatus) -> KnowledgeIndexJobDto {
    KnowledgeIndexJobDto {
        id: id.to_string(),
        dataset_id: "ds".to_string(),
        document_id: document_id.to_string(),
        status,
        error_message: String::new(),
        created_at_ms: 1,
        updated_at_ms: 2,
        started_at_ms: None,
        finished_at_ms: None,
    }
}

fn queued_response(
    filename: &str,
    document_id: &str,
    job_id: &str,
) -> KnowledgeDocumentImportQueuedResponse {
    let mut pending = document(document_id, "ds");
    pending.name = filename.to_string();
    pending.status = KnowledgeDocumentStatus::Pending;
    pending.chunk_count = 0;
    KnowledgeDocumentImportQueuedResponse {
        document: pending,
        job: index_job(job_id, document_id, KnowledgeIndexJobStatus::Queued),
        filename: filename.to_string(),
        mime_type: Some("text/markdown".to_string()),
        source_bytes: 12,
        warnings: vec![],
    }
}

fn chunk(id: &str) -> KnowledgeChunkDto {
    KnowledgeChunkDto {
        id: id.to_string(),
        dataset_id: "ds".to_string(),
        document_id: "doc".to_string(),
        title: "title".to_string(),
        content: "content".to_string(),
        metadata: Value::Null,
        score: Some(0.9),
        ordinal: 0,
        chunk_type: KnowledgeChunkType::Text,
        start_at: 0,
        end_at: 0,
        parent_chunk_id: None,
        prev_chunk_id: None,
        next_chunk_id: None,
        matched_content: None,
        match_type: KnowledgeMatchType::FullText,
        context: None,
    }
}

#[test]
fn knowledge_file_import_state_tracks_selection_success_and_failure() {
    let mut app = test_app();
    app.knowledge.selected_dataset_id = Some("ds".to_string());

    let selected = crate::app::state::KnowledgeFileImportDraft {
        filename: "guide.md".to_string(),
        mime_type: Some("text/markdown".to_string()),
        byte_len: 12,
        content_base64: "IyBoZWxsbyB2aWJl".to_string(),
    };

    let _ = update(&mut app, KnowledgeToolMessage::ImportFileSelected(Ok(vec![selected.clone()])));

    assert_eq!(app.knowledge.file_imports.as_slice(), &[selected]);
    assert!(!app.knowledge.selecting_import_file);
    assert_eq!(app.knowledge.file_imports[0].byte_len, 12);
    assert!(app.knowledge.document_content_editor.text().is_empty());

    let _ = update(&mut app, KnowledgeToolMessage::ImportSelectedFile);
    assert!(app.knowledge.importing_file);

    let response = queued_response("guide.md", "doc", "job");
    let _ =
        update(&mut app, KnowledgeToolMessage::FileImported("ds".to_string(), Ok(vec![response])));

    assert!(!app.knowledge.importing_file);
    assert!(app.knowledge.file_imports.is_empty());
    assert!(app.knowledge.loading_documents);
    assert!(!app.knowledge.loading_chunks);
    assert_eq!(app.knowledge.import_polling_jobs.len(), 1);
    assert_eq!(app.knowledge.notification.as_deref(), Some("文件已入队：guide.md，后台入库中"));
    assert!(!app.knowledge.notification_is_error);
    assert_eq!(
        app.active_toast.as_ref().map(|toast| toast.message.as_str()),
        Some("文件已入队：guide.md，后台入库中")
    );

    app.knowledge.importing_file = true;
    let _ = update(
        &mut app,
        KnowledgeToolMessage::FileImported("ds".to_string(), Err("不支持 DOCX 文件".to_string())),
    );

    assert!(!app.knowledge.importing_file);
    assert_eq!(app.knowledge.notification.as_deref(), Some("不支持 DOCX 文件"));
    assert!(app.knowledge.notification_is_error);
    assert_eq!(
        app.active_toast.as_ref().map(|toast| toast.message.as_str()),
        Some("不支持 DOCX 文件")
    );
}

#[test]
fn knowledge_file_import_enqueues_jobs_and_tracks_polling_state() {
    let mut app = test_app();
    app.knowledge.selected_dataset_id = Some("ds".to_string());

    let files = vec![
        crate::app::state::KnowledgeFileImportDraft {
            filename: "guide.md".to_string(),
            mime_type: Some("text/markdown".to_string()),
            byte_len: 12,
            content_base64: "IyBoZWxsbyB2aWJl".to_string(),
        },
        crate::app::state::KnowledgeFileImportDraft {
            filename: "notes.md".to_string(),
            mime_type: Some("text/markdown".to_string()),
            byte_len: 8,
            content_base64: "bm90ZXM=".to_string(),
        },
    ];

    let _ = update(&mut app, KnowledgeToolMessage::ImportFileSelected(Ok(files)));
    let _ = update(&mut app, KnowledgeToolMessage::ImportSelectedFile);

    assert!(app.knowledge.importing_file);

    let _ = update(
        &mut app,
        KnowledgeToolMessage::FileImported(
            "ds".to_string(),
            Ok(vec![
                queued_response("guide.md", "doc-a", "job-a"),
                queued_response("notes.md", "doc-b", "job-b"),
            ]),
        ),
    );

    assert!(!app.knowledge.importing_file);
    assert!(app.knowledge.file_imports.is_empty());
    assert_eq!(app.knowledge.import_polling_jobs.len(), 2);
    assert_eq!(app.knowledge.import_polling_jobs[0].job_id, "job-a");
    assert_eq!(app.knowledge.import_polling_jobs[1].status, KnowledgeIndexJobStatus::Queued);
    assert!(app.knowledge.loading_documents);
    assert!(app.knowledge.loading_datasets);
    assert_eq!(app.knowledge.notification.as_deref(), Some("已入队 2 个文件，后台入库中"));
    assert_eq!(
        app.active_toast.as_ref().map(|toast| toast.message.as_str()),
        Some("已入队 2 个文件，后台入库中")
    );

    let _ = update(
        &mut app,
        KnowledgeToolMessage::ImportJobPolled(Ok(index_job(
            "job-a",
            "doc-a",
            KnowledgeIndexJobStatus::Processing,
        ))),
    );

    assert_eq!(app.knowledge.import_polling_jobs.len(), 2);
    assert_eq!(app.knowledge.import_polling_jobs[0].status, KnowledgeIndexJobStatus::Processing);

    let mut completed = index_job("job-a", "doc-a", KnowledgeIndexJobStatus::Completed);
    completed.finished_at_ms = Some(3);
    let _ = update(&mut app, KnowledgeToolMessage::ImportJobPolled(Ok(completed)));

    assert_eq!(app.knowledge.import_polling_jobs.len(), 1);
    assert_eq!(app.knowledge.import_polling_jobs[0].job_id, "job-b");
    assert!(app.knowledge.loading_documents);

    let mut failed = index_job("job-b", "doc-b", KnowledgeIndexJobStatus::Failed);
    failed.error_message = "解析失败".to_string();
    failed.finished_at_ms = Some(4);
    let _ = update(&mut app, KnowledgeToolMessage::ImportJobPolled(Ok(failed)));

    assert!(app.knowledge.import_polling_jobs.is_empty());
    assert_eq!(app.knowledge.notification.as_deref(), Some("notes.md 入库失败：解析失败"));
    assert!(app.knowledge.notification_is_error);
    assert_eq!(
        app.active_toast.as_ref().map(|toast| toast.message.as_str()),
        Some("notes.md 入库失败：解析失败")
    );
}

#[test]
fn knowledge_drag_drop_upload_starts_file_loading_and_clears_global_buffers() {
    let mut app = test_app();
    app.knowledge.selected_dataset_id = Some("ds".to_string());
    app.pending_drop_file_paths = vec!["/tmp/guide.md".to_string(), "/tmp/spec.pdf".to_string()];
    app.pending_drop_file_position = Some((3, 4));
    app.knowledge.file_drop_hovered = true;

    let _ = update(&mut app, KnowledgeToolMessage::ImportDroppedFiles);

    assert!(app.knowledge.selecting_import_file);
    assert!(!app.knowledge.file_drop_hovered);
    assert!(app.pending_drop_file_paths.is_empty());
    assert_eq!(app.pending_drop_file_position, None);
    assert!(app.dragging_file_paths.is_empty());
}

#[test]
fn delete_confirmation_state_opens_and_closes_for_dataset_and_document() {
    let mut app = test_app();
    app.knowledge.datasets = vec![dataset("ds", 10, false, 0.15)];
    app.knowledge.selected_dataset_id = Some("ds".to_string());
    app.knowledge.documents = vec![document("doc-a", "ds")];

    let _ = update(&mut app, KnowledgeToolMessage::OpenDeleteDatasetConfirm);
    assert_eq!(app.knowledge.pending_delete_dataset_id.as_deref(), Some("ds"));
    assert!(app.knowledge.pending_delete_document_id.is_none());

    let _ = update(&mut app, KnowledgeToolMessage::OpenDeleteDocumentConfirm("doc-a".to_string()));
    assert!(app.knowledge.pending_delete_dataset_id.is_none());
    assert_eq!(app.knowledge.pending_delete_document_id.as_deref(), Some("doc-a"));

    let _ = update(&mut app, KnowledgeToolMessage::CloseDeleteConfirm);
    assert!(app.knowledge.pending_delete_dataset_id.is_none());
    assert!(app.knowledge.pending_delete_document_id.is_none());
}

#[test]
fn parse_helpers_trim_clamp_and_reject_non_finite_thresholds() {
    assert_eq!(parse_usize_input(" 42 ", 7), 42);
    assert_eq!(parse_usize_input("bad", 7), 7);
    assert_eq!(parse_score_threshold("0.42"), Some(0.42));
    assert_eq!(parse_score_threshold("-1"), Some(0.0));
    assert_eq!(parse_score_threshold("9"), Some(1.0));
    assert_eq!(parse_score_threshold("nan"), None);
    assert_eq!(trim_to_option("  model  "), Some("model".to_string()));
    assert_eq!(trim_to_option("  "), None);
    assert_eq!(sanitize_document_page_size(50), 50);
    assert_eq!(sanitize_document_page_size(15), 10);
}

#[test]
fn update_mutates_simple_inputs_and_mode_constraints() {
    let mut app = test_app();

    let _ = update(&mut app, KnowledgeToolMessage::SelectTab(KnowledgeDetailTab::Retrieval));
    let _ = update(
        &mut app,
        KnowledgeToolMessage::SelectDocumentInputTab(KnowledgeDocumentInputTab::Text),
    );
    let _ = update(&mut app, KnowledgeToolMessage::ImportDropHoverChanged(true));
    let _ = update(&mut app, KnowledgeToolMessage::DatasetSearchChanged("data".to_string()));
    app.knowledge.document_page = 3;
    let _ = update(&mut app, KnowledgeToolMessage::DocumentSearchChanged("doc".to_string()));
    let _ = update(&mut app, KnowledgeToolMessage::DocumentPageChanged(2));
    let _ = update(&mut app, KnowledgeToolMessage::DocumentPageSizeChanged(30));
    let _ = update(&mut app, KnowledgeToolMessage::DatasetNameChanged("name".to_string()));
    let _ = update(&mut app, KnowledgeToolMessage::DatasetDescriptionChanged("desc".to_string()));
    let _ = update(
        &mut app,
        KnowledgeToolMessage::DatasetChunkingModeChanged(KnowledgeChunkingMode::Qa),
    );
    let _ = update(
        &mut app,
        KnowledgeToolMessage::DatasetRetrievalModeChanged(KnowledgeRetrievalMode::Vector),
    );
    let _ = update(&mut app, KnowledgeToolMessage::RetrieveIncludeContextChanged(true));

    assert_eq!(app.knowledge.active_tab, KnowledgeDetailTab::Retrieval);
    assert_eq!(app.knowledge.document_input_tab, KnowledgeDocumentInputTab::Text);
    assert!(app.knowledge.file_drop_hovered);
    assert_eq!(app.knowledge.dataset_search_query, "data");
    assert_eq!(app.knowledge.document_search_query, "doc");
    assert_eq!(app.knowledge.document_page, 1);
    assert_eq!(app.knowledge.document_page_size, 30);
    assert_eq!(app.knowledge.dataset_name_input, "name");
    assert_eq!(app.knowledge.dataset_description_input, "desc");
    assert_eq!(app.knowledge.dataset_chunking_mode, KnowledgeChunkingMode::Qa);
    assert_eq!(app.knowledge.dataset_indexing_mode, KnowledgeIndexingMode::HighQuality);
    assert_eq!(app.knowledge.dataset_retrieval_mode, KnowledgeRetrievalMode::Vector);
    assert!(app.knowledge.retrieve_include_context);

    app.knowledge.dataset_rerank_enabled = true;
    app.knowledge.dataset_score_threshold_enabled = true;
    let _ = update(
        &mut app,
        KnowledgeToolMessage::DatasetIndexingModeChanged(KnowledgeIndexingMode::Economy),
    );

    assert_eq!(app.knowledge.dataset_retrieval_mode, KnowledgeRetrievalMode::FullText);
    assert!(!app.knowledge.dataset_rerank_enabled);
    assert!(!app.knowledge.dataset_score_threshold_enabled);

    let _ = update(&mut app, KnowledgeToolMessage::DatasetRerankEnabledChanged(true));
    assert_eq!(app.knowledge.dataset_rerank_model_input, "local-rerank-v1");

    let _ = update(&mut app, KnowledgeToolMessage::OpenCreateDatasetModal);
    assert!(app.knowledge.create_dataset_modal_open);

    let _ = update(&mut app, KnowledgeToolMessage::CloseCreateDatasetModal);
    assert!(!app.knowledge.create_dataset_modal_open);

    app.knowledge.selected_dataset_id = Some("ds".to_string());
    let _ = update(&mut app, KnowledgeToolMessage::OpenDeleteDatasetConfirm);
    assert_eq!(app.knowledge.pending_delete_dataset_id.as_deref(), Some("ds"));

    app.knowledge.document_preview_open = true;
    let _ = update(&mut app, KnowledgeToolMessage::SelectTab(KnowledgeDetailTab::AddDocument));
    assert!(!app.knowledge.document_preview_open);

    let _ = update(
        &mut app,
        KnowledgeToolMessage::SelectDocumentInputTab(KnowledgeDocumentInputTab::Files),
    );
    assert!(!app.knowledge.file_drop_hovered);

    let _ = update(&mut app, KnowledgeToolMessage::DocumentPageChanged(2));
    assert_eq!(app.knowledge.document_page, 2);
}

#[test]
fn update_handles_chunk_loading_selection_and_stale_results() {
    let mut app = test_app();
    let _ = update(&mut app, KnowledgeToolMessage::LoadDocumentChunks("doc".to_string()));
    assert!(app.knowledge.document_preview_open);
    assert_eq!(app.knowledge.selected_document_id.as_deref(), Some("doc"));
    assert!(app.knowledge.loading_chunks);

    app.knowledge.selected_document_id = Some("doc".to_string());
    app.knowledge.loading_chunks = true;

    let _ = update(
        &mut app,
        KnowledgeToolMessage::ChunksLoaded("other".to_string(), Ok(vec![chunk("stale")])),
    );
    assert!(app.knowledge.loaded_chunks.is_empty());
    assert!(app.knowledge.loading_chunks);

    let _ = update(
        &mut app,
        KnowledgeToolMessage::ChunksLoaded("doc".to_string(), Ok(vec![chunk("fresh")])),
    );
    assert!(!app.knowledge.loading_chunks);
    assert_eq!(app.knowledge.loaded_chunks.len(), 1);
    assert_eq!(app.knowledge.selected_chunk_id.as_deref(), Some("fresh"));

    let _ = update(&mut app, KnowledgeToolMessage::SelectChunk("fresh".to_string()));
    assert_eq!(app.knowledge.selected_chunk_id.as_deref(), Some("fresh"));

    app.knowledge.loading_chunks = true;
    let _ = update(
        &mut app,
        KnowledgeToolMessage::ChunksLoaded("doc".to_string(), Err("chunk load failed".to_string())),
    );
    assert!(!app.knowledge.loading_chunks);
    assert_eq!(app.knowledge.notification.as_deref(), Some("chunk load failed"));
    assert!(app.knowledge.notification_is_error);

    let _ = update(&mut app, KnowledgeToolMessage::CloseDocumentPreview);
    assert!(!app.knowledge.document_preview_open);
    assert!(app.knowledge.selected_document_id.is_none());
    assert!(app.knowledge.loaded_chunks.is_empty());
}

#[test]
fn update_reindex_completion_refreshes_document_state_and_chunks() {
    let mut app = test_app();
    app.knowledge.selected_dataset_id = Some("ds".to_string());
    app.knowledge.documents = vec![document("doc", "ds")];
    app.knowledge.loaded_chunks = vec![chunk("old")];
    app.knowledge.reindexing_document_id = Some("doc".to_string());

    let mut reindexed = document("doc", "ds");
    reindexed.status = KnowledgeDocumentStatus::Completed;
    reindexed.chunk_count = 3;

    let _ = update(&mut app, KnowledgeToolMessage::ReindexFinished(Ok(reindexed)));

    assert!(app.knowledge.reindexing_document_id.is_none());
    assert_eq!(app.knowledge.documents[0].chunk_count, 3);
    assert_eq!(app.knowledge.selected_document_id.as_deref(), Some("doc"));
    assert!(app.knowledge.loading_documents);
    assert!(app.knowledge.loading_chunks);
    assert_eq!(app.knowledge.notification.as_deref(), Some("文档已重新入库"));
}

#[test]
fn update_handles_load_success_error_and_stale_documents() {
    let mut app = test_app();

    let mut notes = BTreeMap::new();
    notes.insert("engine".to_string(), "ok".to_string());
    let status = KnowledgeRuntimeStatus {
        full_text: true,
        vector: true,
        hybrid: true,
        rerank: false,
        notes,
    };

    app.knowledge.loading_status = true;
    let _ = update(&mut app, KnowledgeToolMessage::StatusLoaded(Ok(status.clone())));
    assert!(!app.knowledge.loading_status);
    assert_eq!(app.knowledge.runtime_status, Some(status));

    app.knowledge.loading_status = true;
    let _ = update(&mut app, KnowledgeToolMessage::StatusLoaded(Err("offline".to_string())));
    assert_eq!(app.knowledge.notification.as_deref(), Some("offline"));
    assert!(app.knowledge.notification_is_error);

    let _ = update(
        &mut app,
        KnowledgeToolMessage::DatasetsLoaded(Ok(vec![dataset("ds1", 99, true, 0.234)])),
    );
    assert_eq!(app.knowledge.selected_dataset_id.as_deref(), Some("ds1"));
    assert_eq!(app.knowledge.retrieve_top_k_input, "50");
    assert_eq!(app.knowledge.retrieve_score_threshold_input, "0.23");
    assert!(app.knowledge.loading_documents);

    let _ = update(
        &mut app,
        KnowledgeToolMessage::DocumentsLoaded("other".to_string(), Ok(vec![document("d1", "ds1")])),
    );
    assert!(app.knowledge.documents.is_empty());

    let _ = update(
        &mut app,
        KnowledgeToolMessage::DocumentsLoaded("ds1".to_string(), Ok(vec![document("d1", "ds1")])),
    );
    assert_eq!(app.knowledge.documents.len(), 1);
    assert!(app.knowledge.selected_document_id.is_none());
    assert!(!app.knowledge.loading_chunks);

    app.knowledge.document_preview_open = true;
    app.knowledge.selected_document_id = Some("d1".to_string());
    let _ = update(
        &mut app,
        KnowledgeToolMessage::DocumentsLoaded("ds1".to_string(), Ok(vec![document("d1", "ds1")])),
    );
    assert!(app.knowledge.loading_chunks);
}

#[test]
fn update_switches_between_dataset_list_and_detail() {
    let mut app = test_app();
    app.knowledge.datasets = vec![dataset("ds1", 8, false, 0.1), dataset("ds2", 4, true, 0.5)];

    let _ = update(&mut app, KnowledgeToolMessage::SelectDataset("ds2".to_string()));
    assert_eq!(app.knowledge.selected_dataset_id.as_deref(), Some("ds2"));
    assert!(!app.knowledge.showing_dataset_list);
    assert!(app.knowledge.loading_documents);

    let _ = update(&mut app, KnowledgeToolMessage::ShowDatasetList);
    assert!(app.knowledge.showing_dataset_list);

    app.knowledge.show_dataset_detail();
    let _ =
        update(&mut app, KnowledgeToolMessage::DatasetDeleted(Ok(dataset("ds2", 4, true, 0.5))));
    assert!(app.knowledge.showing_dataset_list);
}

#[test]
fn knowledge_embedding_config_state_loads_gateway_data_and_test_results() {
    let mut app = test_app();

    let _ = update(&mut app, KnowledgeToolMessage::Refresh);
    assert!(app.knowledge.loading_embedding_status);
    assert!(app.knowledge.loading_embedding_presets);

    let status = KnowledgeEmbeddingStatusDto {
        provider_id: "alibaba-cn".to_string(),
        route_hint: Some("semantic".to_string()),
        model: "text-embedding-v4".to_string(),
        dimensions: 1024,
        api_key_configured: true,
        vector_available: true,
        hybrid_available: true,
        retrieval_modes: vec![KnowledgeRetrievalMode::FullText, KnowledgeRetrievalMode::Hybrid],
        notes: BTreeMap::new(),
    };
    let _ = update(&mut app, KnowledgeToolMessage::EmbeddingStatusLoaded(Ok(status.clone())));
    assert!(!app.knowledge.loading_embedding_status);
    assert_eq!(app.knowledge.embedding_status, Some(status));

    let presets = vec![KnowledgeEmbeddingPresetDto {
        id: "doubao-volcengine".to_string(),
        label: "Doubao Volcengine OpenAI-compatible".to_string(),
        provider_id: "custom:https://ark.cn-beijing.volces.com/api/v3".to_string(),
        model: "doubao-embedding-text-240715".to_string(),
        dimensions: 2048,
        endpoint_url: Some("https://ark.cn-beijing.volces.com/api/v3/embeddings".to_string()),
        api_key_env_vars: vec!["ARK_API_KEY".to_string()],
        description: "Volcengine Ark embeddings through the gateway route.".to_string(),
    }];
    let _ = update(&mut app, KnowledgeToolMessage::EmbeddingPresetsLoaded(Ok(presets.clone())));
    assert!(!app.knowledge.loading_embedding_presets);
    assert_eq!(app.knowledge.embedding_presets, presets);

    let _ = update(&mut app, KnowledgeToolMessage::RunEmbeddingTest);
    assert!(app.knowledge.testing_embedding);
    assert!(app.knowledge.embedding_test_result.is_none());

    let response = KnowledgeEmbeddingTestResponse {
        ok: true,
        provider_id: "alibaba-cn".to_string(),
        model: "text-embedding-v4".to_string(),
        configured_dimensions: 1024,
        observed_dimensions: Some(1024),
        latency_ms: Some(42),
        message: "embedding test succeeded".to_string(),
    };
    let _ = update(&mut app, KnowledgeToolMessage::EmbeddingTestFinished(Ok(response.clone())));
    assert!(!app.knowledge.testing_embedding);
    assert_eq!(app.knowledge.embedding_test_result, Some(response));
    assert_eq!(app.knowledge.notification.as_deref(), Some("嵌入连接测试通过：1024 维"));

    app.knowledge.testing_embedding = true;
    let failed = KnowledgeEmbeddingTestResponse {
        ok: false,
        provider_id: "alibaba-cn".to_string(),
        model: "text-embedding-v4".to_string(),
        configured_dimensions: 1024,
        observed_dimensions: None,
        latency_ms: Some(12),
        message: "upstream rejected sk-live-secret-token".to_string(),
    };
    let _ = update(&mut app, KnowledgeToolMessage::EmbeddingTestFinished(Ok(failed)));
    assert!(!app.knowledge.testing_embedding);
    let stored_message = app.knowledge.embedding_test_result.as_ref().unwrap().message.as_str();
    assert!(!stored_message.contains("sk-live-secret-token"));
    assert_eq!(
        app.knowledge.notification.as_deref(),
        Some("嵌入连接测试失败：upstream rejected [REDACTED]")
    );

    app.knowledge.testing_embedding = true;
    let _ = update(
        &mut app,
        KnowledgeToolMessage::EmbeddingTestFinished(Err(
            "gateway leaked dashscope-secret-token".to_string()
        )),
    );
    assert!(!app.knowledge.testing_embedding);
    assert_eq!(app.knowledge.notification.as_deref(), Some("gateway leaked [REDACTED]"));
}

#[test]
fn update_validation_paths_set_local_errors_without_gateway() {
    let mut app = test_app();

    let _ = update(&mut app, KnowledgeToolMessage::CreateDataset);
    assert_eq!(app.knowledge.notification.as_deref(), Some("知识库名称不能为空"));

    app.knowledge.dataset_name_input = "docs".to_string();
    app.knowledge.dataset_score_threshold_input = "bad".to_string();
    let _ = update(&mut app, KnowledgeToolMessage::CreateDataset);
    assert_eq!(app.knowledge.notification.as_deref(), Some("Score 阈值必须是 0 到 1 之间的数字"));

    let _ = update(&mut app, KnowledgeToolMessage::CreateDocument);
    assert_eq!(app.knowledge.notification.as_deref(), Some("请先选择知识库"));

    app.knowledge.selected_dataset_id = Some("ds".to_string());
    let _ = update(&mut app, KnowledgeToolMessage::CreateDocument);
    assert_eq!(app.knowledge.notification.as_deref(), Some("文档名称不能为空"));

    app.knowledge.document_name_input = "doc".to_string();
    let _ = update(&mut app, KnowledgeToolMessage::CreateDocument);
    assert_eq!(app.knowledge.notification.as_deref(), Some("文档内容不能为空"));

    let _ = update(&mut app, KnowledgeToolMessage::Retrieve);
    assert_eq!(app.knowledge.notification.as_deref(), Some("请输入召回测试文本"));
}

#[test]
fn dataset_created_closes_modal_and_selects_detail() {
    let mut app = test_app();
    app.knowledge.create_dataset_modal_open = true;
    app.knowledge.creating_dataset = true;
    app.knowledge.dataset_name_input = "draft".to_string();
    app.knowledge.dataset_description_input = "desc".to_string();

    let _ = update(
        &mut app,
        KnowledgeToolMessage::DatasetCreated(Ok(dataset("created", 10, false, 0.15))),
    );

    assert!(!app.knowledge.creating_dataset);
    assert!(!app.knowledge.create_dataset_modal_open);
    assert_eq!(app.knowledge.selected_dataset_id.as_deref(), Some("created"));
    assert!(!app.knowledge.showing_dataset_list);
    assert!(app.knowledge.dataset_name_input.is_empty());
    assert_eq!(app.knowledge.datasets.len(), 1);
}

#[test]
fn create_dataset_modal_tracks_tabs_and_supported_embedding_selection() {
    let mut app = test_app();
    app.knowledge.embedding_status = Some(KnowledgeEmbeddingStatusDto {
        provider_id: "alibaba-cn".to_string(),
        route_hint: Some("semantic".to_string()),
        model: "text-embedding-v4".to_string(),
        dimensions: 1024,
        api_key_configured: true,
        vector_available: true,
        hybrid_available: true,
        retrieval_modes: vec![KnowledgeRetrievalMode::FullText, KnowledgeRetrievalMode::Hybrid],
        notes: BTreeMap::new(),
    });

    let _ = update(&mut app, KnowledgeToolMessage::OpenCreateDatasetModal);
    assert_eq!(app.knowledge.create_dataset_tab, KnowledgeCreateDatasetTab::Basic);
    assert_eq!(selected_dataset_embedding_model(&app), None);

    let _ = update(
        &mut app,
        KnowledgeToolMessage::SelectCreateDatasetTab(KnowledgeCreateDatasetTab::Retrieval),
    );
    let _ = update(
        &mut app,
        KnowledgeToolMessage::DatasetEmbeddingSelectionChanged(
            KnowledgeDatasetEmbeddingSelection::LockCurrent,
        ),
    );

    assert_eq!(app.knowledge.create_dataset_tab, KnowledgeCreateDatasetTab::Retrieval);
    assert_eq!(selected_dataset_embedding_model(&app).as_deref(), Some("text-embedding-v4"));

    let _ = update(&mut app, KnowledgeToolMessage::CloseCreateDatasetModal);
    let _ = update(&mut app, KnowledgeToolMessage::OpenCreateDatasetModal);
    assert_eq!(app.knowledge.create_dataset_tab, KnowledgeCreateDatasetTab::Basic);
}

#[test]
fn update_completion_messages_reset_flags_and_clear_drafts() {
    let mut app = test_app();
    app.knowledge.datasets = vec![dataset("ds1", 8, false, 0.1), dataset("ds2", 4, true, 0.5)];
    app.knowledge.documents = vec![document("d1", "ds1"), document("d2", "ds1")];
    app.knowledge.retrieve_results = vec![chunk("old")];

    let _ =
        update(&mut app, KnowledgeToolMessage::DatasetDeleted(Ok(dataset("ds1", 8, false, 0.1))));
    assert!(!app.knowledge.deleting);
    assert_eq!(app.knowledge.datasets.len(), 1);
    assert_eq!(app.knowledge.selected_dataset_id.as_deref(), Some("ds2"));
    assert!(app.knowledge.documents.is_empty());
    assert!(app.knowledge.retrieve_results.is_empty());

    app.knowledge.documents = vec![document("d1", "ds2")];
    let _ = update(&mut app, KnowledgeToolMessage::DocumentDeleted(Ok(document("d1", "ds2"))));
    assert!(app.knowledge.documents.is_empty());
    assert_eq!(app.knowledge.notification.as_deref(), Some("文档已删除"));

    let _ = update(
        &mut app,
        KnowledgeToolMessage::RetrieveFinished(Ok(KnowledgeRetrieveResponse {
            chunks: vec![chunk("c1")],
        })),
    );
    assert!(!app.knowledge.retrieving);
    assert_eq!(app.knowledge.retrieve_results.len(), 1);

    app.knowledge.document_name_input = "draft".to_string();
    app.knowledge.document_content_editor = text_editor::Content::with_text("content");
    let _ = update(&mut app, KnowledgeToolMessage::ClearDocumentDraft);
    assert!(app.knowledge.document_name_input.is_empty());
    assert!(app.knowledge.document_content_editor.text().is_empty());

    let _ = update(&mut app, KnowledgeToolMessage::ClearNotification);
    assert!(app.knowledge.notification.is_none());
    assert!(!app.knowledge.notification_is_error);
}

#[test]
fn knowledge_messages_update_document_and_batch_action_results() {
    let mut app = test_app();
    app.knowledge.selected_dataset_id = Some("ds".to_string());
    app.knowledge.document_preview_open = true;
    app.knowledge.selected_document_id = Some("doc-a".to_string());
    app.knowledge.documents =
        vec![document("doc-a", "ds"), document("doc-b", "ds"), document("doc-c", "ds")];
    app.knowledge.document_status_filter = [KnowledgeDocumentStatus::Completed].into();
    app.knowledge.selected_document_ids.extend([
        "doc-a".to_string(),
        "doc-b".to_string(),
        "missing".to_string(),
    ]);
    app.knowledge.rename_document_id = Some("doc-a".to_string());
    app.knowledge.rename_document_name_input = "Renamed doc".to_string();

    let _ = update(
        &mut app,
        KnowledgeToolMessage::SubmitDocumentRename {
            document_id: "doc-a".to_string(),
            name: " Renamed doc ".to_string(),
        },
    );
    assert_eq!(
        build_document_patch_request(Some(" Renamed doc "), None, None),
        Some(KnowledgeDocumentPatchRequest {
            name: Some("Renamed doc".to_string()),
            enabled: None,
            archived: None,
        })
    );
    assert_eq!(app.knowledge.updating_document_id.as_deref(), Some("doc-a"));

    let mut renamed = document("doc-a", "ds");
    renamed.name = "Renamed doc".to_string();
    let _ = update(&mut app, KnowledgeToolMessage::DocumentUpdated(Ok(renamed)));
    assert!(app.knowledge.rename_document_id.is_none());
    assert!(app.knowledge.rename_document_name_input.is_empty());
    assert_eq!(app.knowledge.documents[0].name, "Renamed doc");
    assert!(app.knowledge.selected_document_ids.contains("doc-a"));
    assert!(app.knowledge.selected_document_ids.contains("doc-b"));
    assert!(!app.knowledge.selected_document_ids.contains("missing"));

    let _ = update(
        &mut app,
        KnowledgeToolMessage::SubmitBatchDocumentAction {
            action: KnowledgeDocumentsBatchAction::Archive,
        },
    );
    assert_eq!(
        build_batch_document_action_request(
            &app.knowledge.selected_document_ids,
            KnowledgeDocumentsBatchAction::Archive
        ),
        Some(KnowledgeDocumentsBatchActionRequest {
            document_ids: vec!["doc-a".to_string(), "doc-b".to_string()],
            action: KnowledgeDocumentsBatchAction::Archive,
        })
    );
    assert_eq!(
        app.knowledge.pending_batch_document_action,
        Some(KnowledgeDocumentsBatchAction::Archive)
    );

    let mut archived_a = document("doc-a", "ds");
    archived_a.status = KnowledgeDocumentStatus::Archived;
    archived_a.enabled = false;
    archived_a.archived_at_ms = Some(99);
    let mut archived_b = document("doc-b", "ds");
    archived_b.status = KnowledgeDocumentStatus::Archived;
    archived_b.enabled = false;
    archived_b.archived_at_ms = Some(100);
    let _ = update(
        &mut app,
        KnowledgeToolMessage::DocumentsBatchUpdated(Ok(vec![archived_a, archived_b])),
    );

    assert!(app.knowledge.pending_batch_document_action.is_none());
    assert!(!app.knowledge.batch_updating_documents);
    assert!(app.knowledge.selected_document_ids.is_empty());
    assert!(app.knowledge.documents.iter().take(2).all(|document| {
        document.status == KnowledgeDocumentStatus::Archived && !document.enabled
    }));
    assert!(app.knowledge.selected_document_id.is_none());
    assert!(!app.knowledge.document_preview_open);
    assert_eq!(app.knowledge.notification.as_deref(), Some("已批量归档 2 篇文档"));
}

#[test]
fn update_inverts_visible_document_selection() {
    let mut app = test_app();
    app.knowledge.documents =
        vec![document("doc-a", "ds"), document("doc-b", "ds"), document("doc-c", "ds")];
    app.knowledge.document_page_size = 10;
    app.knowledge.selected_document_ids.insert("doc-a".to_string());

    let _ = update(&mut app, KnowledgeToolMessage::InvertVisibleDocumentSelection);
    assert!(!app.knowledge.selected_document_ids.contains("doc-a"));
    assert!(app.knowledge.selected_document_ids.contains("doc-b"));
    assert!(app.knowledge.selected_document_ids.contains("doc-c"));

    let _ = update(&mut app, KnowledgeToolMessage::InvertVisibleDocumentSelection);
    assert!(app.knowledge.selected_document_ids.contains("doc-a"));
    assert!(!app.knowledge.selected_document_ids.contains("doc-b"));
    assert!(!app.knowledge.selected_document_ids.contains("doc-c"));
}

#[test]
fn document_status_filter_hides_archived_by_default_and_supports_multi_select() {
    let mut app = test_app();
    let mut archived = document("doc-archived", "ds");
    archived.status = KnowledgeDocumentStatus::Archived;
    archived.enabled = false;
    let mut disabled = document("doc-disabled", "ds");
    disabled.enabled = false;
    app.knowledge.documents = vec![document("doc-completed", "ds"), disabled, archived];
    app.knowledge.document_page_size = 10;

    // 默认隐藏归档文档。
    assert_eq!(
        app.knowledge.filtered_document_ids(),
        vec!["doc-completed".to_string(), "doc-disabled".to_string()]
    );

    // 显式选中归档后才显示，且仅显示该状态。
    let _ = update(
        &mut app,
        KnowledgeToolMessage::ToggleDocumentStatusFilter(KnowledgeDocumentStatus::Archived),
    );
    assert_eq!(app.knowledge.filtered_document_ids(), vec!["doc-archived".to_string()]);

    // 多选：再加上停用状态。
    let _ = update(
        &mut app,
        KnowledgeToolMessage::ToggleDocumentStatusFilter(KnowledgeDocumentStatus::Disabled),
    );
    assert_eq!(
        app.knowledge.filtered_document_ids(),
        vec!["doc-disabled".to_string(), "doc-archived".to_string()]
    );

    // 清除筛选回到默认（隐藏归档）。
    let _ = update(&mut app, KnowledgeToolMessage::ClearDocumentStatusFilter);
    assert_eq!(
        app.knowledge.filtered_document_ids(),
        vec!["doc-completed".to_string(), "doc-disabled".to_string()]
    );
}

#[test]
fn import_filename_mime_detection_accepts_supported_extensions_case_insensitively() {
    assert_eq!(mime_type_for_filename("notes.TXT").as_deref(), Some("text/plain"));
    assert_eq!(mime_type_for_filename("guide.MarkDown").as_deref(), Some("text/markdown"));
    assert_eq!(mime_type_for_filename("component.MDX").as_deref(), Some("text/markdown"));
    assert_eq!(mime_type_for_filename("manual.PDF").as_deref(), Some("application/pdf"));
    assert_eq!(mime_type_for_filename("archive.zip"), None);
    assert_eq!(mime_type_for_filename("README"), None);
}

#[test]
fn import_request_preserves_file_fields_and_adds_non_sensitive_source_metadata() {
    let request = import_request_from_file(crate::app::state::KnowledgeFileImportDraft {
        filename: "guide.md".to_string(),
        mime_type: Some("text/markdown".to_string()),
        byte_len: 27,
        content_base64: "IyBHdWlkZQ==".to_string(),
    });

    assert_eq!(request.filename, "guide.md");
    assert_eq!(request.mime_type.as_deref(), Some("text/markdown"));
    assert_eq!(request.content_base64, "IyBHdWlkZQ==");
    assert_eq!(request.metadata["source"], "desktop_file_import");
    assert_eq!(request.metadata["source_bytes"], 27);
    assert!(request.enabled);
}

#[test]
fn import_success_messages_report_single_and_batch_warning_counts() {
    let plain = queued_response("plain.md", "doc-a", "job-a");
    let mut warned = queued_response("warned.pdf", "doc-b", "job-b");
    warned.warnings = vec!["字体缺失".to_string(), "页面为空".to_string()];

    assert_eq!(import_success_message(&plain), "文件已入队：plain.md，后台入库中");
    assert_eq!(import_success_message(&warned), "文件已入队：warned.pdf（2 条警告），后台入库中");
    assert_eq!(
        import_batch_success_message(&[plain, warned]),
        "已入队 2 个文件（2 条警告），后台入库中"
    );
    assert_eq!(import_batch_success_message(&[]), "已入队 0 个文件，后台入库中");
}

#[test]
fn index_job_status_maps_to_document_processing_status() {
    assert_eq!(
        document_status_from_index_job(&KnowledgeIndexJobStatus::Queued),
        KnowledgeDocumentStatus::Pending
    );
    assert_eq!(
        document_status_from_index_job(&KnowledgeIndexJobStatus::Processing),
        KnowledgeDocumentStatus::Processing
    );
    assert_eq!(
        document_status_from_index_job(&KnowledgeIndexJobStatus::Completed),
        KnowledgeDocumentStatus::Completed
    );
    assert_eq!(
        document_status_from_index_job(&KnowledgeIndexJobStatus::Failed),
        KnowledgeDocumentStatus::Failed
    );
}

#[test]
fn sensitive_display_text_redacts_supported_token_shapes_and_preserves_punctuation() {
    let sanitized = sanitize_sensitive_display_text(
        "keys: (sk-live-123), bearer.session-token API_KEY=abc authorization=xyz harmless",
    );

    assert_eq!(sanitized, "keys: ([REDACTED]), [REDACTED] [REDACTED] [REDACTED] harmless");
    assert!(looks_sensitive_token("service-secret-value"));
    assert!(!looks_sensitive_token("secret"));
    assert!(!sanitized.contains("sk-live-123"));
    assert!(!sanitized.contains("session-token"));
}

#[test]
fn ensure_loaded_only_schedules_missing_knowledge_resources() {
    let mut app = test_app();

    let task = super::ensure_loaded(&mut app);

    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
    assert!(app.knowledge.loading_status);
    assert!(app.knowledge.loading_embedding_status);
    assert!(app.knowledge.loading_embedding_presets);
    assert!(app.knowledge.loading_datasets);

    let second = super::ensure_loaded(&mut app);
    assert_eq!(second.units(), 0);

    app.knowledge.loading_status = false;
    app.knowledge.runtime_status = Some(KnowledgeRuntimeStatus {
        full_text: true,
        vector: false,
        hybrid: false,
        rerank: false,
        notes: BTreeMap::new(),
    });
    app.knowledge.loading_embedding_status = false;
    app.knowledge.embedding_status = Some(KnowledgeEmbeddingStatusDto {
        provider_id: "provider".to_string(),
        route_hint: None,
        model: "model".to_string(),
        dimensions: 128,
        api_key_configured: false,
        vector_available: false,
        hybrid_available: false,
        retrieval_modes: vec![KnowledgeRetrievalMode::FullText],
        notes: BTreeMap::new(),
    });
    app.knowledge.loading_embedding_presets = false;
    app.knowledge.embedding_presets = vec![KnowledgeEmbeddingPresetDto {
        id: "preset".to_string(),
        label: "Preset".to_string(),
        provider_id: "provider".to_string(),
        model: "model".to_string(),
        dimensions: 128,
        endpoint_url: None,
        api_key_env_vars: vec![],
        description: String::new(),
    }];
    app.knowledge.loading_datasets = false;

    let datasets_task = super::ensure_loaded(&mut app);
    assert!(iced_runtime::task::into_stream(datasets_task.collect()).is_some());
    assert!(app.knowledge.loading_datasets);
    assert!(!app.knowledge.loading_status);
    assert!(!app.knowledge.loading_embedding_status);
    assert!(!app.knowledge.loading_embedding_presets);
}

#[test]
fn menu_copy_and_delete_target_messages_update_local_state() {
    let mut app = test_app();

    let _ = update(&mut app, KnowledgeToolMessage::ToggleDocumentStatusMenu);
    assert!(app.knowledge.document_status_menu_open);
    let _ = update(&mut app, KnowledgeToolMessage::CloseDocumentStatusMenu);
    assert!(!app.knowledge.document_status_menu_open);

    let _ = update(&mut app, KnowledgeToolMessage::ToggleDatasetMenu("ds-a".to_string()));
    assert_eq!(app.knowledge.dataset_menu_open_id.as_deref(), Some("ds-a"));
    let _ = update(&mut app, KnowledgeToolMessage::OpenDeleteDatasetConfirmFor("ds-b".to_string()));
    assert!(app.knowledge.dataset_menu_open_id.is_none());
    assert_eq!(app.knowledge.pending_delete_dataset_id.as_deref(), Some("ds-b"));
    assert!(app.knowledge.pending_delete_document_id.is_none());

    let _ = update(&mut app, KnowledgeToolMessage::ToggleDatasetMenu("ds-a".to_string()));
    let task = update(&mut app, KnowledgeToolMessage::CopyDatasetId("ds-a".to_string()));
    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
    assert!(app.knowledge.dataset_menu_open_id.is_none());
    assert_eq!(app.knowledge.notification.as_deref(), Some("知识库 ID 已复制"));
    assert!(!app.knowledge.notification_is_error);

    let _ = update(&mut app, KnowledgeToolMessage::CloseDatasetMenu);
    assert!(app.knowledge.dataset_menu_open_id.is_none());
}

#[test]
fn import_empty_error_clear_busy_and_polling_edges_are_local_only() {
    let mut app = test_app();

    let _ = update(&mut app, KnowledgeToolMessage::ImportFileSelected(Ok(Vec::new())));
    assert!(app.knowledge.file_imports.is_empty());
    assert!(!app.knowledge.selecting_import_file);

    let _ = update(
        &mut app,
        KnowledgeToolMessage::ImportFileSelected(Err("取消或读取失败".to_string())),
    );
    assert_eq!(app.knowledge.notification.as_deref(), Some("取消或读取失败"));
    assert!(app.knowledge.notification_is_error);

    app.knowledge.file_imports = vec![crate::app::state::KnowledgeFileImportDraft {
        filename: "guide.md".to_string(),
        mime_type: Some("text/markdown".to_string()),
        byte_len: 12,
        content_base64: "IyBHdWlkZQ==".to_string(),
    }];
    app.knowledge.file_drop_hovered = true;
    let _ = update(&mut app, KnowledgeToolMessage::ClearImportFile);
    assert!(app.knowledge.file_imports.is_empty());
    assert!(!app.knowledge.file_drop_hovered);

    app.knowledge.importing_file = true;
    let _ = update(&mut app, KnowledgeToolMessage::ImportSelectedFile);
    assert!(app.knowledge.importing_file);

    app.knowledge.importing_file = false;
    let _ = update(&mut app, KnowledgeToolMessage::ImportSelectedFile);
    assert_eq!(app.knowledge.notification.as_deref(), Some("请先选择知识库"));

    let empty_poll = update(&mut app, KnowledgeToolMessage::PollImportJobs);
    assert_eq!(empty_poll.units(), 0);

    let _ =
        update(&mut app, KnowledgeToolMessage::ImportJobPolled(Err("gateway down".to_string())));
    assert_eq!(app.knowledge.notification.as_deref(), Some("入库轮询失败：gateway down"));

    app.knowledge.import_polling_jobs.clear();
    let _ = update(
        &mut app,
        KnowledgeToolMessage::ImportJobPolled(Ok(index_job(
            "unknown",
            "doc",
            KnowledgeIndexJobStatus::Completed,
        ))),
    );
    assert!(app.knowledge.import_polling_jobs.is_empty());
}

#[test]
fn document_update_batch_and_delete_validation_cover_error_and_busy_paths() {
    let mut app = test_app();

    let _ = update(&mut app, KnowledgeToolMessage::DeleteSelectedDataset);
    assert_eq!(app.knowledge.notification.as_deref(), Some("请先选择知识库"));

    app.knowledge.deleting = true;
    app.knowledge.selected_dataset_id = Some("ds".to_string());
    let no_delete = update(&mut app, KnowledgeToolMessage::DeleteSelectedDataset);
    assert_eq!(no_delete.units(), 0);
    assert!(app.knowledge.deleting);

    app.knowledge.deleting = false;
    let _ = update(
        &mut app,
        KnowledgeToolMessage::SubmitDocumentRename {
            document_id: "doc-a".to_string(),
            name: "   ".to_string(),
        },
    );
    assert_eq!(app.knowledge.notification.as_deref(), Some("请提供有效的文档更新内容"));

    app.knowledge.updating_document_id = Some("doc-busy".to_string());
    let busy_update = update(
        &mut app,
        KnowledgeToolMessage::SetDocumentEnabled {
            document_id: "doc-a".to_string(),
            enabled: false,
        },
    );
    assert_eq!(busy_update.units(), 0);
    assert_eq!(app.knowledge.updating_document_id.as_deref(), Some("doc-busy"));

    app.knowledge.updating_document_id = None;
    let _ = update(
        &mut app,
        KnowledgeToolMessage::SubmitBatchDocumentAction {
            action: KnowledgeDocumentsBatchAction::Disable,
        },
    );
    assert_eq!(app.knowledge.notification.as_deref(), Some("请先选择文档"));

    app.knowledge.batch_updating_documents = true;
    app.knowledge.selected_document_ids.insert("doc-a".to_string());
    let busy_batch = update(
        &mut app,
        KnowledgeToolMessage::SubmitBatchDocumentAction {
            action: KnowledgeDocumentsBatchAction::Disable,
        },
    );
    assert_eq!(busy_batch.units(), 0);
    assert!(app.knowledge.batch_updating_documents);
}

#[test]
fn document_updated_and_batch_errors_reset_flags_and_archived_document_closes_preview() {
    let mut app = test_app();
    app.knowledge.documents = vec![document("doc-a", "ds")];
    app.knowledge.selected_document_id = Some("doc-a".to_string());
    app.knowledge.selected_chunk_id = Some("chunk-a".to_string());
    app.knowledge.document_preview_open = true;
    app.knowledge.loaded_chunks = vec![chunk("chunk-a")];
    app.knowledge.loading_chunks = true;
    app.knowledge.updating_document_id = Some("doc-a".to_string());

    let mut archived = document("doc-a", "ds");
    archived.status = KnowledgeDocumentStatus::Archived;
    archived.enabled = false;
    archived.archived_at_ms = Some(10);
    let _ = update(&mut app, KnowledgeToolMessage::DocumentUpdated(Ok(archived)));

    assert!(app.knowledge.updating_document_id.is_none());
    assert!(app.knowledge.selected_document_id.is_none());
    assert!(app.knowledge.selected_chunk_id.is_none());
    assert!(!app.knowledge.document_preview_open);
    assert!(app.knowledge.loaded_chunks.is_empty());
    assert!(!app.knowledge.loading_chunks);
    assert_eq!(app.knowledge.notification.as_deref(), Some("文档已更新"));

    app.knowledge.updating_document_id = Some("doc-a".to_string());
    let _ = update(&mut app, KnowledgeToolMessage::DocumentUpdated(Err("更新失败".to_string())));
    assert!(app.knowledge.updating_document_id.is_none());
    assert_eq!(app.knowledge.notification.as_deref(), Some("更新失败"));
    assert!(app.knowledge.notification_is_error);

    app.knowledge.batch_updating_documents = true;
    app.knowledge.pending_batch_document_action = Some(KnowledgeDocumentsBatchAction::Enable);
    let _ =
        update(&mut app, KnowledgeToolMessage::DocumentsBatchUpdated(Err("批量失败".to_string())));
    assert!(!app.knowledge.batch_updating_documents);
    assert!(app.knowledge.pending_batch_document_action.is_none());
    assert_eq!(app.knowledge.notification.as_deref(), Some("批量失败"));
    assert!(app.knowledge.notification_is_error);
}

#[test]
fn retrieve_reindex_and_embedding_busy_guards_do_not_schedule_duplicate_work() {
    let mut app = test_app();

    app.knowledge.retrieving = true;
    let retrieve_task = update(&mut app, KnowledgeToolMessage::Retrieve);
    assert_eq!(retrieve_task.units(), 0);

    app.knowledge.retrieving = false;
    app.knowledge.selected_dataset_id = Some("ds".to_string());
    app.knowledge.retrieve_query_input = "  query  ".to_string();
    app.knowledge.retrieve_top_k_input = "999".to_string();
    app.knowledge.retrieve_score_threshold_enabled = true;
    app.knowledge.retrieve_score_threshold_input = "bad".to_string();
    let retrieve_task = update(&mut app, KnowledgeToolMessage::Retrieve);
    assert!(iced_runtime::task::into_stream(retrieve_task.collect()).is_some());
    assert!(app.knowledge.retrieving);

    app.knowledge.reindexing_document_id = Some("doc-busy".to_string());
    let reindex_task = update(&mut app, KnowledgeToolMessage::ReindexDocument("doc-a".to_string()));
    assert_eq!(reindex_task.units(), 0);
    assert_eq!(app.knowledge.reindexing_document_id.as_deref(), Some("doc-busy"));

    app.knowledge.testing_embedding = true;
    let embedding_task = update(&mut app, KnowledgeToolMessage::RunEmbeddingTest);
    assert_eq!(embedding_task.units(), 0);
    assert!(app.knowledge.testing_embedding);

    let request = super::embedding_test_request();
    assert!(request.provider_id.is_none());
    assert!(request.model.is_none());
    assert!(request.dimensions.is_none());
    assert!(request.api_key_override.is_none());
    assert!(request.sample_text.is_none());
}
