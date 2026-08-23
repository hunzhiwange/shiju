//! 知识库视图测试，覆盖数据集、文档、分块、检索配置和各类弹窗组件。
//! 标签格式、分页边界与明暗主题状态需同时反映网关能力和当前编辑草稿。

use crate::app::state::KnowledgeFileImportDraft;
use crate::app::{App, Message};
use iced::widget::button;
use iced::{Element, Size, Theme};
use std::collections::BTreeMap;

use super::{
    DocumentPageItem, KNOWLEDGE_SCROLLBAR_GUTTER, KNOWLEDGE_SCROLLBAR_WIDTH,
    build_add_document_tab, build_create_dataset_card, build_create_dataset_modal,
    build_create_dataset_panel, build_dataset_catalog_header, build_dataset_catalog_view,
    build_dataset_list_panel, build_dataset_param_section, build_dataset_search_input,
    build_delete_confirm_modal, build_detail, build_document_bulk_toolbar,
    build_document_input_tabs, build_document_pagination, build_document_preview_drawer,
    build_document_rename_modal, build_document_status_menu, build_documents_tab,
    build_documents_table, build_documents_toolbar, build_embedding_model_picker,
    build_embedding_model_section, build_file_import_panel, build_import_polling_panel,
    build_indexing_mode_section, build_no_dataset_result_card, build_rerank_toggle,
    build_retrieval_mode_section, build_retrieval_tab, build_settings_tab,
    build_text_document_panel, build_upload_panel, build_workspace, chrome_style, chunk_card,
    chunk_preview_card, chunk_preview_title, chunk_type_label, chunking_label,
    compact_document_count_label, compact_input_row, dataset_card_button_style,
    dataset_card_container_style, dataset_description, dataset_embedding_model_label,
    dataset_embedding_options, dataset_item, dataset_tag, dataset_title, dataset_title_text_style,
    document_page_button, document_page_nav_button, document_page_size,
    document_page_size_selector, document_pagination_items, document_row, document_row_style,
    document_selection_button_style, document_status_badge, document_status_badge_style,
    document_status_filter_options, document_status_label, document_title_button_style,
    document_total_pages, effective_document_status, embedding_hybrid_available,
    embedding_preset_label, embedding_status_dimensions_label, embedding_status_model_label,
    embedding_status_provider_label, embedding_status_route_label, embedding_test_result_label,
    embedding_vector_available, empty_hint, enabled_label, file_drop_zone_style,
    file_import_summary, file_imports_summary, filtered_documents_page, format_dataset_time,
    format_file_size, format_time, icon_badge, import_job_status_label, indexing_label,
    is_dark_palette, item_style, knowledge_header_button, knowledge_status_menu_entry_style,
    knowledge_status_menu_style, knowledge_toolbar_button_style, match_type_label, option_card,
    option_card_style, retrieval_label, score_threshold_label, section_header, segment_button,
    segment_style, support_label,
};
use vw_gateway_client::{
    KnowledgeCapabilitiesDto, KnowledgeChunkDto, KnowledgeChunkType, KnowledgeChunkingMode,
    KnowledgeDatasetDto, KnowledgeDocumentDto, KnowledgeDocumentStatus,
    KnowledgeEmbeddingPresetDto, KnowledgeEmbeddingStatusDto, KnowledgeEmbeddingTestResponse,
    KnowledgeIndexJobStatus, KnowledgeIndexingMode, KnowledgeMatchType, KnowledgeRetrievalMode,
    KnowledgeRuntimeStatus,
};

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn dataset(id: &str) -> KnowledgeDatasetDto {
    KnowledgeDatasetDto {
        id: id.to_string(),
        name: format!("dataset {id}"),
        description: String::new(),
        capabilities: KnowledgeCapabilitiesDto::default(),
        chunking_mode: KnowledgeChunkingMode::General,
        indexing_mode: KnowledgeIndexingMode::Economy,
        retrieval_mode: KnowledgeRetrievalMode::FullText,
        keyword_count: 10,
        top_k: 10,
        score_threshold_enabled: false,
        score_threshold: 0.15,
        rerank_enabled: false,
        embedding_model: None,
        rerank_model: None,
        document_count: 2,
        chunk_count: 8,
        created_at_ms: 1,
        updated_at_ms: 2,
    }
}

fn document(id: &str) -> KnowledgeDocumentDto {
    KnowledgeDocumentDto {
        id: id.to_string(),
        dataset_id: "dataset-a".to_string(),
        name: format!("document {id}"),
        metadata: serde_json::json!({}),
        enabled: true,
        status: KnowledgeDocumentStatus::Completed,
        error_message: String::new(),
        content_hash: String::new(),
        chunk_count: 3,
        char_count: 0,
        recall_count: 0,
        archived_at_ms: None,
        created_at_ms: 1,
        updated_at_ms: 2,
    }
}

