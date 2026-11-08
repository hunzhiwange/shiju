//! 知识库视图测试，覆盖数据集、文档、分块、检索配置和各类弹窗组件。
//! 标签格式、分页边界与明暗主题状态需同时反映网关能力和当前编辑草稿。

use crate::app::message::{KnowledgeToolMessage, knowledge_tool};
use crate::app::state::{
    KnowledgeCreateDatasetTab, KnowledgeDatasetEmbeddingSelection, KnowledgeFileImportDraft,
};
use crate::app::{App, Message};
use iced::advanced::widget::operation::{Focusable, Operation, Scrollable, TextInput};
use iced::advanced::{
    Layout, clipboard, layout, mouse, renderer,
    widget::{Id, Tree},
};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::button;
use iced::{Element, Event, Point, Rectangle, Renderer, Size, Theme, Vector};
use iced_runtime::user_interface::{Cache, UserInterface};
use std::collections::BTreeMap;
use vw_config_types::ui::DesktopLocale;

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
    assert_eq!(
        indexing_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &KnowledgeIndexingMode::Economy
        ),
        "经济"
    );
    assert_eq!(
        indexing_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &KnowledgeIndexingMode::HighQuality
        ),
        "高质量"
    );
    assert_eq!(chunking_label(&KnowledgeChunkingMode::General), "General");
    assert_eq!(
        chunk_type_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &KnowledgeChunkType::ParentText
        ),
        "父块"
    );
    assert_eq!(
        retrieval_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &KnowledgeRetrievalMode::Hybrid
        ),
        "混合检索"
    );
    assert_eq!(
        document_status_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &KnowledgeDocumentStatus::Failed
        ),
        "失败"
    );
    assert_eq!(
        match_type_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &KnowledgeMatchType::Hybrid
        ),
        "混合"
    );
    assert_eq!(
        support_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            true
        ),
        "可用"
    );
    assert_eq!(
        support_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            false
        ),
        "不可用"
    );
    assert_eq!(
        enabled_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            true
        ),
        "启用"
    );
    assert_eq!(
        enabled_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            false
        ),
        "关闭"
    );
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
    assert_eq!(
        embedding_status_provider_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            Some(&status)
        ),
        "alibaba-cn"
    );

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
    assert_eq!(
        embedding_preset_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &alibaba
        ),
        "阿里百炼/DashScope · text-embedding-v4 / 1024维"
    );
    assert_eq!(
        embedding_preset_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &doubao
        ),
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
    assert_eq!(
        embedding_test_result_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            Some(&result)
        ),
        "通过 · 1024 维 · 50 ms"
    );
}

