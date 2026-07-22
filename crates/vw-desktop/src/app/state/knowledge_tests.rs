use super::*;
use serde_json::Value;
use vw_gateway_client::{
    KnowledgeCapabilitiesDto, KnowledgeChunkDto, KnowledgeChunkType, KnowledgeDocumentDto,
    KnowledgeDocumentStatus, KnowledgeMatchType,
};

fn dataset(id: &str, top_k: usize, threshold_enabled: bool, threshold: f64) -> KnowledgeDatasetDto {
    KnowledgeDatasetDto {
        id: id.to_string(),
        name: format!("Dataset {id}"),
        description: String::new(),
        capabilities: KnowledgeCapabilitiesDto::default(),
        chunking_mode: KnowledgeChunkingMode::General,
        indexing_mode: KnowledgeIndexingMode::Economy,
        retrieval_mode: KnowledgeRetrievalMode::FullText,
        keyword_count: 10,
        top_k,
        score_threshold_enabled: threshold_enabled,
        score_threshold: threshold,
        rerank_enabled: false,
        embedding_model: None,
        rerank_model: None,
        document_count: 0,
        chunk_count: 0,
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}

fn document(id: &str) -> KnowledgeDocumentDto {
    KnowledgeDocumentDto {
        id: id.to_string(),
        dataset_id: "first".to_string(),
        name: format!("Document {id}"),
        metadata: Value::Null,
        enabled: true,
        status: KnowledgeDocumentStatus::Completed,
        error_message: String::new(),
        content_hash: String::new(),
        chunk_count: 1,
        char_count: 0,
        recall_count: 0,
        archived_at_ms: None,
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}

fn chunk(id: &str, document_id: &str) -> KnowledgeChunkDto {
    KnowledgeChunkDto {
        id: id.to_string(),
        dataset_id: "first".to_string(),
        document_id: document_id.to_string(),
        title: format!("Chunk {id}"),
        content: "content".to_string(),
        metadata: Value::Null,
        score: None,
        ordinal: 0,
        chunk_type: KnowledgeChunkType::Text,
        start_at: 0,
        end_at: 7,
        parent_chunk_id: None,
        prev_chunk_id: None,
        next_chunk_id: None,
        matched_content: None,
        match_type: KnowledgeMatchType::FullText,
        context: None,
    }
}

#[test]
fn detail_tabs_expose_expected_titles_and_default() {
    assert_eq!(KnowledgeDetailTab::default(), KnowledgeDetailTab::Documents);
    assert_eq!(
        KnowledgeDetailTab::ALL,
        [
            KnowledgeDetailTab::Documents,
            KnowledgeDetailTab::AddDocument,
            KnowledgeDetailTab::Retrieval,
            KnowledgeDetailTab::Settings
        ]
    );
    assert_eq!(KnowledgeDetailTab::Documents.title(), "文档");
    assert_eq!(KnowledgeDetailTab::AddDocument.title(), "添加文档");
    assert_eq!(KnowledgeDetailTab::Retrieval.title(), "召回测试");
    assert_eq!(KnowledgeDetailTab::Settings.title(), "设置");
    assert_eq!(KnowledgeDocumentInputTab::default(), KnowledgeDocumentInputTab::Files);
    assert_eq!(
        KnowledgeDocumentInputTab::ALL,
        [KnowledgeDocumentInputTab::Files, KnowledgeDocumentInputTab::Text]
    );
    assert_eq!(KnowledgeDocumentInputTab::Files.title(), "文件");
    assert_eq!(KnowledgeDocumentInputTab::Text.title(), "文本");
}

#[test]
fn default_state_matches_empty_idle_ui_defaults() {
    let state = KnowledgeUiState::default();

    assert!(state.datasets.is_empty());
    assert!(state.documents.is_empty());
    assert!(state.loaded_chunks.is_empty());
    assert!(state.retrieve_results.is_empty());
    assert!(state.runtime_status.is_none());
    assert!(state.selected_dataset_id.is_none());
    assert!(state.selected_document_id.is_none());
    assert!(state.selected_chunk_id.is_none());
    assert_eq!(state.active_tab, KnowledgeDetailTab::Documents);
    assert_eq!(state.document_input_tab, KnowledgeDocumentInputTab::Files);
    assert!(state.showing_dataset_list);
    assert!(!state.create_dataset_modal_open);
    assert!(!state.document_preview_open);
    assert_eq!(state.document_page, 1);
    assert_eq!(state.document_page_size, 10);
    assert_eq!(state.dataset_chunking_mode, KnowledgeChunkingMode::General);
    assert_eq!(state.dataset_indexing_mode, KnowledgeIndexingMode::Economy);
    assert_eq!(state.dataset_retrieval_mode, KnowledgeRetrievalMode::FullText);
    assert_eq!(state.dataset_keyword_count_input, "10");
    assert_eq!(state.dataset_top_k_input, "10");
    assert_eq!(state.dataset_score_threshold_input, "0.15");
    assert_eq!(state.retrieve_top_k_input, "10");
    assert_eq!(state.retrieve_score_threshold_input, "0.15");
    assert!(state.file_imports.is_empty());
    assert!(!state.file_drop_hovered);
    assert!(state.pending_delete_dataset_id.is_none());
    assert!(state.pending_delete_document_id.is_none());
    assert!(state.dataset_menu_open_id.is_none());
    assert!(!state.retrieve_include_context);
    assert!(!state.dataset_score_threshold_enabled);
    assert!(!state.dataset_rerank_enabled);
    assert!(!state.loading_chunks);
    assert!(state.reindexing_document_id.is_none());
    assert!(!state.is_busy());
    assert!(state.notification.is_none());
    assert!(!state.notification_is_error);
}

#[test]
fn dataset_list_visibility_switches_between_list_and_detail() {
    let mut state = KnowledgeUiState::default();

    state.show_dataset_detail();
    assert!(!state.showing_dataset_list);

    state.show_dataset_list();
    assert!(state.showing_dataset_list);
}

#[test]
fn selected_dataset_returns_matching_dataset_only() {
    let mut state = KnowledgeUiState {
        datasets: vec![dataset("first", 10, false, 0.15), dataset("second", 12, true, 0.8)],
        selected_dataset_id: Some("second".to_string()),
        ..Default::default()
    };

    assert_eq!(state.selected_dataset().map(|dataset| dataset.id.as_str()), Some("second"));

    state.selected_dataset_id = Some("missing".to_string());
    assert!(state.selected_dataset().is_none());

    state.selected_dataset_id = None;
    assert!(state.selected_dataset().is_none());
}

#[test]
fn busy_state_tracks_all_loading_and_mutating_flags() {
    let idle = KnowledgeUiState::default();
    assert!(!idle.is_busy());

    let mut checks: Vec<Box<dyn Fn(&mut KnowledgeUiState)>> = vec![
        Box::new(|state| state.loading_status = true),
        Box::new(|state| state.loading_embedding_status = true),
        Box::new(|state| state.loading_embedding_presets = true),
        Box::new(|state| state.loading_datasets = true),
        Box::new(|state| state.loading_documents = true),
        Box::new(|state| state.creating_dataset = true),
        Box::new(|state| state.creating_document = true),
        Box::new(|state| state.selecting_import_file = true),
        Box::new(|state| state.importing_file = true),
        Box::new(|state| state.deleting = true),
        Box::new(|state| state.retrieving = true),
        Box::new(|state| state.testing_embedding = true),
        Box::new(|state| state.loading_chunks = true),
        Box::new(|state| state.reindexing_document_id = Some("doc".to_string())),
        Box::new(|state| state.updating_document_id = Some("doc".to_string())),
        Box::new(|state| state.batch_updating_documents = true),
    ];

    for set_busy in checks.drain(..) {
        let mut state = KnowledgeUiState::default();
        set_busy(&mut state);
        assert!(state.is_busy());
    }
}

#[test]
fn document_and_chunk_selection_helpers_keep_valid_selection_or_fallback() {
    let mut state = KnowledgeUiState {
        documents: vec![document("doc-a"), document("doc-b")],
        loaded_chunks: vec![chunk("chunk-a", "doc-b"), chunk("chunk-b", "doc-b")],
        selected_document_id: Some("doc-b".to_string()),
        selected_chunk_id: Some("chunk-b".to_string()),
        ..Default::default()
    };

    assert_eq!(state.selected_document().map(|document| document.id.as_str()), Some("doc-b"));
    assert_eq!(state.selected_chunk().map(|chunk| chunk.id.as_str()), Some("chunk-b"));

    state.selected_document_id = Some("missing".to_string());
    assert!(state.selected_document().is_none());

    state.selected_chunk_id = Some("missing".to_string());
    state.select_first_chunk_if_needed();
    assert_eq!(state.selected_chunk_id.as_deref(), Some("chunk-a"));

    state.documents.clear();
    state.loaded_chunks.clear();
    state.select_first_chunk_if_needed();
    assert!(state.selected_chunk_id.is_none());
}

#[test]
fn select_first_dataset_keeps_valid_selection_or_falls_back() {
    let mut state = KnowledgeUiState {
        datasets: vec![dataset("first", 10, false, 0.15), dataset("second", 12, true, 0.8)],
        selected_dataset_id: Some("second".to_string()),
        ..Default::default()
    };

    state.select_first_dataset_if_needed();
    assert_eq!(state.selected_dataset_id.as_deref(), Some("second"));

    state.selected_dataset_id = Some("missing".to_string());
    state.select_first_dataset_if_needed();
    assert_eq!(state.selected_dataset_id.as_deref(), Some("first"));

    state.datasets.clear();
    state.select_first_dataset_if_needed();
    assert!(state.selected_dataset_id.is_none());
}

#[test]
fn sync_retrieve_defaults_clamps_selected_dataset_values() {
    let mut state = KnowledgeUiState {
        datasets: vec![dataset("low", 0, true, -0.5), dataset("high", 500, false, 2.0)],
        selected_dataset_id: Some("low".to_string()),
        retrieve_top_k_input: "unchanged".to_string(),
        retrieve_score_threshold_input: "unchanged".to_string(),
        ..Default::default()
    };

    state.sync_retrieve_defaults_from_selected_dataset();
    assert_eq!(state.retrieve_top_k_input, "1");
    assert!(state.retrieve_score_threshold_enabled);
    assert_eq!(state.retrieve_score_threshold_input, "0.0");

    state.selected_dataset_id = Some("high".to_string());
    state.sync_retrieve_defaults_from_selected_dataset();
    assert_eq!(state.retrieve_top_k_input, "50");
    assert!(!state.retrieve_score_threshold_enabled);
    assert_eq!(state.retrieve_score_threshold_input, "1.0");
}

#[test]
fn sync_retrieve_defaults_leaves_inputs_when_no_dataset_is_selected() {
    let mut state = KnowledgeUiState {
        datasets: vec![dataset("first", 10, false, 0.15)],
        selected_dataset_id: Some("missing".to_string()),
        retrieve_top_k_input: "7".to_string(),
        retrieve_score_threshold_enabled: true,
        retrieve_score_threshold_input: "0.7".to_string(),
        ..Default::default()
    };

    state.sync_retrieve_defaults_from_selected_dataset();

    assert_eq!(state.retrieve_top_k_input, "7");
    assert!(state.retrieve_score_threshold_enabled);
    assert_eq!(state.retrieve_score_threshold_input, "0.7");
}

#[test]
fn score_threshold_formatting_removes_redundant_zeroes_but_keeps_decimal() {
    assert_eq!(format_score_threshold(0.0), "0.0");
    assert_eq!(format_score_threshold(1.0), "1.0");
    assert_eq!(format_score_threshold(0.5), "0.5");
    assert_eq!(format_score_threshold(0.15), "0.15");
    assert_eq!(format_score_threshold(0.156), "0.16");
}

#[test]
fn effective_document_status_maps_disabled_documents_but_preserves_archived_status() {
    let mut disabled = document("disabled");
    disabled.enabled = false;
    assert_eq!(effective_document_status(&disabled), KnowledgeDocumentStatus::Disabled);

    disabled.status = KnowledgeDocumentStatus::Archived;
    assert_eq!(effective_document_status(&disabled), KnowledgeDocumentStatus::Archived);
}

#[test]
fn knowledge_state_tracks_document_filters_selection_and_rename_draft() {
    let mut archived = document("doc-archived");
    archived.status = KnowledgeDocumentStatus::Archived;
    archived.enabled = false;
    archived.archived_at_ms = Some(42);

    let mut state = KnowledgeUiState {
        documents: vec![document("doc-a"), document("doc-b"), archived],
        document_search_query: "Document".to_string(),
        document_page: 3,
        document_page_size: 2,
        ..Default::default()
    };

    state.toggle_document_status_filter(KnowledgeDocumentStatus::Completed);
    assert!(state.document_status_filter.contains(&KnowledgeDocumentStatus::Completed));
    assert_eq!(state.document_page, 1);
    assert_eq!(state.filtered_document_ids(), vec!["doc-a".to_string(), "doc-b".to_string()]);

    state.toggle_document_selection("doc-a");
    assert!(state.selected_document_ids.contains("doc-a"));

    state.toggle_select_all_visible_documents();
    assert!(state.selected_document_ids.contains("doc-a"));
    assert!(state.selected_document_ids.contains("doc-b"));
    assert_eq!(state.selected_document_ids.len(), 2);

    state.toggle_select_all_visible_documents();
    assert!(state.selected_document_ids.is_empty());

    state.toggle_document_selection("doc-a");
    state.toggle_document_selection("doc-b");
    state.open_document_rename_modal("doc-b");
    assert_eq!(state.rename_document_id.as_deref(), Some("doc-b"));
    assert_eq!(state.rename_document_name_input, "Document doc-b");

    state.close_document_rename_modal();
    assert!(state.rename_document_id.is_none());
    assert!(state.rename_document_name_input.is_empty());

    state.documents.retain(|document| document.id != "doc-a");
    state.prune_document_selection();
    assert!(state.selected_document_ids.contains("doc-b"));
    assert!(!state.selected_document_ids.contains("doc-a"));
}

#[test]
fn knowledge_state_handles_status_menu_pagination_and_selection_edges() {
    let mut state = KnowledgeUiState {
        documents: vec![document("doc-a"), document("doc-b"), document("doc-c")],
        document_page: 99,
        document_page_size: 0,
        ..Default::default()
    };

    assert_eq!(state.visible_document_ids(), vec!["doc-c".to_string()]);

    state.toggle_document_status_menu();
    assert!(state.document_status_menu_open);
    state.close_document_status_menu();
    assert!(!state.document_status_menu_open);

    state.document_page = 3;
    state.toggle_document_status_filter(KnowledgeDocumentStatus::Completed);
    assert_eq!(state.document_page, 1);
    state.clear_document_status_filter();
    assert!(state.document_status_filter.is_empty());
    assert_eq!(state.document_page, 1);

    state.toggle_document_selection("missing");
    assert!(state.selected_document_ids.is_empty());

    state.document_page_size = 2;
    state.toggle_document_selection("doc-a");
    state.toggle_document_selection("doc-a");
    assert!(state.selected_document_ids.is_empty());

    state.toggle_select_all_visible_documents();
    assert_eq!(state.selected_document_ids.len(), 2);
    state.invert_visible_document_selection();
    assert!(state.selected_document_ids.is_empty());

    state.open_document_rename_modal("doc-b");
    assert_eq!(state.rename_document_id.as_deref(), Some("doc-b"));
    state.documents.retain(|document| document.id != "doc-b");
    state.prune_document_selection();
    assert!(state.rename_document_id.is_none());
    assert!(state.rename_document_name_input.is_empty());

    state.clear_document_selection();
    assert!(state.selected_document_ids.is_empty());
}

#[test]
fn dataset_menu_toggles_per_card_and_resolves_dataset_by_id() {
    let mut state = KnowledgeUiState {
        datasets: vec![dataset("first", 10, false, 0.15), dataset("second", 12, true, 0.8)],
        ..Default::default()
    };

    assert_eq!(state.dataset_by_id("second").map(|dataset| dataset.id.as_str()), Some("second"));
    assert!(state.dataset_by_id("missing").is_none());

    state.toggle_dataset_menu("first");
    assert_eq!(state.dataset_menu_open_id.as_deref(), Some("first"));

    // 切到另一张卡片应改为打开后者。
    state.toggle_dataset_menu("second");
    assert_eq!(state.dataset_menu_open_id.as_deref(), Some("second"));

    // 再次点击同一张卡片应关闭。
    state.toggle_dataset_menu("second");
    assert!(state.dataset_menu_open_id.is_none());

    state.toggle_dataset_menu("first");
    state.close_dataset_menu();
    assert!(state.dataset_menu_open_id.is_none());
}