fn chunk(id: &str, score: Option<f64>) -> KnowledgeChunkDto {
    KnowledgeChunkDto {
        id: id.to_string(),
        dataset_id: "dataset-a".to_string(),
        document_id: "document-a".to_string(),
        title: format!("chunk {id}"),
        content: "content".to_string(),
        metadata: serde_json::json!({}),
        score,
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

fn runtime_status(vector: bool, rerank: bool) -> KnowledgeRuntimeStatus {
    KnowledgeRuntimeStatus {
        full_text: true,
        vector,
        hybrid: vector,
        rerank,
        notes: BTreeMap::from([("engine".to_string(), "sqlite".to_string())]),
    }
}

#[test]
fn knowledge_labels_match_ui_copy() {
    assert_eq!(indexing_label(&KnowledgeIndexingMode::Economy), "经济");
    assert_eq!(indexing_label(&KnowledgeIndexingMode::HighQuality), "高质量");
    assert_eq!(chunking_label(&KnowledgeChunkingMode::General), "General");
    assert_eq!(chunk_type_label(&KnowledgeChunkType::ParentText), "父块");
    assert_eq!(retrieval_label(&KnowledgeRetrievalMode::Hybrid), "混合检索");
    assert_eq!(document_status_label(&KnowledgeDocumentStatus::Failed), "失败");
    assert_eq!(match_type_label(&KnowledgeMatchType::Hybrid), "混合");
    assert_eq!(support_label(true), "可用");
    assert_eq!(support_label(false), "不可用");
    assert_eq!(enabled_label(true), "启用");
    assert_eq!(enabled_label(false), "关闭");
}

#[test]
fn detail_tab_titles_match_ui_copy() {
    assert_eq!(crate::app::state::KnowledgeDetailTab::Documents.title(), "文档");
    assert_eq!(crate::app::state::KnowledgeDetailTab::AddDocument.title(), "添加文档");
    assert_eq!(crate::app::state::KnowledgeDetailTab::Retrieval.title(), "召回测试");
    assert_eq!(crate::app::state::KnowledgeDetailTab::Settings.title(), "设置");
}

#[test]
fn dataset_embedding_label_uses_resolved_global_config() {
    let mut app = test_app();
    let existing = dataset("existing");
    assert_eq!(dataset_embedding_model_label(&app, &existing), "跟随记忆配置/嵌入路由");

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
    assert_eq!(dataset_embedding_model_label(&app, &existing), "text-embedding-v4 / 1024维");
}

#[test]
fn knowledge_embedding_config_state_labels_report_gateway_values() {
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
    assert_eq!(embedding_status_provider_label(Some(&status)), "alibaba-cn");

    let alibaba = KnowledgeEmbeddingPresetDto {
        id: "alibaba-cn".to_string(),
        label: "Alibaba Bailian DashScope China".to_string(),
        provider_id: "alibaba-cn".to_string(),
        model: "text-embedding-v4".to_string(),
        dimensions: 1024,
        endpoint_url: Some(
            "https://dashscope.aliyuncs.com/compatible-mode/v1/embeddings".to_string(),
        ),
        api_key_env_vars: vec!["DASHSCOPE_API_KEY".to_string()],
        description: "DashScope China embeddings through the gateway.".to_string(),
    };
    let doubao = KnowledgeEmbeddingPresetDto {
        id: "doubao-volcengine".to_string(),
        label: "Doubao Volcengine OpenAI-compatible".to_string(),
        provider_id: "custom:https://ark.cn-beijing.volces.com/api/v3".to_string(),
        model: "doubao-embedding-text-240715".to_string(),
        dimensions: 2048,
        endpoint_url: Some("https://ark.cn-beijing.volces.com/api/v3/embeddings".to_string()),
        api_key_env_vars: vec!["ARK_API_KEY".to_string()],
        description: "Volcengine Ark embeddings through the gateway.".to_string(),
    };
    assert_eq!(embedding_preset_label(&alibaba), "阿里百炼/DashScope · text-embedding-v4 / 1024维");
    assert_eq!(
        embedding_preset_label(&doubao),
        "豆包/火山方舟 · doubao-embedding-text-240715 / 2048维"
    );

    let result = KnowledgeEmbeddingTestResponse {
        ok: true,
        provider_id: "alibaba-cn".to_string(),
        model: "text-embedding-v4".to_string(),
        configured_dimensions: 1024,
        observed_dimensions: Some(1024),
        latency_ms: Some(50),
        message: "embedding test succeeded".to_string(),
    };
    assert_eq!(embedding_test_result_label(Some(&result)), "通过 · 1024 维 · 50 ms");
}

#[test]
fn score_threshold_and_time_labels_cover_enabled_disabled_and_zero() {
    let mut enabled = dataset("enabled");
    enabled.score_threshold_enabled = true;
    enabled.score_threshold = 0.2;
    assert_eq!(score_threshold_label(&enabled), "0.20");

    let disabled = dataset("disabled");
    assert_eq!(score_threshold_label(&disabled), "关闭");
    assert_eq!(format_time(0), "-");
    assert_ne!(format_time(42), "-");
    assert_ne!(format_dataset_time(42), "--");
    assert_eq!(dataset_title("  "), "未命名知识库");
    assert_eq!(dataset_description("  "), "暂无描述");
}

#[test]
fn style_helpers_cover_light_dark_active_disabled_states() {
    for theme in [Theme::Light, Theme::Dark] {
        if matches!(theme, Theme::Dark) {
            assert!(is_dark_palette(&theme));
        } else {
            assert!(!is_dark_palette(&theme));
        }

        let selected = item_style(&theme, true);
        let idle = item_style(&theme, false);
        assert!(selected.background.is_some());
        assert!(idle.background.is_some());
        assert_eq!(selected.border.width, 1.0);

        let active = option_card_style(&theme, true, true);
        let disabled = option_card_style(&theme, false, false);
        assert_eq!(active.border.width, 2.0);
        assert!(disabled.text_color.is_some());

        let chrome = chrome_style(&theme);
        assert!(chrome.background.is_some());
        assert_eq!(chrome.border.width, 1.0);

        let dataset_card = dataset_card_container_style(&theme);
        assert!(dataset_card.background.is_some());
        assert_eq!(dataset_card.border.width, 1.0);
        let dataset_button = dataset_card_button_style(&theme, button::Status::Active);
        assert!(dataset_button.background.is_none());
        assert!(dataset_title_text_style(&theme).color.is_some());

        let active_segment = segment_style(&theme, button::Status::Active, true);
        let hovered_segment = segment_style(&theme, button::Status::Hovered, false);
        assert!(active_segment.background.is_some());
        assert!(hovered_segment.background.is_some());

        for status in [
            button::Status::Active,
            button::Status::Hovered,
            button::Status::Pressed,
            button::Status::Disabled,
        ] {
            let style = knowledge_toolbar_button_style(&theme, status);
            assert!(style.background.is_some());
            assert_eq!(style.border.width, 1.0);
        }
    }
}

#[test]
fn view_builds_empty_and_notification_states() {
    let mut app = test_app();
    keep_element(super::view(&app));
    keep_element(build_workspace(&app, Size::new(1280.0, 800.0)));
    keep_element(build_workspace(&app, Size::new(720.0, 640.0)));
    keep_element(build_dataset_catalog_view(&app));
    keep_element(build_dataset_catalog_header(&app));
    keep_element(build_dataset_search_input(&app));
    keep_element(build_dataset_list_panel(&app));
    keep_element(build_detail(&app, Size::new(1280.0, 800.0)));
    keep_element(build_create_dataset_modal(&app, None));

    app.knowledge.datasets = vec![dataset("dataset-a")];
    app.knowledge.selected_dataset_id = Some("dataset-a".to_string());
    app.knowledge.show_dataset_detail();
    keep_element(build_workspace(&app, Size::new(720.0, 640.0)));
    app.knowledge.show_dataset_list();
    keep_element(build_workspace(&app, Size::new(720.0, 640.0)));

    app.knowledge.notification = Some("ok".to_string());
    keep_element(super::view(&app));
    app.knowledge.notification_is_error = true;
    keep_element(super::view(&app));
    app.knowledge.create_dataset_modal_open = true;
    keep_element(super::view(&app));
}

#[test]
fn dataset_catalog_builds_loading_empty_filtered_and_populated_states() {
    let mut app = test_app();
    app.knowledge.loading_datasets = true;
    keep_element(build_dataset_catalog_view(&app));
    keep_element(build_dataset_list_panel(&app));
    keep_element(build_create_dataset_panel(&app));
    keep_element(build_create_dataset_card());
    keep_element(knowledge_header_button(
        "刷新",
        Some(crate::app::message::KnowledgeToolMessage::Refresh),
    ));

    app.knowledge.loading_datasets = false;
    keep_element(build_dataset_list_panel(&app));
    app.knowledge.datasets = vec![dataset("dataset-a")];
    app.knowledge.dataset_search_query = "missing".to_string();
    keep_element(build_dataset_catalog_view(&app));
    keep_element(build_dataset_list_panel(&app));
    keep_element(build_no_dataset_result_card());

    app.knowledge.dataset_search_query = "DATASET".to_string();
    app.knowledge.selected_dataset_id = Some("dataset-a".to_string());
    keep_element(build_dataset_catalog_view(&app));
    keep_element(build_dataset_list_panel(&app));
    keep_element(dataset_item(&app, &app.knowledge.datasets[0]));
    keep_element(dataset_tag("知识库"));
}

#[test]
fn create_dataset_controls_build_for_vector_and_rerank_capabilities() {
    let mut app = test_app();
    app.knowledge.create_dataset_tab = crate::app::state::KnowledgeCreateDatasetTab::Indexing;
    keep_element(build_create_dataset_panel(&app));
    app.knowledge.runtime_status = Some(runtime_status(false, false));
    app.knowledge.dataset_indexing_mode = KnowledgeIndexingMode::Economy;
    app.knowledge.dataset_rerank_enabled = true;
    keep_element(build_indexing_mode_section(&app));
    keep_element(build_retrieval_mode_section(&app));
    keep_element(build_dataset_param_section(&app));
    keep_element(build_rerank_toggle(&app));

    app.knowledge.runtime_status = Some(runtime_status(true, true));
    app.knowledge.dataset_indexing_mode = KnowledgeIndexingMode::HighQuality;
    app.knowledge.dataset_retrieval_mode = KnowledgeRetrievalMode::Hybrid;
    keep_element(build_indexing_mode_section(&app));
    keep_element(build_retrieval_mode_section(&app));
    keep_element(build_dataset_param_section(&app));
    keep_element(build_embedding_model_section(&app));
    keep_element(build_embedding_model_picker(&app));

    app.knowledge.create_dataset_tab = crate::app::state::KnowledgeCreateDatasetTab::Retrieval;
    keep_element(build_create_dataset_panel(&app));
}

#[test]
fn detail_builds_all_tabs_and_responsive_document_layouts() {
    let mut app = test_app();
    let mut current = dataset("dataset-a");
    current.description = "description".to_string();
    current.chunking_mode = KnowledgeChunkingMode::ParentChild;
    current.indexing_mode = KnowledgeIndexingMode::HighQuality;
    current.retrieval_mode = KnowledgeRetrievalMode::Hybrid;
    current.score_threshold_enabled = true;
    current.rerank_enabled = true;
    current.rerank_model = Some("rerank-v1".to_string());
    app.knowledge.datasets = vec![current];
    app.knowledge.selected_dataset_id = Some("dataset-a".to_string());
    app.knowledge.runtime_status = Some(runtime_status(true, true));
    app.knowledge.documents = vec![document("document-a")];
    app.knowledge.selected_document_id = Some("document-a".to_string());
    app.knowledge.loaded_chunks = vec![chunk("chunk-a", None)];

    app.knowledge.active_tab = crate::app::state::KnowledgeDetailTab::Documents;
    keep_element(build_detail(&app, Size::new(1280.0, 800.0)));
    keep_element(build_documents_tab(&app, Size::new(1280.0, 800.0)));
    keep_element(build_documents_tab(&app, Size::new(900.0, 800.0)));

    app.knowledge.active_tab = crate::app::state::KnowledgeDetailTab::AddDocument;
    keep_element(build_detail(&app, Size::new(1280.0, 800.0)));
    keep_element(build_add_document_tab(&app));
    keep_element(build_document_input_tabs(&app));
    keep_element(build_upload_panel(&app));
    app.knowledge.document_input_tab = crate::app::state::KnowledgeDocumentInputTab::Text;
    keep_element(build_text_document_panel(&app));
    keep_element(build_upload_panel(&app));

    app.knowledge.active_tab = crate::app::state::KnowledgeDetailTab::Retrieval;
    keep_element(build_detail(&app, Size::new(1280.0, 800.0)));

    app.knowledge.active_tab = crate::app::state::KnowledgeDetailTab::Settings;
    keep_element(build_detail(&app, Size::new(1280.0, 800.0)));
    keep_element(build_settings_tab(&app, app.knowledge.selected_dataset().unwrap()));
}

#[test]
fn upload_panel_builds_file_import_states() {
    let mut app = test_app();
    app.knowledge.selected_dataset_id = Some("dataset-a".to_string());

    keep_element(build_file_import_panel(&app));

    app.knowledge.file_imports = vec![KnowledgeFileImportDraft {
        filename: "guide.md".to_string(),
        mime_type: Some("text/markdown".to_string()),
        byte_len: 1536,
        content_base64: "IyBoZWxsbyB2aWJl".to_string(),
    }];

    assert_eq!(
        file_import_summary(app.knowledge.file_imports.first().unwrap()),
        "1.5 KB · text/markdown"
    );
    assert_eq!(file_imports_summary(&app.knowledge.file_imports), "1.5 KB · text/markdown");
    app.knowledge.file_imports.push(KnowledgeFileImportDraft {
        filename: "guide.pdf".to_string(),
        mime_type: Some("application/pdf".to_string()),
        byte_len: 2048,
        content_base64: "cGRm".to_string(),
    });
    assert_eq!(file_imports_summary(&app.knowledge.file_imports), "3.5 KB · 2 个文件");
    assert_eq!(format_file_size(12), "12 B");
    assert_eq!(format_file_size(1_048_576), "1.0 MB");
    keep_element(build_file_import_panel(&app));

    app.knowledge.importing_file = true;
    keep_element(build_file_import_panel(&app));
    keep_element(build_upload_panel(&app));

    app.knowledge.importing_file = false;
    app.knowledge.import_polling_jobs = vec![crate::app::state::KnowledgeImportPollingJob {
        job_id: "job-a".to_string(),
        dataset_id: "dataset-a".to_string(),
        document_id: "document-a".to_string(),
        filename: "guide.md".to_string(),
        status: KnowledgeIndexJobStatus::Queued,
    }];
    assert_eq!(import_job_status_label(&KnowledgeIndexJobStatus::Queued), "入队");
    assert_eq!(import_job_status_label(&KnowledgeIndexJobStatus::Processing), "索引中");
    assert_eq!(import_job_status_label(&KnowledgeIndexJobStatus::Failed), "失败");
    keep_element(build_import_polling_panel(&app.knowledge.import_polling_jobs));
    keep_element(build_file_import_panel(&app));
}

#[test]
fn documents_table_builds_loading_empty_filtered_and_row_states() {
    let mut app = test_app();
    app.knowledge.loading_documents = true;
    keep_element(build_documents_table(&app));

    app.knowledge.loading_documents = false;
    keep_element(build_documents_table(&app));

    let mut disabled = document("document-b");
    disabled.enabled = false;
    app.knowledge.documents = vec![document("document-a"), disabled];
    app.knowledge.selected_document_id = Some("document-a".to_string());
    app.knowledge.loaded_chunks = vec![chunk("chunk-a", None)];
    app.knowledge.document_search_query = "missing".to_string();
    keep_element(build_documents_table(&app));

    app.knowledge.document_search_query = "DOCUMENT".to_string();
    keep_element(build_documents_table(&app));
    keep_element(document_row(&app, &app.knowledge.documents[0]));

    app.knowledge.deleting = true;
    keep_element(document_row(&app, &app.knowledge.documents[1]));
    app.knowledge.loading_chunks = true;
    keep_element(super::build_chunks_panel(&app));
    keep_element(build_document_preview_drawer(&app));
    app.knowledge.loading_chunks = false;
    keep_element(super::build_chunks_panel(&app));
    keep_element(build_document_preview_drawer(&app));
    keep_element(chunk_preview_card(&app.knowledge.loaded_chunks[0], true));

    app.knowledge.documents =
        (0..25).map(|index| document(&format!("document-{index:02}"))).collect();
    app.knowledge.document_search_query.clear();
    app.knowledge.document_page = 2;
    app.knowledge.document_page_size = 10;
    keep_element(build_documents_table(&app));
    keep_element(build_document_pagination(2, 3, 25, 10));
    keep_element(document_page_button(2, true));
}

#[test]
fn documents_table_filters_status_and_builds_bulk_toolbar() {
    let mut app = test_app();
    let mut completed = document("doc-completed");
    completed.name = "Guide alpha".to_string();
    completed.chunk_count = 12;
    completed.char_count = 12_345;
    completed.recall_count = 1_240;

    let mut disabled = document("doc-disabled");
    disabled.name = "Guide beta".to_string();
    disabled.enabled = false;

    let mut archived = document("doc-archived");
    archived.name = "Guide archive".to_string();
    archived.status = KnowledgeDocumentStatus::Archived;
    archived.enabled = false;
    archived.archived_at_ms = Some(42);

    app.knowledge.documents = vec![completed, disabled, archived];
    app.knowledge.document_search_query = "guide".to_string();
    app.knowledge.document_status_filter = [KnowledgeDocumentStatus::Completed].into();
    app.knowledge.selected_document_ids.insert("doc-completed".to_string());
    app.knowledge.rename_document_id = Some("doc-completed".to_string());
    app.knowledge.rename_document_name_input = "Guide alpha renamed".to_string();

    let filtered = filtered_documents_page(&app);
    assert_eq!(filtered.total_matches, 1);
    assert_eq!(filtered.documents.len(), 1);
    assert_eq!(filtered.documents[0].id, "doc-completed");

    keep_element(build_documents_table(&app));
    keep_element(build_document_bulk_toolbar(&app, &filtered.documents));
    keep_element(build_document_rename_modal(&app, None));
}

#[test]
fn filtered_documents_page_clamps_out_of_range_page_and_preserves_row_offset() {
    let mut app = test_app();
    app.knowledge.documents =
        (0..13).map(|index| document(&format!("document-{index:02}"))).collect();
    app.knowledge.document_page_size = 10;
    app.knowledge.document_page = 99;

    let filtered = filtered_documents_page(&app);

    assert_eq!(filtered.total_matches, 13);
    assert_eq!(filtered.total_pages, 2);
    assert_eq!(filtered.current_page, 2);
    assert_eq!(filtered.page_size, 10);
    assert_eq!(filtered.start_index, 10);
    assert_eq!(filtered.documents.len(), 3);
    assert_eq!(filtered.documents[0].id, "document-10");

    app.knowledge.document_page = 0;
    let filtered = filtered_documents_page(&app);
    assert_eq!(filtered.current_page, 1);
    assert_eq!(filtered.start_index, 0);
    assert_eq!(filtered.documents[0].id, "document-00");
}

#[test]
fn document_toolbar_status_menu_and_pagination_controls_build_for_filter_states() {
    let mut app = test_app();
    app.knowledge.documents = vec![document("doc-a"), document("doc-b")];

    keep_element(build_documents_toolbar(&app));
    keep_element(build_document_status_menu(&app));
    keep_element(document_page_size_selector(20));
    keep_element(document_page_nav_button("上一页", None));
    keep_element(document_page_nav_button("下一页", Some(2)));

    app.knowledge.document_status_menu_open = true;
    app.knowledge.document_status_filter =
        [KnowledgeDocumentStatus::Completed, KnowledgeDocumentStatus::Pending].into();
    keep_element(build_documents_table(&app));
    keep_element(build_documents_toolbar(&app));
    keep_element(build_document_status_menu(&app));

    app.knowledge.document_status_filter = [KnowledgeDocumentStatus::Archived].into();
    keep_element(build_documents_toolbar(&app));
}

#[test]
fn document_modals_and_preview_drawer_cover_missing_busy_and_delete_states() {
    let mut app = test_app();
    app.knowledge.datasets = vec![dataset("dataset-a")];
    app.knowledge.selected_dataset_id = Some("dataset-a".to_string());
    app.knowledge.documents = vec![document("document-a")];

    keep_element(build_document_rename_modal(&app, None));
    app.knowledge.rename_document_id = Some("missing-document".to_string());
    app.knowledge.rename_document_name_input = "  ".to_string();
    keep_element(build_document_rename_modal(&app, None));

    app.knowledge.rename_document_id = Some("document-a".to_string());
    app.knowledge.rename_document_name_input = "Renamed document".to_string();
    app.knowledge.updating_document_id = Some("document-a".to_string());
    keep_element(build_document_rename_modal(&app, None));

    app.knowledge.pending_delete_document_id = Some("document-a".to_string());
    keep_element(build_delete_confirm_modal(&app, None));
    app.knowledge.pending_delete_document_id = None;
    app.knowledge.pending_delete_dataset_id = Some("dataset-a".to_string());
    app.knowledge.deleting = true;
    keep_element(build_delete_confirm_modal(&app, None));

    app.knowledge.document_preview_open = true;
    app.knowledge.selected_document_id = None;
    keep_element(build_document_preview_drawer(&app));
    app.knowledge.selected_document_id = Some("document-a".to_string());
    app.knowledge.loading_chunks = true;
    keep_element(build_document_preview_drawer(&app));
    app.knowledge.loading_chunks = false;
    app.knowledge.loaded_chunks = vec![chunk("chunk-a", Some(0.42))];
    app.knowledge.selected_chunk_id = Some("chunk-a".to_string());
    keep_element(build_document_preview_drawer(&app));
}

#[test]
fn document_pagination_helpers_build_expected_ranges() {
    assert_eq!(document_total_pages(0, 10), 1);
    assert_eq!(document_total_pages(21, 10), 3);
    assert_eq!(document_total_pages(21, 20), 2);
    assert_eq!(
        document_pagination_items(1, 8),
        vec![
            DocumentPageItem::Page(1),
            DocumentPageItem::Page(2),
            DocumentPageItem::Page(3),
            DocumentPageItem::Ellipsis,
            DocumentPageItem::Page(6),
            DocumentPageItem::Page(7),
            DocumentPageItem::Page(8),
        ]
    );
    assert_eq!(
        document_pagination_items(5, 10),
        vec![
            DocumentPageItem::Page(1),
            DocumentPageItem::Ellipsis,
            DocumentPageItem::Page(4),
            DocumentPageItem::Page(5),
            DocumentPageItem::Page(6),
            DocumentPageItem::Ellipsis,
            DocumentPageItem::Page(10),
        ]
    );

    assert_eq!(
        document_pagination_items(2, 4),
        vec![
            DocumentPageItem::Page(1),
            DocumentPageItem::Page(2),
            DocumentPageItem::Page(3),
            DocumentPageItem::Page(4),
        ]
    );
    assert_eq!(
        document_pagination_items(9, 10),
        vec![
            DocumentPageItem::Page(1),
            DocumentPageItem::Page(2),
            DocumentPageItem::Page(3),
            DocumentPageItem::Ellipsis,
            DocumentPageItem::Page(8),
            DocumentPageItem::Page(9),
            DocumentPageItem::Page(10),
        ]
    );
}

#[test]
fn document_page_size_accepts_supported_values_and_rejects_others() {
    let mut app = test_app();
    for supported in [10, 20, 30, 40, 50, 100] {
        app.knowledge.document_page_size = supported;
        assert_eq!(document_page_size(&app), supported);
    }

    app.knowledge.document_page_size = 0;
    assert_eq!(document_page_size(&app), 10);
    app.knowledge.document_page_size = 25;
    assert_eq!(document_page_size(&app), 10);
}

#[test]
fn document_status_helpers_preserve_archives_and_override_disabled_documents() {
    let options = document_status_filter_options();
    assert_eq!(
        options.map(|option| (option.label, option.status)),
        [
            ("可用", KnowledgeDocumentStatus::Completed),
            ("索引中", KnowledgeDocumentStatus::Processing),
            ("入队", KnowledgeDocumentStatus::Pending),
            ("停用", KnowledgeDocumentStatus::Disabled),
            ("归档", KnowledgeDocumentStatus::Archived),
        ]
    );

    let enabled = document("enabled");
    assert_eq!(effective_document_status(&enabled), KnowledgeDocumentStatus::Completed);

    let mut disabled = document("disabled");
    disabled.enabled = false;
    assert_eq!(effective_document_status(&disabled), KnowledgeDocumentStatus::Disabled);

    disabled.status = KnowledgeDocumentStatus::Archived;
    assert_eq!(effective_document_status(&disabled), KnowledgeDocumentStatus::Archived);
}

#[test]
fn document_stats_labels_format_counts_compactly() {
    assert_eq!(compact_document_count_label(0), "0");
    assert_eq!(compact_document_count_label(999), "999");
    assert_eq!(compact_document_count_label(1_200), "1.2K");
    assert_eq!(compact_document_count_label(12_345), "12.3K");
    assert_eq!(compact_document_count_label(2_500_000), "2.5M");
    assert_eq!(compact_document_count_label(1_000), "1K");
    assert_eq!(compact_document_count_label(1_000_000), "1M");
}

#[test]
fn document_styles_and_badges_cover_status_and_interaction_states() {
    for theme in [Theme::Light, Theme::Dark] {
        assert!(document_row_style(&theme, true).background.is_some());
        assert!(document_row_style(&theme, false).background.is_some());
        assert_eq!(
            document_selection_button_style(&theme, button::Status::Active).border.width,
            0.0
        );

        let title_idle = document_title_button_style(&theme, button::Status::Active, false);
        let title_disabled = document_title_button_style(&theme, button::Status::Disabled, false);
        let title_hovered = document_title_button_style(&theme, button::Status::Hovered, false);
        let title_selected = document_title_button_style(&theme, button::Status::Active, true);
        assert_eq!(title_idle.text_color, theme.palette().text);
        assert_eq!(title_disabled.text_color, theme.palette().text);
        assert_ne!(title_idle.text_color, title_hovered.text_color);
        assert_eq!(title_hovered.text_color, title_selected.text_color);

        for (highlighted, has_selection, expected_width) in
            [(true, false, 2.0), (false, true, 1.0), (false, false, 1.0)]
        {
            let style = file_drop_zone_style(&theme, highlighted, has_selection);
            assert!(style.background.is_some());
            assert_eq!(style.border.width, expected_width);
        }

        let status_menu = knowledge_status_menu_style(&theme);
        assert!(status_menu.background.is_some());
        assert_eq!(status_menu.border.width, 1.0);
        assert!(
            knowledge_status_menu_entry_style(&theme, button::Status::Hovered).background.is_some()
        );
        assert!(
            knowledge_status_menu_entry_style(&theme, button::Status::Active).background.is_none()
        );
    }

    for status in [
        KnowledgeDocumentStatus::Pending,
        KnowledgeDocumentStatus::Processing,
        KnowledgeDocumentStatus::Completed,
        KnowledgeDocumentStatus::Failed,
        KnowledgeDocumentStatus::Disabled,
        KnowledgeDocumentStatus::Archived,
    ] {
        let mut value = document("status");
        value.status = status.clone();
        value.enabled = status != KnowledgeDocumentStatus::Disabled;
        let style = document_status_badge_style(&Theme::Light, &value);
        assert!(style.background.is_some());
        assert_eq!(style.border.width, 1.0);
        keep_element(document_status_badge(&value, document_status_label(&status)));
    }
}

#[test]
fn format_time_renders_human_readable_timestamp() {
    assert_eq!(format_time(0), "-");
    assert_eq!(format_time(u64::MAX), "-");
    assert!(format_time(1_700_000_000_000).contains('-'));
    assert!(format_time(1_700_000_000_000).contains(':'));
}

#[test]
fn chunk_preview_title_uses_title_content_then_ordinal_fallback() {
    let mut value = chunk("preview", None);
    value.title = "  Explicit title  ".to_string();
    value.content = "first line\nsecond line".to_string();
    assert_eq!(chunk_preview_title(&value), "Explicit title");

    value.title = "  ".to_string();
    assert_eq!(chunk_preview_title(&value), "first line");

    value.content = " \nsecond line".to_string();
    value.ordinal = 7;
    assert_eq!(chunk_preview_title(&value), "分段 7");
}

#[test]
fn embedding_availability_and_labels_cover_fallback_and_empty_values() {
    let mut app = test_app();
    assert!(!embedding_vector_available(&app));
    assert!(!embedding_hybrid_available(&app));
    assert_eq!(dataset_embedding_options(&app).len(), 1);

    keep_element(build_embedding_model_picker(&app));
    app.knowledge.loading_embedding_presets = true;
    keep_element(build_embedding_model_picker(&app));
    app.knowledge.loading_embedding_presets = false;

    app.knowledge.runtime_status = Some(runtime_status(true, true));
    assert!(embedding_vector_available(&app));
    assert!(embedding_hybrid_available(&app));

    let status = KnowledgeEmbeddingStatusDto {
        provider_id: "  ".to_string(),
        route_hint: Some("  ".to_string()),
        model: "  ".to_string(),
        dimensions: 768,
        api_key_configured: true,
        vector_available: false,
        hybrid_available: false,
        retrieval_modes: vec![],
        notes: BTreeMap::new(),
    };
    app.knowledge.embedding_status = Some(status.clone());
    assert!(!embedding_vector_available(&app));
    assert!(!embedding_hybrid_available(&app));
    assert_eq!(embedding_status_provider_label(Some(&status)), "检测中");
    assert_eq!(embedding_status_model_label(Some(&status)), "检测中");
    assert_eq!(embedding_status_dimensions_label(Some(&status)), "768 维");
    assert_eq!(embedding_status_route_label(Some(&status)), "默认");
    assert_eq!(embedding_status_dimensions_label(None), "检测中");
    assert_eq!(embedding_status_route_label(None), "默认");

    app.knowledge.embedding_presets = vec![KnowledgeEmbeddingPresetDto {
        id: "custom".to_string(),
        label: "Custom Provider".to_string(),
        provider_id: "custom".to_string(),
        model: "custom-embedding".to_string(),
        dimensions: 384,
        endpoint_url: None,
        api_key_env_vars: vec![],
        description: "custom".to_string(),
    }];
    keep_element(build_embedding_model_picker(&app));

    app.knowledge.embedding_status = Some(KnowledgeEmbeddingStatusDto {
        provider_id: "custom".to_string(),
        route_hint: None,
        model: "custom-embedding".to_string(),
        dimensions: 384,
        api_key_configured: true,
        vector_available: true,
        hybrid_available: true,
        retrieval_modes: vec![KnowledgeRetrievalMode::Vector, KnowledgeRetrievalMode::Hybrid],
        notes: BTreeMap::new(),
    });
    let options = dataset_embedding_options(&app);
    assert_eq!(options.len(), 2);
    assert!(options[1].label.contains("锁定当前"));
    assert!(options[1].label.contains("custom-embedding"));

    let failed = KnowledgeEmbeddingTestResponse {
        ok: false,
        provider_id: "custom".to_string(),
        model: "custom-embedding".to_string(),
        configured_dimensions: 384,
        observed_dimensions: None,
        latency_ms: None,
        message: "missing API key".to_string(),
    };
    assert_eq!(embedding_test_result_label(Some(&failed)), "失败 · missing API key · 耗时未知");
}

#[test]
fn file_import_summary_covers_empty_and_unknown_mime_states() {
    assert_eq!(file_imports_summary(&[]), "等待选择");
    let file = KnowledgeFileImportDraft {
        filename: "unknown.bin".to_string(),
        mime_type: None,
        byte_len: 1024,
        content_base64: String::new(),
    };
    assert_eq!(file_import_summary(&file), "1.0 KB · 未知类型");
    assert_eq!(file_imports_summary(&[file]), "1.0 KB · 未知类型");
}

#[test]
fn retrieval_tab_builds_empty_and_result_states() {
    let mut app = test_app();
    keep_element(build_retrieval_tab(&app));

    let mut debug = chunk("scored", Some(0.1234));
    debug.match_type = KnowledgeMatchType::Hybrid;
    debug.matched_content = Some("matched words".to_string());
    debug.context = Some("nearby context".to_string());
    app.knowledge.retrieve_include_context = true;
    app.knowledge.retrieve_results = vec![debug, chunk("unscored", None)];
    keep_element(build_retrieval_tab(&app));
    keep_element(chunk_card(&app.knowledge.retrieve_results[0]));
    keep_element(chunk_card(&app.knowledge.retrieve_results[1]));

    app.knowledge.retrieving = true;
    keep_element(build_retrieval_tab(&app));
}

#[test]
fn knowledge_scrollbar_reserves_requested_gutter() {
    assert_eq!(KNOWLEDGE_SCROLLBAR_WIDTH, 4.0);
    assert_eq!(KNOWLEDGE_SCROLLBAR_GUTTER, 12.0);
    assert!(KNOWLEDGE_SCROLLBAR_GUTTER >= KNOWLEDGE_SCROLLBAR_WIDTH);
}

#[test]
fn small_element_helpers_build_without_panicking() {
    keep_element(section_header("title", "description"));
    keep_element(compact_input_row("Top K", "10", |_| {
        Message::Knowledge(crate::app::message::KnowledgeToolMessage::RetrieveTopKChanged(
            "10".to_string(),
        ))
    }));
    keep_element(option_card(
        "高质量",
        "生成向量索引。",
        crate::app::assets::Icon::Speedometer2,
        true,
        true,
        true,
        Message::Knowledge(crate::app::message::KnowledgeToolMessage::CreateDataset),
    ));
    keep_element(segment_button(
        "设置",
        true,
        Message::Knowledge(crate::app::message::KnowledgeToolMessage::ClearNotification),
    ));
    keep_element(icon_badge(crate::app::assets::Icon::Journals, true));
    keep_element(empty_hint("暂无文档"));
}