#[test]
fn score_threshold_and_time_labels_cover_enabled_disabled_and_zero() {
    let mut enabled = dataset("enabled");
    enabled.score_threshold_enabled = true;
    enabled.score_threshold = 0.2;
    assert_eq!(
        score_threshold_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &enabled
        ),
        "0.20"
    );

    let disabled = dataset("disabled");
    assert_eq!(
        score_threshold_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &disabled
        ),
        "关闭"
    );
    assert_eq!(format_time(0), "-");
    assert_ne!(format_time(42), "-");
    assert_ne!(format_dataset_time(42), "--");
    assert_eq!(
        dataset_title(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            "  "
        ),
        "未命名知识库"
    );
    assert_eq!(
        dataset_description(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            "  "
        ),
        "暂无描述"
    );
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
    keep_element(build_create_dataset_card(crate::app::i18n::I18n::new(
        vw_config_types::ui::DesktopLocale::SimplifiedChinese,
    )));
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
    keep_element(build_no_dataset_result_card(crate::app::i18n::I18n::new(
        vw_config_types::ui::DesktopLocale::SimplifiedChinese,
    )));

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
        file_import_summary(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            app.knowledge.file_imports.first().unwrap()
        ),
        "1.5 KB · text/markdown"
    );
    assert_eq!(
        file_imports_summary(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &app.knowledge.file_imports
        ),
        "1.5 KB · text/markdown"
    );
    app.knowledge.file_imports.push(KnowledgeFileImportDraft {
        filename: "guide.pdf".to_string(),
        mime_type: Some("application/pdf".to_string()),
        byte_len: 2048,
        content_base64: "cGRm".to_string(),
    });
    assert_eq!(
        file_imports_summary(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &app.knowledge.file_imports
        ),
        "3.5 KB · 2 个文件"
    );
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
    assert_eq!(
        import_job_status_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &KnowledgeIndexJobStatus::Queued
        ),
        "入队"
    );
    assert_eq!(
        import_job_status_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &KnowledgeIndexJobStatus::Processing
        ),
        "索引中"
    );
    assert_eq!(
        import_job_status_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &KnowledgeIndexJobStatus::Failed
        ),
        "失败"
    );
    keep_element(build_import_polling_panel(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        &app.knowledge.import_polling_jobs,
    ));
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
    keep_element(chunk_preview_card(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        &app.knowledge.loaded_chunks[0],
        true,
    ));

    app.knowledge.documents =
        (0..25).map(|index| document(&format!("document-{index:02}"))).collect();
    app.knowledge.document_search_query.clear();
    app.knowledge.document_page = 2;
    app.knowledge.document_page_size = 10;
    keep_element(build_documents_table(&app));
    keep_element(build_document_pagination(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        2,
        3,
        25,
        10,
    ));
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
    let options = document_status_filter_options(crate::app::i18n::I18n::new(
        vw_config_types::ui::DesktopLocale::SimplifiedChinese,
    ));
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
        keep_element(document_status_badge(
            &value,
            document_status_label(
                crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
                &status,
            ),
        ));
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
    assert_eq!(
        chunk_preview_title(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &value
        ),
        "Explicit title"
    );

    value.title = "  ".to_string();
    assert_eq!(
        chunk_preview_title(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &value
        ),
        "first line"
    );

    value.content = " \nsecond line".to_string();
    value.ordinal = 7;
    assert_eq!(
        chunk_preview_title(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &value
        ),
        "分段 7"
    );
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
    assert_eq!(
        embedding_status_provider_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            Some(&status)
        ),
        "检测中"
    );
    assert_eq!(
        embedding_status_model_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            Some(&status)
        ),
        "检测中"
    );
    assert_eq!(
        embedding_status_dimensions_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            Some(&status)
        ),
        "768 维"
    );
    assert_eq!(
        embedding_status_route_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            Some(&status)
        ),
        "默认"
    );
    assert_eq!(
        embedding_status_dimensions_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            None
        ),
        "检测中"
    );
    assert_eq!(
        embedding_status_route_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            None
        ),
        "默认"
    );

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
    assert_eq!(
        embedding_test_result_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            Some(&failed)
        ),
        "失败 · missing API key · 耗时未知"
    );
}

#[test]
fn file_import_summary_covers_empty_and_unknown_mime_states() {
    assert_eq!(
        file_imports_summary(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &[]
        ),
        "等待选择"
    );
    let file = KnowledgeFileImportDraft {
        filename: "unknown.bin".to_string(),
        mime_type: None,
        byte_len: 1024,
        content_base64: String::new(),
    };
    assert_eq!(
        file_import_summary(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &file
        ),
        "1.0 KB · 未知类型"
    );
    assert_eq!(
        file_imports_summary(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &[file]
        ),
        "1.0 KB · 未知类型"
    );
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
    keep_element(chunk_card(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        &app.knowledge.retrieve_results[0],
    ));
    keep_element(chunk_card(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        &app.knowledge.retrieve_results[1],
    ));

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
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
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

const FORM_LOCALES: [DesktopLocale; 3] =
    [DesktopLocale::SimplifiedChinese, DesktopLocale::TraditionalChinese, DesktopLocale::English];
// Centered layer → card → column, for the modal without its optional backdrop view.
const MODAL_CONTENT: &[usize] = &[1, 0, 0];

struct FormUi<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    node: layout::Node,
    size: Size<u32>,
}

impl<'a> FormUi<'a> {
    fn new(mut view: Element<'a, Message>, renderer: &mut Renderer, width: u32) -> Self {
        let size = Size::new(width, 720);
        let bounds = Size::new(width as f32, size.height as f32);
        let mut tree = Tree::new(view.as_widget());
        let node = view.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        let ui = UserInterface::build(view, bounds, Cache::new(), renderer);
        Self { ui, node, size }
    }

    fn bounds(&self, path: &[usize]) -> Rectangle {
        path.iter()
            .fold(Layout::new(&self.node), |layout, &index| {
                layout.children().nth(index).unwrap_or_else(|| panic!("missing layout {path:?}"))
            })
            .bounds()
    }

    fn send(
        &mut self,
        renderer: &mut Renderer,
        events: &[Event],
        cursor: mouse::Cursor,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
        messages
    }

    fn key(&mut self, renderer: &mut Renderer, key: Named) -> Vec<Message> {
        self.send(renderer, &[form_key(Key::Named(key), None)], mouse::Cursor::Unavailable)
    }

    fn click(&mut self, renderer: &mut Renderer, point: Point) -> Vec<Message> {
        self.send(
            renderer,
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(point),
        )
    }

    fn draw(&mut self, renderer: &mut Renderer, theme: &Theme) -> Vec<u8> {
        self.draw_at(renderer, theme, mouse::Cursor::Unavailable)
    }

    fn draw_at(
        &mut self,
        renderer: &mut Renderer,
        theme: &Theme,
        cursor: mouse::Cursor,
    ) -> Vec<u8> {
        use renderer::Headless;
        self.send(
            renderer,
            &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
            cursor,
        );
        self.ui.draw(
            renderer,
            theme,
            &renderer::Style { text_color: theme.palette().text },
            cursor,
        );
        renderer.screenshot(self.size, 1.0, theme.palette().background)
    }

    fn probe(&mut self, renderer: &Renderer) -> FormProbe {
        let mut probe = FormProbe::default();
        self.ui.operate(renderer, &mut probe);
        probe
    }

    fn check_select(
        &mut self,
        renderer: &mut Renderer,
        theme: &Theme,
        trigger: Rectangle,
        selected_index: usize,
        move_to: Named,
    ) -> Vec<Message> {
        assert_eq!(trigger.height, 38.0);
        self.ui.operate(renderer, &mut FormProbe { trigger: Some(trigger), ..Default::default() });
        assert!(self.key(renderer, Named::Enter).is_empty());
        let pixels = self.draw(renderer, theme);
        let menu = self.probe(renderer).menu.expect("Enter should open the shared menu");
        assert!(menu.x >= 0.0 && menu.x + menu.width <= self.size.width as f32);
        assert!(menu.y >= 0.0 && menu.y + menu.height <= self.size.height as f32);

        // The checkmark occupies the trailing 16px slot of the selected 36px row.
        let mark_x = (menu.x + menu.width) as usize - 25;
        let mark_y = menu.y as usize + 18 + selected_index * 38;
        let pixel = |x: usize, y: usize| {
            let start = (y * self.size.width as usize + x) * 4;
            &pixels[start..start + 4]
        };
        let background = pixel(mark_x, mark_y - 9);
        let mark_pixels = (mark_y - 6..=mark_y + 6)
            .flat_map(|y| (mark_x - 7..=mark_x + 7).map(move |x| (x, y)))
            .filter(|&(x, y)| pixel(x, y) != background)
            .count();
        assert!(mark_pixels > 10, "selected option should draw a checkmark");

        assert!(self.key(renderer, Named::Escape).is_empty());
        assert!(self.probe(renderer).menu.is_none());
        assert!(self.key(renderer, Named::Space).is_empty());
        assert!(self.key(renderer, move_to).is_empty());
        let messages = self.key(renderer, Named::Enter);
        assert!(self.probe(renderer).menu.is_none());
        messages
    }
}

#[derive(Default)]
struct FormProbe {
    trigger: Option<Rectangle>,
    menu: Option<Rectangle>,
    inputs: Vec<(String, Rectangle)>,
    labels: Vec<(String, Rectangle)>,
}

impl FormProbe {
    fn label(&self, value: &str, index: usize) -> Rectangle {
        self.labels
            .iter()
            .filter(|(label, _)| label == value)
            .nth(index)
            .unwrap_or_else(|| panic!("missing label {value:?} at index {index}"))
            .1
    }
}

impl Operation for FormProbe {
    fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
        visit(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        if self.trigger == Some(bounds) {
            state.focus();
        }
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        _translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if id == Some(&Id::from("settings-select-menu")) {
            self.menu = Some(bounds);
        }
    }

    fn text_input(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn TextInput) {
        self.inputs.push((state.text().to_owned(), bounds));
    }

    fn text(&mut self, _id: Option<&Id>, bounds: Rectangle, value: &str) {
        self.labels.push((value.into(), bounds));
    }
}

fn form_key(key: Key, text: Option<&str>) -> Event {
    use keyboard::key;
    Event::Keyboard(keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: text.map(Into::into),
        repeat: false,
    })
}

async fn form_renderer() -> Renderer {
    use renderer::Headless;
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    Renderer::new(iced::Font::with_name("Noto Sans CJK SC"), iced::Pixels(14.0), Some("tiny-skia"))
        .await
        .expect("headless renderer")
}

fn form_test_app() -> App {
    let root = tempfile::tempdir().unwrap();
    crate::app::theme_catalog::set_test_themes_root(Some(root.path().to_path_buf()));
    let app = test_app();
    crate::app::theme_catalog::set_test_themes_root(None);
    app
}

#[tokio::test]
async fn knowledge_form_controls_use_shared_embedding_and_page_size_select() {
    let mut renderer = form_renderer().await;
    let mut app = form_test_app();
    app.knowledge.create_dataset_tab = KnowledgeCreateDatasetTab::Indexing;
    app.knowledge.dataset_embedding_selection = KnowledgeDatasetEmbeddingSelection::LockCurrent;
    app.knowledge.embedding_status = Some(KnowledgeEmbeddingStatusDto {
        provider_id: "fixture".into(),
        route_hint: None,
        model: "用于测试的长标签與繁體文字 /fixtures/models/long-name".repeat(6),
        dimensions: 768,
        api_key_configured: false,
        vector_available: true,
        hybrid_available: false,
        retrieval_modes: vec![],
        notes: BTreeMap::new(),
    });

    for locale in FORM_LOCALES {
        app.desktop_locale = locale;
        let i18n = crate::app::i18n::I18n::new(locale);
        for theme in [Theme::Light, Theme::Dark] {
            for width in [420, 960] {
                let mut modal =
                    FormUi::new(build_create_dataset_modal(&app, None), &mut renderer, width);
                // Panel → scrollable → indexing row → right column → embedding section → picker.
                let trigger = modal.bounds(&[MODAL_CONTENT, &[1, 1, 0, 1, 1, 0, 1]].concat());
                assert!(matches!(
                    modal
                        .check_select(&mut renderer, &theme, trigger, 1, Named::ArrowUp)
                        .as_slice(),
                    [Message::Knowledge(KnowledgeToolMessage::DatasetEmbeddingSelectionChanged(
                        KnowledgeDatasetEmbeddingSelection::FollowGlobal
                    ))]
                ));

                let pagination = build_document_pagination(i18n, 6, 15, 150, 10);
                let mut pagination = FormUi::new(pagination, &mut renderer, width);
                let footer = pagination.bounds(&[]);
                let trigger = pagination.bounds(&[0, 0, 2]);
                let pages = pagination.bounds(&[0, 1]);
                for child in [trigger, pages] {
                    assert!(
                        child.x >= footer.x && child.x + child.width <= footer.x + footer.width
                    );
                    assert!(
                        child.y >= footer.y && child.y + child.height <= footer.y + footer.height
                    );
                }
                assert_eq!(pages.y >= trigger.y + trigger.height, width == 420);
                assert!(matches!(
                    pagination
                        .check_select(&mut renderer, &theme, trigger, 0, Named::ArrowDown)
                        .as_slice(),
                    [Message::Knowledge(KnowledgeToolMessage::DocumentPageSizeChanged(20))]
                ));
            }
        }
    }
}

#[tokio::test]
async fn knowledge_form_controls_preserve_dataset_input_messages() {
    let mut renderer = form_renderer().await;

    for (locale, title) in FORM_LOCALES.into_iter().zip(["测试资料", "測試資料", "Sample dataset"])
    {
        for theme in [Theme::Light, Theme::Dark] {
            for width in [420, 960] {
                let mut app = form_test_app();
                app.desktop_locale = locale;
                app.knowledge.dataset_name_input = title.into();
                app.knowledge.dataset_description_input = format!("{title} docs");
                app.knowledge.dataset_indexing_mode = KnowledgeIndexingMode::Economy;
                app.knowledge.dataset_keyword_count_input = "12".into();
                app.knowledge.dataset_top_k_input = "6".into();
                app.knowledge.dataset_score_threshold_input = "0.2".into();
                app.knowledge.dataset_rerank_model_input = "fixture-rerank".into();
                app.knowledge.dataset_rerank_enabled = true;
                app.knowledge.runtime_status = Some(runtime_status(true, true));

                for (tab, expected) in [
                    (
                        KnowledgeCreateDatasetTab::Basic,
                        vec![title.to_owned(), format!("{title} docs")],
                    ),
                    (
                        KnowledgeCreateDatasetTab::Retrieval,
                        ["12", "6", "0.2", "fixture-rerank"].map(str::to_owned).to_vec(),
                    ),
                ] {
                    let _ = knowledge_tool::update(
                        &mut app,
                        KnowledgeToolMessage::SelectCreateDatasetTab(tab),
                    );
                    let changes = {
                        let mut ui = FormUi::new(
                            build_create_dataset_modal(&app, None),
                            &mut renderer,
                            width,
                        );
                        ui.draw(&mut renderer, &theme);
                        let fields = ui.probe(&renderer).inputs;
                        let mut changes = Vec::new();
                        assert_eq!(fields.len(), expected.len());
                        for ((value, bounds), expected_value) in fields.into_iter().zip(expected) {
                            assert_eq!(value, expected_value);
                            assert!(bounds.width > 0.0 && bounds.x + bounds.width <= width as f32);
                            assert!(ui.click(&mut renderer, bounds.center()).is_empty());
                            assert!(ui.key(&mut renderer, Named::End).is_empty());
                            let events = [form_key(Key::Character("1".into()), Some("1"))];
                            let messages =
                                ui.send(&mut renderer, &events, mouse::Cursor::Unavailable);
                            let [Message::Knowledge(change)] = messages.as_slice() else {
                                panic!("expected exactly one input update for {value}");
                            };
                            changes.push(change.clone());
                        }
                        changes
                    };
                    for change in changes {
                        let _ = knowledge_tool::update(&mut app, change);
                    }
                }

                assert_eq!(app.knowledge.dataset_name_input, format!("{title}1"));
                assert_eq!(app.knowledge.dataset_description_input, format!("{title} docs1"));
                assert_eq!(app.knowledge.dataset_keyword_count_input, "121");
                assert_eq!(app.knowledge.dataset_top_k_input, "61");
                assert_eq!(app.knowledge.dataset_score_threshold_input, "0.21");
                assert_eq!(app.knowledge.dataset_rerank_model_input, "fixture-rerank1");

                let _ = knowledge_tool::update(
                    &mut app,
                    KnowledgeToolMessage::SelectCreateDatasetTab(KnowledgeCreateDatasetTab::Basic),
                );
                let mut modal =
                    FormUi::new(build_create_dataset_modal(&app, None), &mut renderer, width);
                modal.draw(&mut renderer, &theme);
                assert_eq!(modal.probe(&renderer).inputs[0].0, format!("{title}1"));
                let cancel = modal.bounds(&[MODAL_CONTENT, &[2, 1]].concat()).center();
                let submit = modal.bounds(&[MODAL_CONTENT, &[2, 2]].concat()).center();
                assert!(matches!(
                    modal.click(&mut renderer, cancel).as_slice(),
                    [Message::Knowledge(KnowledgeToolMessage::CloseCreateDatasetModal)]
                ));
                assert!(matches!(
                    modal.click(&mut renderer, submit).as_slice(),
                    [Message::Knowledge(KnowledgeToolMessage::CreateDataset)]
                ));
            }
        }
    }
}

fn isolated_help_test(test: &str) -> bool {
    const CHILD: &str = "SHIJU_HELP_ROLLOUT_22_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("gateway-client-bootstrap.json"),
        br#"{"host":"[","servers":[]}"#,
    )
    .unwrap();
    let module = module_path!().split_once("::").unwrap().1;
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("{module}::{test}")])
        .env(CHILD, "1")
        .env("HOME", root.path())
        .env("VIBEWINDOW_CONFIG_DIR", root.path())
        .env("VIBEWINDOW_WORKSPACE", root.path())
        .current_dir(root.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed; 0 failed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    false
}

const HELP_LAYOUTS: [(DesktopLocale, Theme, u32); 3] = [
    (DesktopLocale::SimplifiedChinese, Theme::Light, 420),
    (DesktopLocale::English, Theme::Dark, 420),
    (DesktopLocale::TraditionalChinese, Theme::Dark, 640),
];

fn check_form_help(
    ui: &mut FormUi<'_>,
    renderer: &mut Renderer,
    app: &App,
    title: &str,
    description: &str,
    index: usize,
) -> Point {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let title = i18n.__(title);
    let description = i18n.__(description);
    let probe = ui.probe(renderer);
    assert!(
        probe.labels.iter().all(|(label, _)| label != description),
        "{title}: static explanation is still inline: {description}"
    );
    let label = probe.label(title, index);
    let icon = Point::new(label.x + label.width + 15.0, label.center_y());
    assert!(
        Rectangle::with_size(Size::new(ui.size.width as f32, ui.size.height as f32)).contains(icon)
    );
    let idle = ui.draw(renderer, &app.app_theme);
    let cursor = mouse::Cursor::Available(icon);
    let messages =
        ui.send(renderer, &[Event::Mouse(mouse::Event::CursorMoved { position: icon })], cursor);
    assert!(form_actions(messages).is_empty(), "{title}: hovering help emitted a form action");
    let hover = ui.draw_at(renderer, &app.app_theme, cursor);
    assert!(idle != hover, "{title}: hovering over the question mark must open help");
    assert!(
        form_actions(ui.click(renderer, icon)).is_empty(),
        "{title}: pinning help emitted a form action"
    );
    let pinned = ui.draw(renderer, &app.app_theme);
    assert!(idle != pinned, "{title}: help must stay open after clicking");
    assert!(ui.key(renderer, Named::Escape).is_empty());
    assert!(ui.draw(renderer, &app.app_theme) == idle, "{title}: Escape must close only help");

    if let Some(directory) = std::env::var_os("SHIJU_KNOWLEDGE_HELP_PREVIEWS") {
        let directory = std::path::PathBuf::from(directory);
        let source = format!(
            "{:?}-{:?}-{:?}-{title}",
            app.desktop_locale, app.knowledge.create_dataset_tab, app.knowledge.active_tab
        );
        for (phase, pixels) in [("idle", idle), ("hover", hover), ("pinned", pinned)] {
            image::save_buffer(
                directory.join(format!("{source}-{phase}.png")),
                &pixels,
                ui.size.width,
                ui.size.height,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
    icon
}

fn form_actions(mut messages: Vec<Message>) -> Vec<Message> {
    // The existing drop area reports pointer exits even when no file is being dragged.
    messages.retain(|message| {
        !matches!(message, Message::Knowledge(KnowledgeToolMessage::ImportDropHoverChanged(false)))
    });
    messages
}

fn check_help_menu(ui: &mut FormUi<'_>, renderer: &mut Renderer, icon: Point, description: &str) {
    use crate::app::message::ViewMessage;

    assert!(ui.click(renderer, icon).is_empty());
    let body = Point::new(icon.x + 29.0, icon.y + 5.0);
    let line_height = iced::advanced::text::LineHeight::default().to_absolute(12.0.into());
    let item_height = f32::from(line_height) + 10.0;
    for index in [0, 3] {
        assert!(
            ui.send(
                renderer,
                &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
                mouse::Cursor::Available(body),
            )
            .is_empty()
        );
        let item = Point::new(body.x + 24.0, body.y + 3.0 + item_height * (index as f32 + 0.5));
        let messages = ui.click(renderer, item);
        match (index, messages.as_slice()) {
            (0, [Message::CopyCode(value)]) => assert_eq!(value, description),
            (3, [Message::View(ViewMessage::OpenUrlExternal(url))]) => {
                assert_eq!(
                    url,
                    &format!(
                        "https://www.google.com/search?q={}",
                        urlencoding::encode(description)
                    )
                );
            }
            _ => panic!("unexpected help action: {messages:?}"),
        }
    }
    assert!(ui.key(renderer, Named::Escape).is_empty());
}

#[tokio::test]
async fn help_rollout_22_renders_contextual_help() {
    if !isolated_help_test("help_rollout_22_renders_contextual_help") {
        return;
    }
    let mut renderer = form_renderer().await;
    let mut app = form_test_app();
    app.knowledge.create_dataset_modal_open = true;
    app.knowledge.dataset_name_input = "fixture-dataset".into();
    app.knowledge.dataset_indexing_mode = KnowledgeIndexingMode::Economy;
    app.knowledge.dataset_keyword_count_input = "17".into();

    for (locale, theme, width) in HELP_LAYOUTS {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let i18n = crate::app::i18n::I18n::new(locale);

        app.knowledge.create_dataset_tab = KnowledgeCreateDatasetTab::Basic;
        let mut ui = FormUi::new(super::view(&app), &mut renderer, width);
        let intro = "按模块完成基础信息、索引与检索配置。";
        let icon = check_form_help(&mut ui, &mut renderer, &app, "创建知识库", intro, 0);
        if width == 640 {
            check_help_menu(&mut ui, &mut renderer, icon, i18n.__(intro));
        }
        check_form_help(
            &mut ui,
            &mut renderer,
            &app,
            "基础信息",
            "名称用于识别知识库，描述用于说明内容范围。",
            usize::from(i18n.__("基础信息") == "基础信息"),
        );
        let probe = ui.probe(&renderer);
        assert_eq!(probe.inputs[0].0, "fixture-dataset");
        probe.label(i18n.__("填写名称与用途"), 0);
        assert!(matches!(
            ui.click(&mut renderer, probe.label(i18n.__("取消"), 0).center()).as_slice(),
            [Message::Knowledge(KnowledgeToolMessage::CloseCreateDatasetModal)]
        ));
        assert!(matches!(
            ui.click(&mut renderer, probe.label(i18n.__("创建知识库"), 1).center()).as_slice(),
            [Message::Knowledge(KnowledgeToolMessage::CreateDataset)]
        ));
        drop(ui);
        app.knowledge.creating_dataset = true;
        let mut ui = FormUi::new(super::view(&app), &mut renderer, width);
        let busy = ui.probe(&renderer).label(i18n.__("创建中"), 0).center();
        assert!(ui.click(&mut renderer, busy).is_empty());
        drop(ui);
        app.knowledge.creating_dataset = false;

        app.knowledge.create_dataset_tab = KnowledgeCreateDatasetTab::Indexing;
        let mut ui = FormUi::new(super::view(&app), &mut renderer, width);
        check_form_help(&mut ui, &mut renderer, &app, "分段模式", "选择文档入库结构。", 0);
        check_form_help(
            &mut ui,
            &mut renderer,
            &app,
            "嵌入模型",
            "跟随全局配置，或锁定当前已生效模型。",
            0,
        );
        let probe = ui.probe(&renderer);
        probe.label(i18n.__("高质量需要嵌入能力。"), 0);
        let disabled = probe.label(i18n.__("高质量"), 0).center();
        assert!(ui.click(&mut renderer, disabled).is_empty());
        assert!(matches!(
            ui.click(&mut renderer, probe.label("Parent-Child", 0).center()).as_slice(),
            [Message::Knowledge(KnowledgeToolMessage::DatasetChunkingModeChanged(
                KnowledgeChunkingMode::ParentChild
            ))]
        ));
        drop(ui);

        app.knowledge.create_dataset_tab = KnowledgeCreateDatasetTab::Retrieval;
        let mut ui = FormUi::new(super::view(&app), &mut renderer, width);
        check_form_help(
            &mut ui,
            &mut renderer,
            &app,
            "检索设置",
            "选择召回策略。",
            usize::from(i18n.__("检索设置") == "检索设置"),
        );
        check_form_help(&mut ui, &mut renderer, &app, "参数", "默认召回参数。", 0);
        assert_eq!(ui.probe(&renderer).inputs[0].0, "17");
    }
}

#[tokio::test]
async fn help_rollout_22_document_forms_render_contextual_help() {
    use crate::app::state::{KnowledgeDetailTab, KnowledgeDocumentInputTab};

    if !isolated_help_test("help_rollout_22_document_forms_render_contextual_help") {
        return;
    }
    let mut renderer = form_renderer().await;
    let mut app = form_test_app();
    app.knowledge.showing_dataset_list = false;
    app.knowledge.datasets = vec![dataset("fixture-dataset")];
    app.knowledge.selected_dataset_id = Some("fixture-dataset".into());
    app.knowledge.notification = Some("fixture-save-feedback".into());
    app.knowledge.document_name_input = "fixture-document".into();
    app.knowledge.embedding_status = Some(KnowledgeEmbeddingStatusDto {
        provider_id: "fixture-provider".into(),
        model: "fixture-embed".into(),
        dimensions: 768,
        route_hint: None,
        api_key_configured: false,
        vector_available: true,
        hybrid_available: false,
        retrieval_modes: vec![],
        notes: BTreeMap::new(),
    });

    for (locale, theme, width) in HELP_LAYOUTS {
        app.desktop_locale = locale;
        app.app_theme = theme;
        app.knowledge.notification_is_error = locale == DesktopLocale::English;
        let i18n = crate::app::i18n::I18n::new(locale);
        app.knowledge.active_tab = KnowledgeDetailTab::AddDocument;
        app.knowledge.document_input_tab = KnowledgeDocumentInputTab::Files;
        let mut ui = FormUi::new(super::view(&app), &mut renderer, width);
        check_form_help(
            &mut ui,
            &mut renderer,
            &app,
            "添加文档",
            "支持拖拽文件上传，或直接粘贴长文本入库。",
            usize::from(i18n.__("添加文档") == "添加文档"),
        );
        check_form_help(
            &mut ui,
            &mut renderer,
            &app,
            "文件入库",
            "可以手动选择文件，也可以拖到下方区域。",
            0,
        );
        let probe = ui.probe(&renderer);
        probe.label("fixture-save-feedback", 0);
        let pick = probe.label(i18n.__("手动选择文件"), 0).center();
        assert!(matches!(
            form_actions(ui.click(&mut renderer, pick)).as_slice(),
            [Message::Knowledge(KnowledgeToolMessage::PickImportFile)]
        ));
        drop(ui);
        app.knowledge.document_input_tab = KnowledgeDocumentInputTab::Text;
        let mut ui = FormUi::new(super::view(&app), &mut renderer, width);
        check_form_help(
            &mut ui,
            &mut renderer,
            &app,
            "文本入库",
            "适合长说明、Markdown、提示词、知识笔记直接粘贴入库。",
            0,
        );
        let probe = ui.probe(&renderer);
        assert_eq!(probe.inputs[0].0, "fixture-document");
        probe.label(i18n.__("内容输入区"), 0);
        drop(ui);

        app.knowledge.active_tab = KnowledgeDetailTab::Settings;
        let mut ui = FormUi::new(super::view(&app), &mut renderer, width);
        check_form_help(
            &mut ui,
            &mut renderer,
            &app,
            "嵌入模型",
            "跟随全局配置，或锁定当前已生效模型。",
            0,
        );
        let probe = ui.probe(&renderer);
        probe.label("fixture-dataset", 0);
        probe.label("fixture-embed", 0);
        probe.label("General", 1);
        let test = probe.label(i18n.__("测试连接"), 0).center();
        assert!(matches!(
            ui.click(&mut renderer, test).as_slice(),
            [Message::Knowledge(KnowledgeToolMessage::RunEmbeddingTest)]
        ));
    }
}
