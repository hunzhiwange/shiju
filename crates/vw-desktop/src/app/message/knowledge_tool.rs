//! 知识库工作台消息处理。

use crate::app::state::{
    KnowledgeCreateDatasetTab, KnowledgeDatasetEmbeddingSelection, KnowledgeDetailTab,
    KnowledgeDocumentInputTab, KnowledgeFileImportDraft, KnowledgeImportPollingJob,
};
use crate::app::{App, Message};
#[cfg(not(target_arch = "wasm32"))]
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use iced::Task;
use iced::widget::text_editor;
use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;
use vw_gateway_client::vw_api_types::knowledge::{
    KnowledgeDocumentPatchRequest, KnowledgeDocumentsBatchAction,
    KnowledgeDocumentsBatchActionRequest,
};
use vw_gateway_client::{
    KnowledgeChunkDto, KnowledgeChunkingMode, KnowledgeDatasetCreateRequest, KnowledgeDatasetDto,
    KnowledgeDocumentCreateRequest, KnowledgeDocumentDto, KnowledgeDocumentImportQueuedResponse,
    KnowledgeDocumentImportRequest, KnowledgeDocumentReindexRequest, KnowledgeDocumentStatus,
    KnowledgeEmbeddingPresetDto, KnowledgeEmbeddingStatusDto, KnowledgeEmbeddingTestRequest,
    KnowledgeEmbeddingTestResponse, KnowledgeIndexJobDto, KnowledgeIndexJobStatus,
    KnowledgeIndexingMode, KnowledgeRetrievalMode, KnowledgeRetrieveRequest,
    KnowledgeRetrieveResponse, KnowledgeRuntimeStatus,
};

const IMPORT_JOB_POLL_INTERVAL: Duration = Duration::from_secs(5);
const KNOWLEDGE_NOTIFICATION_DURATION: Duration = Duration::from_secs(3);

#[derive(Debug, Clone)]
pub enum KnowledgeToolMessage {
    Refresh,
    StatusLoaded(Result<KnowledgeRuntimeStatus, String>),
    EmbeddingStatusLoaded(Result<KnowledgeEmbeddingStatusDto, String>),
    EmbeddingPresetsLoaded(Result<Vec<KnowledgeEmbeddingPresetDto>, String>),
    DatasetsLoaded(Result<Vec<KnowledgeDatasetDto>, String>),
    DocumentsLoaded(String, Result<Vec<KnowledgeDocumentDto>, String>),
    LoadDocumentChunks(String),
    ChunksLoaded(String, Result<Vec<KnowledgeChunkDto>, String>),
    SelectDataset(String),
    ShowDatasetList,
    SelectChunk(String),
    SelectTab(KnowledgeDetailTab),
    SelectDocumentInputTab(KnowledgeDocumentInputTab),
    CloseDocumentPreview,
    DatasetSearchChanged(String),
    DocumentSearchChanged(String),
    ToggleDocumentStatusFilter(KnowledgeDocumentStatus),
    ClearDocumentStatusFilter,
    ToggleDocumentStatusMenu,
    CloseDocumentStatusMenu,
    DocumentPageChanged(usize),
    DocumentPageSizeChanged(usize),
    ToggleDocumentSelection(String),
    ToggleSelectAllVisibleDocuments,
    InvertVisibleDocumentSelection,
    ClearDocumentSelection,
    OpenRenameDocument(String),
    CloseRenameDocument,
    RenameDocumentDraftChanged(String),
    SubmitDocumentRename { document_id: String, name: String },
    DocumentUpdated(Result<KnowledgeDocumentDto, String>),
    SetDocumentEnabled { document_id: String, enabled: bool },
    SetDocumentArchived { document_id: String, archived: bool },
    SubmitBatchDocumentAction { action: KnowledgeDocumentsBatchAction },
    DocumentsBatchUpdated(Result<Vec<KnowledgeDocumentDto>, String>),
    DatasetNameChanged(String),
    DatasetDescriptionChanged(String),
    SelectCreateDatasetTab(KnowledgeCreateDatasetTab),
    DatasetChunkingModeChanged(KnowledgeChunkingMode),
    DatasetIndexingModeChanged(KnowledgeIndexingMode),
    DatasetRetrievalModeChanged(KnowledgeRetrievalMode),
    DatasetEmbeddingSelectionChanged(KnowledgeDatasetEmbeddingSelection),
    DatasetKeywordCountChanged(String),
    DatasetTopKChanged(String),
    DatasetScoreThresholdEnabledChanged(bool),
    DatasetScoreThresholdChanged(String),
    DatasetRerankEnabledChanged(bool),
    DatasetRerankModelChanged(String),
    OpenCreateDatasetModal,
    CloseCreateDatasetModal,
    CreateDataset,
    DatasetCreated(Result<KnowledgeDatasetDto, String>),
    OpenDeleteDatasetConfirm,
    OpenDeleteDatasetConfirmFor(String),
    ToggleDatasetMenu(String),
    CloseDatasetMenu,
    CopyDatasetId(String),
    OpenDeleteDocumentConfirm(String),
    CloseDeleteConfirm,
    DeleteSelectedDataset,
    DatasetDeleted(Result<KnowledgeDatasetDto, String>),
    DocumentNameChanged(String),
    DocumentContentAction(text_editor::Action),
    CreateDocument,
    DocumentCreated(String, Result<KnowledgeDocumentDto, String>),
    PickImportFile,
    ImportFileSelected(Result<Vec<KnowledgeFileImportDraft>, String>),
    ClearImportFile,
    ImportSelectedFile,
    ImportDropHoverChanged(bool),
    ImportDroppedFiles,
    FileImported(String, Result<Vec<KnowledgeDocumentImportQueuedResponse>, String>),
    PollImportJobs,
    ImportJobPolled(Result<KnowledgeIndexJobDto, String>),
    DeleteDocument(String),
    DocumentDeleted(Result<KnowledgeDocumentDto, String>),
    ReindexDocument(String),
    ReindexFinished(Result<KnowledgeDocumentDto, String>),
    RetrieveQueryChanged(String),
    RetrieveTopKChanged(String),
    RetrieveScoreThresholdEnabledChanged(bool),
    RetrieveScoreThresholdChanged(String),
    RetrieveIncludeContextChanged(bool),
    Retrieve,
    RetrieveFinished(Result<KnowledgeRetrieveResponse, String>),
    RunEmbeddingTest,
    EmbeddingTestFinished(Result<KnowledgeEmbeddingTestResponse, String>),
    ClearDocumentDraft,
    ClearNotification,
}

pub fn update(app: &mut App, message: KnowledgeToolMessage) -> Task<Message> {
    match message {
        KnowledgeToolMessage::Refresh => {
            app.knowledge.loading_status = true;
            app.knowledge.loading_embedding_status = true;
            app.knowledge.loading_embedding_presets = true;
            app.knowledge.loading_datasets = true;
            Task::batch([
                load_status_task(),
                load_embedding_status_task(),
                load_embedding_presets_task(),
                load_datasets_task(),
            ])
        }
        KnowledgeToolMessage::StatusLoaded(result) => {
            app.knowledge.loading_status = false;
            match result {
                Ok(status) => {
                    app.knowledge.runtime_status = Some(status);
                    Task::none()
                }
                Err(error) => set_error(app, error),
            }
        }
        KnowledgeToolMessage::EmbeddingStatusLoaded(result) => {
            app.knowledge.loading_embedding_status = false;
            match result {
                Ok(status) => {
                    app.knowledge.embedding_status = Some(status);
                    Task::none()
                }
                Err(error) => set_error(app, error),
            }
        }
        KnowledgeToolMessage::EmbeddingPresetsLoaded(result) => {
            app.knowledge.loading_embedding_presets = false;
            match result {
                Ok(presets) => {
                    app.knowledge.embedding_presets = presets;
                    Task::none()
                }
                Err(error) => set_error(app, error),
            }
        }
        KnowledgeToolMessage::DatasetsLoaded(result) => {
            app.knowledge.loading_datasets = false;
            match result {
                Ok(datasets) => {
                    app.knowledge.datasets = datasets;
                    app.knowledge.select_first_dataset_if_needed();
                    if app.knowledge.selected_dataset_id.is_none() {
                        app.knowledge.show_dataset_list();
                    }
                    app.knowledge.sync_retrieve_defaults_from_selected_dataset();
                    if let Some(dataset_id) = app.knowledge.selected_dataset_id.clone() {
                        app.knowledge.loading_documents = true;
                        load_documents_task(dataset_id)
                    } else {
                        app.knowledge.documents.clear();
                        app.knowledge.loaded_chunks.clear();
                        app.knowledge.selected_document_id = None;
                        app.knowledge.selected_chunk_id = None;
                        app.knowledge.document_preview_open = false;
                        app.knowledge.document_page = 1;
                        Task::none()
                    }
                }
                Err(error) => set_error(app, error),
            }
        }
        KnowledgeToolMessage::DocumentsLoaded(dataset_id, result) => {
            app.knowledge.loading_documents = false;
            if app.knowledge.selected_dataset_id.as_deref() != Some(dataset_id.as_str()) {
                return Task::none();
            }
            match result {
                Ok(documents) => {
                    app.knowledge.documents = documents;
                    app.knowledge.prune_document_selection();
                    let selected_document_id =
                        app.knowledge.selected_document_id.clone().filter(|selected_id| {
                            app.knowledge
                                .documents
                                .iter()
                                .any(|document| document.id.as_str() == selected_id.as_str())
                        });
                    if let Some(document_id) =
                        selected_document_id.filter(|_| app.knowledge.document_preview_open)
                    {
                        app.knowledge.loading_chunks = true;
                        load_chunks_task(document_id)
                    } else {
                        app.knowledge.document_preview_open = false;
                        app.knowledge.loaded_chunks.clear();
                        app.knowledge.selected_document_id = None;
                        app.knowledge.selected_chunk_id = None;
                        Task::none()
                    }
                }
                Err(error) => set_error(app, error),
            }
        }
        KnowledgeToolMessage::LoadDocumentChunks(document_id) => {
            if app.knowledge.loading_chunks
                && app.knowledge.selected_document_id.as_deref() == Some(document_id.as_str())
            {
                return Task::none();
            }
            app.knowledge.selected_document_id = Some(document_id.clone());
            app.knowledge.selected_chunk_id = None;
            app.knowledge.loaded_chunks.clear();
            app.knowledge.document_preview_open = true;
            app.knowledge.loading_chunks = true;
            load_chunks_task(document_id)
        }
        KnowledgeToolMessage::ChunksLoaded(document_id, result) => {
            if app.knowledge.selected_document_id.as_deref() != Some(document_id.as_str()) {
                return Task::none();
            }
            app.knowledge.loading_chunks = false;
            match result {
                Ok(chunks) => {
                    app.knowledge.loaded_chunks = chunks;
                    app.knowledge.select_first_chunk_if_needed();
                    Task::none()
                }
                Err(error) => set_error(app, error),
            }
        }
        KnowledgeToolMessage::SelectDataset(dataset_id) => {
            app.knowledge.close_dataset_menu();
            app.knowledge.selected_dataset_id = Some(dataset_id.clone());
            app.knowledge.show_dataset_detail();
            app.knowledge.sync_retrieve_defaults_from_selected_dataset();
            app.knowledge.documents.clear();
            app.knowledge.loaded_chunks.clear();
            app.knowledge.retrieve_results.clear();
            app.knowledge.selected_document_id = None;
            app.knowledge.selected_chunk_id = None;
            app.knowledge.document_preview_open = false;
            app.knowledge.document_page = 1;
            app.knowledge.clear_document_selection();
            app.knowledge.close_document_rename_modal();
            app.knowledge.loading_documents = true;
            app.knowledge.loading_chunks = false;
            load_documents_task(dataset_id)
        }
        KnowledgeToolMessage::ShowDatasetList => {
            app.knowledge.show_dataset_list();
            app.knowledge.document_preview_open = false;
            Task::none()
        }
        KnowledgeToolMessage::SelectChunk(chunk_id) => {
            if app.knowledge.loaded_chunks.iter().any(|chunk| chunk.id == chunk_id) {
                app.knowledge.selected_chunk_id = Some(chunk_id);
            }
            Task::none()
        }
        KnowledgeToolMessage::SelectTab(tab) => {
            app.knowledge.active_tab = tab;
            if tab != KnowledgeDetailTab::Documents {
                app.knowledge.document_preview_open = false;
            }
            Task::none()
        }
        KnowledgeToolMessage::SelectDocumentInputTab(tab) => {
            app.knowledge.document_input_tab = tab;
            app.knowledge.file_drop_hovered = false;
            Task::none()
        }
        KnowledgeToolMessage::CloseDocumentPreview => {
            app.knowledge.document_preview_open = false;
            app.knowledge.selected_document_id = None;
            app.knowledge.selected_chunk_id = None;
            app.knowledge.loaded_chunks.clear();
            app.knowledge.loading_chunks = false;
            Task::none()
        }
        KnowledgeToolMessage::DatasetSearchChanged(value) => {
            app.knowledge.dataset_search_query = value;
            Task::none()
        }
        KnowledgeToolMessage::DocumentSearchChanged(value) => {
            app.knowledge.document_search_query = value;
            app.knowledge.document_page = 1;
            Task::none()
        }
        KnowledgeToolMessage::ToggleDocumentStatusFilter(status) => {
            app.knowledge.toggle_document_status_filter(status);
            Task::none()
        }
        KnowledgeToolMessage::ClearDocumentStatusFilter => {
            app.knowledge.clear_document_status_filter();
            Task::none()
        }
        KnowledgeToolMessage::ToggleDocumentStatusMenu => {
            app.knowledge.toggle_document_status_menu();
            Task::none()
        }
        KnowledgeToolMessage::CloseDocumentStatusMenu => {
            app.knowledge.close_document_status_menu();
            Task::none()
        }
        KnowledgeToolMessage::DocumentPageChanged(page) => {
            app.knowledge.document_page = page.max(1);
            Task::none()
        }
        KnowledgeToolMessage::DocumentPageSizeChanged(page_size) => {
            app.knowledge.document_page_size = sanitize_document_page_size(page_size);
            app.knowledge.document_page = 1;
            Task::none()
        }
        KnowledgeToolMessage::ToggleDocumentSelection(document_id) => {
            app.knowledge.toggle_document_selection(&document_id);
            Task::none()
        }
        KnowledgeToolMessage::ToggleSelectAllVisibleDocuments => {
            app.knowledge.toggle_select_all_visible_documents();
            Task::none()
        }
        KnowledgeToolMessage::InvertVisibleDocumentSelection => {
            app.knowledge.invert_visible_document_selection();
            Task::none()
        }
        KnowledgeToolMessage::ClearDocumentSelection => {
            app.knowledge.clear_document_selection();
            Task::none()
        }
        KnowledgeToolMessage::OpenRenameDocument(document_id) => {
            app.knowledge.open_document_rename_modal(&document_id);
            Task::none()
        }
        KnowledgeToolMessage::CloseRenameDocument => {
            app.knowledge.close_document_rename_modal();
            Task::none()
        }
        KnowledgeToolMessage::RenameDocumentDraftChanged(value) => {
            app.knowledge.rename_document_name_input = value;
            Task::none()
        }
        KnowledgeToolMessage::SubmitDocumentRename { document_id, name } => {
            submit_document_update(app, document_id, Some(name.as_str()), None, None)
        }
        KnowledgeToolMessage::DocumentUpdated(result) => handle_document_updated(app, result),
        KnowledgeToolMessage::SetDocumentEnabled { document_id, enabled } => {
            submit_document_update(app, document_id, None, Some(enabled), None)
        }
        KnowledgeToolMessage::SetDocumentArchived { document_id, archived } => {
            submit_document_update(app, document_id, None, None, Some(archived))
        }
        KnowledgeToolMessage::SubmitBatchDocumentAction { action } => {
            submit_batch_document_action(app, action)
        }
        KnowledgeToolMessage::DocumentsBatchUpdated(result) => {
            handle_documents_batch_updated(app, result)
        }
        KnowledgeToolMessage::DatasetNameChanged(value) => {
            app.knowledge.dataset_name_input = value;
            Task::none()
        }
        KnowledgeToolMessage::DatasetDescriptionChanged(value) => {
            app.knowledge.dataset_description_input = value;
            Task::none()
        }
        KnowledgeToolMessage::SelectCreateDatasetTab(tab) => {
            app.knowledge.create_dataset_tab = tab;
            Task::none()
        }
        KnowledgeToolMessage::DatasetChunkingModeChanged(mode) => {
            app.knowledge.dataset_chunking_mode = mode;
            Task::none()
        }
        KnowledgeToolMessage::DatasetIndexingModeChanged(mode) => {
            if mode == KnowledgeIndexingMode::Economy {
                app.knowledge.dataset_retrieval_mode = KnowledgeRetrievalMode::FullText;
                app.knowledge.dataset_rerank_enabled = false;
                app.knowledge.dataset_score_threshold_enabled = false;
            }
            app.knowledge.dataset_indexing_mode = mode;
            Task::none()
        }
        KnowledgeToolMessage::DatasetRetrievalModeChanged(mode) => {
            if mode != KnowledgeRetrievalMode::FullText {
                app.knowledge.dataset_indexing_mode = KnowledgeIndexingMode::HighQuality;
            }
            app.knowledge.dataset_retrieval_mode = mode;
            Task::none()
        }
        KnowledgeToolMessage::DatasetEmbeddingSelectionChanged(selection) => {
            app.knowledge.dataset_embedding_selection = selection;
            Task::none()
        }
        KnowledgeToolMessage::DatasetKeywordCountChanged(value) => {
            app.knowledge.dataset_keyword_count_input = value;
            Task::none()
        }
        KnowledgeToolMessage::DatasetTopKChanged(value) => {
            app.knowledge.dataset_top_k_input = value;
            Task::none()
        }
        KnowledgeToolMessage::DatasetScoreThresholdEnabledChanged(value) => {
            app.knowledge.dataset_score_threshold_enabled = value;
            Task::none()
        }
        KnowledgeToolMessage::DatasetScoreThresholdChanged(value) => {
            app.knowledge.dataset_score_threshold_input = value;
            Task::none()
        }
        KnowledgeToolMessage::DatasetRerankEnabledChanged(value) => {
            app.knowledge.dataset_rerank_enabled = value;
            if value && app.knowledge.dataset_rerank_model_input.trim().is_empty() {
                app.knowledge.dataset_rerank_model_input = "local-rerank-v1".to_string();
            }
            Task::none()
        }
        KnowledgeToolMessage::DatasetRerankModelChanged(value) => {
            app.knowledge.dataset_rerank_model_input = value;
            Task::none()
        }
        KnowledgeToolMessage::OpenCreateDatasetModal => {
            app.knowledge.create_dataset_modal_open = true;
            app.knowledge.create_dataset_tab = KnowledgeCreateDatasetTab::Basic;
            Task::none()
        }
        KnowledgeToolMessage::CloseCreateDatasetModal => {
            app.knowledge.create_dataset_modal_open = false;
            Task::none()
        }
        KnowledgeToolMessage::CreateDataset => create_dataset(app),
        KnowledgeToolMessage::DatasetCreated(result) => {
            app.knowledge.creating_dataset = false;
            match result {
                Ok(dataset) => {
                    app.knowledge.selected_dataset_id = Some(dataset.id.clone());
                    app.knowledge.show_dataset_detail();
                    app.knowledge.create_dataset_modal_open = false;
                    app.knowledge.dataset_name_input.clear();
                    app.knowledge.dataset_description_input.clear();
                    app.knowledge.datasets.insert(0, dataset);
                    app.knowledge.sync_retrieve_defaults_from_selected_dataset();
                    Task::batch([
                        set_success(app, "知识库已创建"),
                        load_datasets_task(),
                        load_status_task(),
                    ])
                }
                Err(error) => set_error(app, error),
            }
        }
        KnowledgeToolMessage::OpenDeleteDatasetConfirm => {
            app.knowledge.pending_delete_document_id = None;
            app.knowledge.pending_delete_dataset_id = app.knowledge.selected_dataset_id.clone();
            Task::none()
        }
        KnowledgeToolMessage::OpenDeleteDatasetConfirmFor(dataset_id) => {
            app.knowledge.close_dataset_menu();
            app.knowledge.pending_delete_document_id = None;
            app.knowledge.pending_delete_dataset_id = Some(dataset_id);
            Task::none()
        }
        KnowledgeToolMessage::ToggleDatasetMenu(dataset_id) => {
            app.knowledge.toggle_dataset_menu(&dataset_id);
            Task::none()
        }
        KnowledgeToolMessage::CloseDatasetMenu => {
            app.knowledge.close_dataset_menu();
            Task::none()
        }
        KnowledgeToolMessage::CopyDatasetId(dataset_id) => {
            app.knowledge.close_dataset_menu();
            Task::batch([iced::clipboard::write(dataset_id), set_success(app, "知识库 ID 已复制")])
        }
        KnowledgeToolMessage::OpenDeleteDocumentConfirm(document_id) => {
            app.knowledge.pending_delete_dataset_id = None;
            app.knowledge.pending_delete_document_id = Some(document_id);
            Task::none()
        }
        KnowledgeToolMessage::CloseDeleteConfirm => {
            app.knowledge.pending_delete_dataset_id = None;
            app.knowledge.pending_delete_document_id = None;
            Task::none()
        }
        KnowledgeToolMessage::DeleteSelectedDataset => delete_selected_dataset(app),
        KnowledgeToolMessage::DatasetDeleted(result) => {
            app.knowledge.deleting = false;
            match result {
                Ok(dataset) => {
                    app.knowledge.datasets.retain(|item| item.id != dataset.id);
                    app.knowledge.show_dataset_list();
                    app.knowledge.select_first_dataset_if_needed();
                    app.knowledge.sync_retrieve_defaults_from_selected_dataset();
                    app.knowledge.documents.clear();
                    app.knowledge.loaded_chunks.clear();
                    app.knowledge.retrieve_results.clear();
                    app.knowledge.selected_document_id = None;
                    app.knowledge.selected_chunk_id = None;
                    app.knowledge.clear_document_selection();
                    app.knowledge.close_document_rename_modal();
                    let toast = set_success(app, "知识库已删除");
                    if let Some(dataset_id) = app.knowledge.selected_dataset_id.clone() {
                        app.knowledge.loading_documents = true;
                        Task::batch([toast, load_documents_task(dataset_id)])
                    } else {
                        toast
                    }
                }
                Err(error) => set_error(app, error),
            }
        }
        KnowledgeToolMessage::DocumentNameChanged(value) => {
            app.knowledge.document_name_input = value;
            Task::none()
        }
        KnowledgeToolMessage::DocumentContentAction(action) => {
            app.knowledge.document_content_editor.perform(action);
            Task::none()
        }
        KnowledgeToolMessage::CreateDocument => create_document(app),
        KnowledgeToolMessage::DocumentCreated(dataset_id, result) => {
            app.knowledge.creating_document = false;
            match result {
                Ok(document) => {
                    app.knowledge.selected_document_id = Some(document.id);
                    app.knowledge.selected_chunk_id = None;
                    app.knowledge.document_preview_open = false;
                    app.knowledge.active_tab = KnowledgeDetailTab::Documents;
                    app.knowledge.document_page = 1;
                    app.knowledge.clear_document_selection();
                    app.knowledge.close_document_rename_modal();
                    app.knowledge.document_name_input.clear();
                    app.knowledge.document_content_editor = text_editor::Content::new();
                    let toast = set_success(app, "文档已添加并入库");
                    app.knowledge.loading_documents = true;
                    Task::batch([toast, load_documents_task(dataset_id)])
                }
                Err(error) => set_error(app, error),
            }
        }
        KnowledgeToolMessage::PickImportFile => pick_import_file(app),
        KnowledgeToolMessage::ImportFileSelected(result) => {
            app.knowledge.selecting_import_file = false;
            app.knowledge.file_drop_hovered = false;
            match result {
                Ok(files) if !files.is_empty() => {
                    app.knowledge.file_imports = files;
                    Task::none()
                }
                Ok(_) => Task::none(),
                Err(error) => set_error(app, error),
            }
        }
        KnowledgeToolMessage::ClearImportFile => {
            app.knowledge.file_imports.clear();
            app.knowledge.file_drop_hovered = false;
            Task::none()
        }
        KnowledgeToolMessage::ImportSelectedFile => import_selected_files(app),
        KnowledgeToolMessage::ImportDropHoverChanged(value) => {
            app.knowledge.file_drop_hovered = value;
            Task::none()
        }
        KnowledgeToolMessage::ImportDroppedFiles => handle_import_dropped_files(app),
        KnowledgeToolMessage::FileImported(dataset_id, result) => {
            app.knowledge.importing_file = false;
            app.knowledge.file_drop_hovered = false;
            match result {
                Ok(responses) => {
                    let document_id = responses.last().map(|response| response.document.id.clone());
                    let message = import_batch_success_message(&responses);
                    for response in &responses {
                        track_import_job(app, response);
                    }
                    app.knowledge.file_imports.clear();
                    app.knowledge.selected_document_id = document_id;
                    app.knowledge.selected_chunk_id = None;
                    app.knowledge.document_preview_open = false;
                    app.knowledge.active_tab = KnowledgeDetailTab::Documents;
                    app.knowledge.document_page = 1;
                    app.knowledge.loaded_chunks.clear();
                    app.knowledge.loading_chunks = false;
                    app.knowledge.clear_document_selection();
                    app.knowledge.close_document_rename_modal();
                    if app.knowledge.selected_dataset_id.as_deref() == Some(dataset_id.as_str()) {
                        app.knowledge.loading_documents = true;
                        app.knowledge.loading_datasets = true;
                        Task::batch(vec![
                            set_success(app, message),
                            load_documents_task(dataset_id),
                            load_datasets_task(),
                            schedule_import_polling_task(
                                !app.knowledge.import_polling_jobs.is_empty(),
                            ),
                        ])
                    } else {
                        Task::batch([
                            set_success(app, message),
                            schedule_import_polling_task(
                                !app.knowledge.import_polling_jobs.is_empty(),
                            ),
                        ])
                    }
                }
                Err(error) => set_error(app, error),
            }
        }
        KnowledgeToolMessage::PollImportJobs => poll_import_jobs(app),
        KnowledgeToolMessage::ImportJobPolled(result) => handle_import_job_polled(app, result),
        KnowledgeToolMessage::DeleteDocument(document_id) => {
            app.knowledge.pending_delete_dataset_id = None;
            app.knowledge.pending_delete_document_id = None;
            if app.knowledge.deleting {
                return Task::none();
            }
            app.knowledge.deleting = true;
            Task::perform(delete_document_async(document_id), |result| {
                Message::Knowledge(KnowledgeToolMessage::DocumentDeleted(result))
            })
        }
        KnowledgeToolMessage::DocumentDeleted(result) => {
            app.knowledge.deleting = false;
            match result {
                Ok(document) => {
                    let deleted_selected =
                        app.knowledge.selected_document_id.as_deref() == Some(document.id.as_str());
                    app.knowledge.documents.retain(|item| item.id != document.id);
                    if deleted_selected {
                        app.knowledge.loaded_chunks.clear();
                        app.knowledge.selected_document_id = None;
                        app.knowledge.selected_chunk_id = None;
                        app.knowledge.document_preview_open = false;
                    }
                    app.knowledge.selected_document_ids.remove(&document.id);
                    app.knowledge.prune_document_selection();
                    set_success(app, "文档已删除")
                }
                Err(error) => set_error(app, error),
            }
        }
        KnowledgeToolMessage::ReindexDocument(document_id) => {
            if app.knowledge.reindexing_document_id.is_some() {
                return Task::none();
            }
            app.knowledge.selected_document_id = Some(document_id.clone());
            app.knowledge.reindexing_document_id = Some(document_id.clone());
            Task::perform(reindex_document_async(document_id), |result| {
                Message::Knowledge(KnowledgeToolMessage::ReindexFinished(result))
            })
        }
        KnowledgeToolMessage::ReindexFinished(result) => {
            app.knowledge.reindexing_document_id = None;
            match result {
                Ok(document) => {
                    if let Some(existing) =
                        app.knowledge.documents.iter_mut().find(|item| item.id == document.id)
                    {
                        *existing = document.clone();
                    } else {
                        app.knowledge.documents.insert(0, document.clone());
                    }
                    app.knowledge.selected_document_id = Some(document.id.clone());
                    app.knowledge.selected_chunk_id = None;
                    app.knowledge.loaded_chunks.clear();
                    let toast = set_success(app, "文档已重新入库");
                    if app.knowledge.selected_dataset_id.as_deref()
                        == Some(document.dataset_id.as_str())
                    {
                        app.knowledge.loading_documents = true;
                        app.knowledge.loading_chunks = true;
                        Task::batch([
                            toast,
                            load_documents_task(document.dataset_id),
                            load_chunks_task(document.id),
                        ])
                    } else {
                        toast
                    }
                }
                Err(error) => set_error(app, error),
            }
        }
        KnowledgeToolMessage::RetrieveQueryChanged(value) => {
            app.knowledge.retrieve_query_input = value;
            Task::none()
        }
        KnowledgeToolMessage::RetrieveTopKChanged(value) => {
            app.knowledge.retrieve_top_k_input = value;
            Task::none()
        }
        KnowledgeToolMessage::RetrieveScoreThresholdEnabledChanged(value) => {
            app.knowledge.retrieve_score_threshold_enabled = value;
            Task::none()
        }
        KnowledgeToolMessage::RetrieveScoreThresholdChanged(value) => {
            app.knowledge.retrieve_score_threshold_input = value;
            Task::none()
        }
        KnowledgeToolMessage::RetrieveIncludeContextChanged(value) => {
            app.knowledge.retrieve_include_context = value;
            Task::none()
        }
        KnowledgeToolMessage::Retrieve => retrieve(app),
        KnowledgeToolMessage::RetrieveFinished(result) => {
            app.knowledge.retrieving = false;
            match result {
                Ok(response) => {
                    app.knowledge.retrieve_results = response.chunks;
                    set_success(app, "召回测试完成")
                }
                Err(error) => set_error(app, error),
            }
        }
        KnowledgeToolMessage::RunEmbeddingTest => run_embedding_test(app),
        KnowledgeToolMessage::EmbeddingTestFinished(result) => {
            app.knowledge.testing_embedding = false;
            match result {
                Ok(mut response) => {
                    response.message = sanitize_sensitive_display_text(&response.message);
                    let ok = response.ok;
                    let message = if ok {
                        embedding_test_success_message(&response)
                    } else {
                        embedding_test_failure_message(&response)
                    };
                    app.knowledge.embedding_test_result = Some(response);
                    if ok { set_success(app, message) } else { set_error(app, message) }
                }
                Err(error) => set_error(app, sanitize_sensitive_display_text(&error)),
            }
        }
        KnowledgeToolMessage::ClearDocumentDraft => {
            app.knowledge.document_name_input.clear();
            app.knowledge.document_content_editor = text_editor::Content::new();
            Task::none()
        }
        KnowledgeToolMessage::ClearNotification => {
            app.knowledge.notification = None;
            app.knowledge.notification_is_error = false;
            Task::none()
        }
    }
}

pub(crate) fn ensure_loaded(app: &mut App) -> Task<Message> {
    let mut tasks = Vec::new();
    if app.knowledge.runtime_status.is_none() && !app.knowledge.loading_status {
        app.knowledge.loading_status = true;
        tasks.push(load_status_task());
    }
    if app.knowledge.embedding_status.is_none() && !app.knowledge.loading_embedding_status {
        app.knowledge.loading_embedding_status = true;
        tasks.push(load_embedding_status_task());
    }
    if app.knowledge.embedding_presets.is_empty() && !app.knowledge.loading_embedding_presets {
        app.knowledge.loading_embedding_presets = true;
        tasks.push(load_embedding_presets_task());
    }
    if app.knowledge.datasets.is_empty() && !app.knowledge.loading_datasets {
        app.knowledge.loading_datasets = true;
        tasks.push(load_datasets_task());
    }
    if tasks.is_empty() { Task::none() } else { Task::batch(tasks) }
}

fn create_dataset(app: &mut App) -> Task<Message> {
    if app.knowledge.creating_dataset {
        return Task::none();
    }
    let name = app.knowledge.dataset_name_input.trim().to_string();
    if name.is_empty() {
        return set_error(app, "知识库名称不能为空");
    }
    let keyword_count =
        parse_usize_input(&app.knowledge.dataset_keyword_count_input, 10).clamp(1, 250);
    let top_k = parse_usize_input(&app.knowledge.dataset_top_k_input, 10).clamp(1, 50);
    let Some(score_threshold) = parse_score_threshold(&app.knowledge.dataset_score_threshold_input)
    else {
        return set_error(app, "Score 阈值必须是 0 到 1 之间的数字");
    };
    app.knowledge.creating_dataset = true;
    let body = KnowledgeDatasetCreateRequest {
        name,
        description: app.knowledge.dataset_description_input.trim().to_string(),
        chunking_mode: app.knowledge.dataset_chunking_mode.clone(),
        indexing_mode: app.knowledge.dataset_indexing_mode.clone(),
        retrieval_mode: app.knowledge.dataset_retrieval_mode.clone(),
        keyword_count,
        top_k,
        score_threshold_enabled: app.knowledge.dataset_score_threshold_enabled,
        score_threshold,
        rerank_enabled: app.knowledge.dataset_rerank_enabled,
        embedding_model: selected_dataset_embedding_model(app),
        rerank_model: trim_to_option(&app.knowledge.dataset_rerank_model_input),
    };
    Task::perform(create_dataset_async(body), |result| {
        Message::Knowledge(KnowledgeToolMessage::DatasetCreated(result))
    })
}

fn selected_dataset_embedding_model(app: &App) -> Option<String> {
    if app.knowledge.dataset_embedding_selection
        != KnowledgeDatasetEmbeddingSelection::LockCurrent
    {
        return None;
    }
    app.knowledge
        .embedding_status
        .as_ref()
        .map(|status| status.model.trim())
        .filter(|model| !model.is_empty())
        .map(str::to_string)
}

fn delete_selected_dataset(app: &mut App) -> Task<Message> {
    if app.knowledge.deleting {
        return Task::none();
    }
    let Some(dataset_id) = app
        .knowledge
        .pending_delete_dataset_id
        .clone()
        .or_else(|| app.knowledge.selected_dataset_id.clone())
    else {
        return set_error(app, "请先选择知识库");
    };
    app.knowledge.pending_delete_dataset_id = None;
    app.knowledge.pending_delete_document_id = None;
    app.knowledge.deleting = true;
    Task::perform(delete_dataset_async(dataset_id), |result| {
        Message::Knowledge(KnowledgeToolMessage::DatasetDeleted(result))
    })
}

fn create_document(app: &mut App) -> Task<Message> {
    if app.knowledge.creating_document {
        return Task::none();
    }
    let Some(dataset_id) = app.knowledge.selected_dataset_id.clone() else {
        return set_error(app, "请先选择知识库");
    };
    let name = app.knowledge.document_name_input.trim().to_string();
    if name.is_empty() {
        return set_error(app, "文档名称不能为空");
    }
    let content = app.knowledge.document_content_editor.text();
    if content.trim().is_empty() {
        return set_error(app, "文档内容不能为空");
    }
    app.knowledge.creating_document = true;
    let body =
        KnowledgeDocumentCreateRequest { name, content, metadata: Value::Null, enabled: true };
    let task_dataset_id = dataset_id.clone();
    Task::perform(create_document_async(dataset_id, body), move |result| {
        Message::Knowledge(KnowledgeToolMessage::DocumentCreated(task_dataset_id.clone(), result))
    })
}

fn pick_import_file(app: &mut App) -> Task<Message> {
    if app.knowledge.selecting_import_file || app.knowledge.importing_file {
        return Task::none();
    }
    app.knowledge.selecting_import_file = true;
    Task::perform(pick_import_files_async(), |result| {
        Message::Knowledge(KnowledgeToolMessage::ImportFileSelected(result))
    })
}

fn handle_import_dropped_files(app: &mut App) -> Task<Message> {
    app.knowledge.file_drop_hovered = false;
    if app.knowledge.selecting_import_file || app.knowledge.importing_file {
        clear_drag_drop_buffers(app);
        return Task::none();
    }
    let paths = if !app.dragging_file_paths.is_empty() {
        app.dragging_file_paths.clone()
    } else {
        app.pending_drop_file_paths.clone()
    };
    clear_drag_drop_buffers(app);
    if paths.is_empty() {
        return Task::none();
    }
    app.knowledge.selecting_import_file = true;
    Task::perform(load_import_files_from_paths_async(paths), |result| {
        Message::Knowledge(KnowledgeToolMessage::ImportFileSelected(result))
    })
}

fn import_selected_files(app: &mut App) -> Task<Message> {
    if app.knowledge.importing_file {
        return Task::none();
    }
    let Some(dataset_id) = app.knowledge.selected_dataset_id.clone() else {
        return set_error(app, "请先选择知识库");
    };
    let files = app.knowledge.file_imports.clone();
    if files.is_empty() {
        return set_error(app, "请先选择入库文件");
    }

    app.knowledge.importing_file = true;
    let task_dataset_id = dataset_id.clone();
    Task::perform(import_documents_async(dataset_id, files), move |result| {
        Message::Knowledge(KnowledgeToolMessage::FileImported(task_dataset_id.clone(), result))
    })
}

fn track_import_job(app: &mut App, response: &KnowledgeDocumentImportQueuedResponse) {
    let tracked = KnowledgeImportPollingJob {
        job_id: response.job.id.clone(),
        dataset_id: response.job.dataset_id.clone(),
        document_id: response.job.document_id.clone(),
        filename: response.filename.clone(),
        status: response.job.status.clone(),
    };
    if let Some(existing) =
        app.knowledge.import_polling_jobs.iter_mut().find(|job| job.job_id == tracked.job_id)
    {
        *existing = tracked;
    } else {
        app.knowledge.import_polling_jobs.push(tracked);
    }
}

fn poll_import_jobs(app: &mut App) -> Task<Message> {
    if app.knowledge.import_polling_jobs.is_empty() {
        return Task::none();
    }
    let mut tasks = app
        .knowledge
        .import_polling_jobs
        .iter()
        .map(|job| poll_import_job_task(job.job_id.clone()))
        .collect::<Vec<_>>();
    tasks.push(schedule_import_polling_task(true));
    Task::batch(tasks)
}

fn handle_import_job_polled(
    app: &mut App,
    result: Result<KnowledgeIndexJobDto, String>,
) -> Task<Message> {
    let job = match result {
        Ok(job) => job,
        Err(error) => {
            return set_error(app, format!("入库轮询失败：{error}"));
        }
    };

    let Some(index) =
        app.knowledge.import_polling_jobs.iter().position(|item| item.job_id == job.id)
    else {
        return Task::none();
    };

    match job.status {
        KnowledgeIndexJobStatus::Queued | KnowledgeIndexJobStatus::Processing => {
            update_import_polling_job_status(app, index, &job);
            sync_document_status_from_index_job(app, &job);
            Task::none()
        }
        KnowledgeIndexJobStatus::Completed => {
            app.knowledge.import_polling_jobs.remove(index);
            sync_document_status_from_index_job(app, &job);
            refresh_import_polling_documents(app, &job)
        }
        KnowledgeIndexJobStatus::Failed => {
            let tracked = app.knowledge.import_polling_jobs.remove(index);
            sync_document_status_from_index_job(app, &job);
            let message = if job.error_message.trim().is_empty() {
                format!("{} 入库失败", tracked.filename)
            } else {
                format!("{} 入库失败：{}", tracked.filename, job.error_message)
            };
            let refresh = refresh_import_polling_documents(app, &job);
            Task::batch([set_error(app, message), refresh])
        }
    }
}

fn update_import_polling_job_status(app: &mut App, index: usize, job: &KnowledgeIndexJobDto) {
    if app.knowledge.import_polling_jobs[index].status != job.status {
        app.knowledge.import_polling_jobs[index].status = job.status.clone();
    }
}

fn sync_document_status_from_index_job(app: &mut App, job: &KnowledgeIndexJobDto) {
    let Some(document) =
        app.knowledge.documents.iter_mut().find(|document| document.id == job.document_id)
    else {
        return;
    };
    document.status = document_status_from_index_job(&job.status);
    if job.status == KnowledgeIndexJobStatus::Failed {
        document.error_message = job.error_message.clone();
    } else if job.status != KnowledgeIndexJobStatus::Queued {
        document.error_message.clear();
    }
}

fn document_status_from_index_job(status: &KnowledgeIndexJobStatus) -> KnowledgeDocumentStatus {
    match status {
        KnowledgeIndexJobStatus::Queued => KnowledgeDocumentStatus::Pending,
        KnowledgeIndexJobStatus::Processing => KnowledgeDocumentStatus::Processing,
        KnowledgeIndexJobStatus::Completed => KnowledgeDocumentStatus::Completed,
        KnowledgeIndexJobStatus::Failed => KnowledgeDocumentStatus::Failed,
    }
}

fn refresh_import_polling_documents(app: &mut App, job: &KnowledgeIndexJobDto) -> Task<Message> {
    let mut tasks = Vec::new();
    if app.knowledge.selected_dataset_id.as_deref() == Some(job.dataset_id.as_str()) {
        app.knowledge.loading_documents = true;
        app.knowledge.loading_datasets = true;
        tasks.push(load_documents_task(job.dataset_id.clone()));
        tasks.push(load_datasets_task());
    }
    Task::batch(tasks)
}

fn clear_drag_drop_buffers(app: &mut App) {
    app.dragging_file_paths.clear();
    app.dragging_file_position = None;
    app.pending_drop_file_paths.clear();
    app.pending_drop_file_position = None;
}

fn retrieve(app: &mut App) -> Task<Message> {
    if app.knowledge.retrieving {
        return Task::none();
    }
    let Some(dataset_id) = app.knowledge.selected_dataset_id.clone() else {
        return set_error(app, "请先选择知识库");
    };
    let query = app.knowledge.retrieve_query_input.trim().to_string();
    if query.is_empty() {
        return set_error(app, "请输入召回测试文本");
    }
    let top_k =
        app.knowledge.retrieve_top_k_input.trim().parse::<usize>().unwrap_or(10).clamp(1, 50);
    let score_threshold = app.knowledge.retrieve_score_threshold_enabled.then(|| {
        parse_score_threshold(&app.knowledge.retrieve_score_threshold_input).unwrap_or(0.15)
    });
    app.knowledge.retrieving = true;
    let body = KnowledgeRetrieveRequest {
        query,
        dataset_ids: vec![dataset_id],
        top_k,
        score_threshold,
        metadata_filter: None,
        include_context: app.knowledge.retrieve_include_context,
    };
    Task::perform(retrieve_async(body), |result| {
        Message::Knowledge(KnowledgeToolMessage::RetrieveFinished(result))
    })
}

fn run_embedding_test(app: &mut App) -> Task<Message> {
    if app.knowledge.testing_embedding {
        return Task::none();
    }
    app.knowledge.testing_embedding = true;
    app.knowledge.embedding_test_result = None;
    Task::perform(embedding_test_async(embedding_test_request()), |result| {
        Message::Knowledge(KnowledgeToolMessage::EmbeddingTestFinished(result))
    })
}

fn embedding_test_request() -> KnowledgeEmbeddingTestRequest {
    KnowledgeEmbeddingTestRequest {
        provider_id: None,
        model: None,
        dimensions: None,
        api_key_override: None,
        sample_text: None,
    }
}

fn load_status_task() -> Task<Message> {
    Task::perform(status_async(), |result| {
        Message::Knowledge(KnowledgeToolMessage::StatusLoaded(result))
    })
}

fn load_embedding_status_task() -> Task<Message> {
    Task::perform(embedding_status_async(), |result| {
        Message::Knowledge(KnowledgeToolMessage::EmbeddingStatusLoaded(result))
    })
}

fn load_embedding_presets_task() -> Task<Message> {
    Task::perform(embedding_presets_async(), |result| {
        Message::Knowledge(KnowledgeToolMessage::EmbeddingPresetsLoaded(result))
    })
}

fn load_datasets_task() -> Task<Message> {
    Task::perform(list_datasets_async(), |result| {
        Message::Knowledge(KnowledgeToolMessage::DatasetsLoaded(result))
    })
}

fn load_documents_task(dataset_id: String) -> Task<Message> {
    Task::perform(list_documents_async(dataset_id.clone()), move |result| {
        Message::Knowledge(KnowledgeToolMessage::DocumentsLoaded(dataset_id.clone(), result))
    })
}

fn load_chunks_task(document_id: String) -> Task<Message> {
    Task::perform(list_chunks_async(document_id.clone()), move |result| {
        Message::Knowledge(KnowledgeToolMessage::ChunksLoaded(document_id.clone(), result))
    })
}

fn poll_import_job_task(job_id: String) -> Task<Message> {
    Task::perform(get_index_job_async(job_id), |result| {
        Message::Knowledge(KnowledgeToolMessage::ImportJobPolled(result))
    })
}

async fn pick_import_files_async() -> Result<Vec<KnowledgeFileImportDraft>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        Err("当前平台暂不支持文件选择".to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(files) = rfd::AsyncFileDialog::new()
            .add_filter("知识库文档", &["txt", "md", "markdown", "mdx", "pdf"])
            .pick_files()
            .await
        else {
            return Ok(Vec::new());
        };
        let mut drafts = Vec::with_capacity(files.len());
        for file in files {
            let filename = file.file_name();
            let bytes = file.read().await;
            let byte_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
            let content_base64 = BASE64_STANDARD.encode(bytes);
            drafts.push(KnowledgeFileImportDraft {
                mime_type: mime_type_for_filename(&filename),
                filename,
                byte_len,
                content_base64,
            });
        }
        Ok(drafts)
    }
}

async fn load_import_files_from_paths_async(
    paths: Vec<String>,
) -> Result<Vec<KnowledgeFileImportDraft>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = paths;
        Err("当前平台暂不支持拖拽文件上传".to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut drafts = Vec::new();
        let mut skipped = Vec::new();
        for path in paths {
            let path = std::path::PathBuf::from(path);
            if !path.is_file() {
                skipped.push(path.to_string_lossy().to_string());
                continue;
            }
            let Some(filename) =
                path.file_name().and_then(|name| name.to_str()).map(ToOwned::to_owned)
            else {
                skipped.push(path.to_string_lossy().to_string());
                continue;
            };
            let Some(mime_type) = mime_type_for_filename(&filename) else {
                skipped.push(filename);
                continue;
            };
            let bytes = std::fs::read(&path)
                .map_err(|error| format!("读取拖拽文件失败：{} ({error})", path.display()))?;
            let byte_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
            let content_base64 = BASE64_STANDARD.encode(bytes);
            drafts.push(KnowledgeFileImportDraft {
                filename,
                mime_type: Some(mime_type),
                byte_len,
                content_base64,
            });
        }
        if drafts.is_empty() {
            if skipped.is_empty() {
                Err("没有可导入的文件".to_string())
            } else {
                Err("仅支持拖拽 TXT / MD / MDX / PDF 文件".to_string())
            }
        } else {
            Ok(drafts)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn mime_type_for_filename(filename: &str) -> Option<String> {
    let extension = filename.rsplit('.').next()?.to_ascii_lowercase();
    let mime = match extension.as_str() {
        "txt" => "text/plain",
        "md" | "markdown" | "mdx" => "text/markdown",
        "pdf" => "application/pdf",
        _ => return None,
    };
    Some(mime.to_string())
}

fn import_success_message(response: &KnowledgeDocumentImportQueuedResponse) -> String {
    if response.warnings.is_empty() {
        return format!("文件已入队：{}，后台入库中", response.filename);
    }
    format!("文件已入队：{}（{} 条警告），后台入库中", response.filename, response.warnings.len())
}

fn import_batch_success_message(responses: &[KnowledgeDocumentImportQueuedResponse]) -> String {
    if responses.len() == 1 {
        return import_success_message(&responses[0]);
    }
    let warning_count = responses.iter().map(|response| response.warnings.len()).sum::<usize>();
    if warning_count == 0 {
        return format!("已入队 {} 个文件，后台入库中", responses.len());
    }
    format!("已入队 {} 个文件（{} 条警告），后台入库中", responses.len(), warning_count)
}

fn schedule_import_polling_task(enabled: bool) -> Task<Message> {
    if enabled {
        crate::app::message::after(
            IMPORT_JOB_POLL_INTERVAL,
            Message::Knowledge(KnowledgeToolMessage::PollImportJobs),
        )
    } else {
        Task::none()
    }
}

fn embedding_test_success_message(response: &KnowledgeEmbeddingTestResponse) -> String {
    response
        .observed_dimensions
        .map(|dimensions| format!("嵌入连接测试通过：{dimensions} 维"))
        .unwrap_or_else(|| "嵌入连接测试通过".to_string())
}

fn embedding_test_failure_message(response: &KnowledgeEmbeddingTestResponse) -> String {
    format!("嵌入连接测试失败：{}", response.message)
}

fn sanitize_sensitive_display_text(message: &str) -> String {
    let truncated: String = message.chars().take(600).collect();
    truncated.split_whitespace().map(redact_sensitive_token).collect::<Vec<_>>().join(" ")
}

fn redact_sensitive_token(token: &str) -> String {
    let trimmed = token.trim_matches(|character: char| {
        matches!(character, ',' | '.' | ';' | ':' | '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}')
    });
    if trimmed.is_empty() || !looks_sensitive_token(trimmed) {
        return token.to_string();
    }
    token.replacen(trimmed, "[REDACTED]", 1)
}

fn looks_sensitive_token(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("sk-")
        || lower.starts_with("bearer.")
        || lower.starts_with("bearer_")
        || lower.starts_with("bearer-")
        || lower.contains("api_key=")
        || lower.contains("apikey=")
        || lower.contains("authorization=")
        || (lower.contains("secret") && value.len() > 8)
}

fn parse_usize_input(input: &str, fallback: usize) -> usize {
    input.trim().parse::<usize>().unwrap_or(fallback)
}

fn parse_score_threshold(input: &str) -> Option<f64> {
    let value = input.trim().parse::<f64>().ok()?;
    value.is_finite().then_some(value.clamp(0.0, 1.0))
}

fn trim_to_option(input: &str) -> Option<String> {
    let value = input.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn build_document_patch_request(
    name: Option<&str>,
    enabled: Option<bool>,
    archived: Option<bool>,
) -> Option<KnowledgeDocumentPatchRequest> {
    let name = name.and_then(trim_to_option);
    if name.is_none() && enabled.is_none() && archived.is_none() {
        None
    } else {
        Some(KnowledgeDocumentPatchRequest { name, enabled, archived })
    }
}

fn build_batch_document_action_request(
    selected_document_ids: &HashSet<String>,
    action: KnowledgeDocumentsBatchAction,
) -> Option<KnowledgeDocumentsBatchActionRequest> {
    if selected_document_ids.is_empty() {
        return None;
    }
    let mut document_ids = selected_document_ids.iter().cloned().collect::<Vec<_>>();
    document_ids.sort();
    Some(KnowledgeDocumentsBatchActionRequest { document_ids, action })
}

fn sanitize_document_page_size(page_size: usize) -> usize {
    match page_size {
        10 | 20 | 30 | 40 | 50 | 100 => page_size,
        _ => 10,
    }
}

fn submit_document_update(
    app: &mut App,
    document_id: String,
    name: Option<&str>,
    enabled: Option<bool>,
    archived: Option<bool>,
) -> Task<Message> {
    if app.knowledge.updating_document_id.is_some() {
        return Task::none();
    }
    let Some(body) = build_document_patch_request(name, enabled, archived) else {
        return set_error(app, "请提供有效的文档更新内容");
    };
    app.knowledge.updating_document_id = Some(document_id.clone());
    Task::perform(update_document_async(document_id, body), |result| {
        Message::Knowledge(KnowledgeToolMessage::DocumentUpdated(result))
    })
}

fn handle_document_updated(
    app: &mut App,
    result: Result<KnowledgeDocumentDto, String>,
) -> Task<Message> {
    app.knowledge.updating_document_id = None;
    match result {
        Ok(document) => {
            let document_id = document.id.clone();
            let renamed_document =
                app.knowledge.rename_document_id.as_deref() == Some(document_id.as_str());
            replace_document(app, document.clone());
            app.knowledge.prune_document_selection();
            if renamed_document {
                app.knowledge.close_document_rename_modal();
            }
            if document.status == KnowledgeDocumentStatus::Archived {
                close_preview_if_selected_document(app, document.id.as_str());
            }
            set_success(app, "文档已更新")
        }
        Err(error) => set_error(app, error),
    }
}

fn submit_batch_document_action(
    app: &mut App,
    action: KnowledgeDocumentsBatchAction,
) -> Task<Message> {
    if app.knowledge.batch_updating_documents {
        return Task::none();
    }
    let Some(body) =
        build_batch_document_action_request(&app.knowledge.selected_document_ids, action.clone())
    else {
        return set_error(app, "请先选择文档");
    };
    app.knowledge.batch_updating_documents = true;
    app.knowledge.pending_batch_document_action = Some(action);
    Task::perform(batch_update_documents_async(body), |result| {
        Message::Knowledge(KnowledgeToolMessage::DocumentsBatchUpdated(result))
    })
}

fn handle_documents_batch_updated(
    app: &mut App,
    result: Result<Vec<KnowledgeDocumentDto>, String>,
) -> Task<Message> {
    let action = app.knowledge.pending_batch_document_action.take();
    app.knowledge.batch_updating_documents = false;
    match result {
        Ok(documents) => {
            let updated_count = documents.len();
            for document in documents {
                let close_preview = document.status == KnowledgeDocumentStatus::Archived;
                let document_id = document.id.clone();
                replace_document(app, document);
                if close_preview {
                    close_preview_if_selected_document(app, document_id.as_str());
                }
            }
            app.knowledge.prune_document_selection();
            app.knowledge.clear_document_selection();
            app.knowledge.close_document_rename_modal();
            set_success(app, batch_document_action_success_message(action.as_ref(), updated_count))
        }
        Err(error) => set_error(app, error),
    }
}

fn replace_document(app: &mut App, document: KnowledgeDocumentDto) {
    if let Some(existing) = app.knowledge.documents.iter_mut().find(|item| item.id == document.id) {
        *existing = document;
    } else {
        app.knowledge.documents.insert(0, document);
    }
}

fn close_preview_if_selected_document(app: &mut App, document_id: &str) {
    if app.knowledge.selected_document_id.as_deref() != Some(document_id) {
        return;
    }
    app.knowledge.selected_document_id = None;
    app.knowledge.selected_chunk_id = None;
    app.knowledge.document_preview_open = false;
    app.knowledge.loaded_chunks.clear();
    app.knowledge.loading_chunks = false;
}

fn batch_document_action_success_message(
    action: Option<&KnowledgeDocumentsBatchAction>,
    updated_count: usize,
) -> String {
    let action_label = match action.unwrap_or(&KnowledgeDocumentsBatchAction::Enable) {
        KnowledgeDocumentsBatchAction::Enable => "启用",
        KnowledgeDocumentsBatchAction::Disable => "停用",
        KnowledgeDocumentsBatchAction::Archive => "归档",
    };
    format!("已批量{action_label} {updated_count} 篇文档")
}

async fn status_async() -> Result<KnowledgeRuntimeStatus, String> {
    crate::app::gateway_client()?.knowledge_status().await
}

async fn embedding_status_async() -> Result<KnowledgeEmbeddingStatusDto, String> {
    crate::app::gateway_client()?.knowledge_embedding_status().await
}

async fn embedding_presets_async() -> Result<Vec<KnowledgeEmbeddingPresetDto>, String> {
    crate::app::gateway_client()?.knowledge_embedding_presets().await
}

async fn list_datasets_async() -> Result<Vec<KnowledgeDatasetDto>, String> {
    crate::app::gateway_client()?.knowledge_datasets_list().await
}

async fn list_documents_async(dataset_id: String) -> Result<Vec<KnowledgeDocumentDto>, String> {
    crate::app::gateway_client()?.knowledge_documents_list(&dataset_id).await
}

async fn list_chunks_async(document_id: String) -> Result<Vec<KnowledgeChunkDto>, String> {
    crate::app::gateway_client()?.knowledge_chunks_list(&document_id).await
}

async fn create_dataset_async(
    body: KnowledgeDatasetCreateRequest,
) -> Result<KnowledgeDatasetDto, String> {
    crate::app::gateway_client()?.knowledge_dataset_create(&body).await
}

async fn delete_dataset_async(dataset_id: String) -> Result<KnowledgeDatasetDto, String> {
    crate::app::gateway_client()?.knowledge_dataset_delete(&dataset_id).await
}

async fn create_document_async(
    dataset_id: String,
    body: KnowledgeDocumentCreateRequest,
) -> Result<KnowledgeDocumentDto, String> {
    crate::app::gateway_client()?.knowledge_document_create(&dataset_id, &body).await
}

fn import_request_from_file(file: KnowledgeFileImportDraft) -> KnowledgeDocumentImportRequest {
    KnowledgeDocumentImportRequest {
        filename: file.filename,
        mime_type: file.mime_type,
        content_base64: file.content_base64,
        metadata: serde_json::json!({
            "source": "desktop_file_import",
            "source_bytes": file.byte_len,
        }),
        enabled: true,
    }
}

async fn import_documents_async(
    dataset_id: String,
    files: Vec<KnowledgeFileImportDraft>,
) -> Result<Vec<KnowledgeDocumentImportQueuedResponse>, String> {
    let client = crate::app::gateway_client()?;
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

async fn get_index_job_async(job_id: String) -> Result<KnowledgeIndexJobDto, String> {
    crate::app::gateway_client()?.knowledge_index_job_get(&job_id).await
}

async fn delete_document_async(document_id: String) -> Result<KnowledgeDocumentDto, String> {
    crate::app::gateway_client()?.knowledge_document_delete(&document_id).await
}

async fn update_document_async(
    document_id: String,
    body: KnowledgeDocumentPatchRequest,
) -> Result<KnowledgeDocumentDto, String> {
    crate::app::gateway_client()?.knowledge_document_update(&document_id, &body).await
}

async fn batch_update_documents_async(
    body: KnowledgeDocumentsBatchActionRequest,
) -> Result<Vec<KnowledgeDocumentDto>, String> {
    crate::app::gateway_client()?.knowledge_documents_batch_action(&body).await
}

async fn reindex_document_async(document_id: String) -> Result<KnowledgeDocumentDto, String> {
    crate::app::gateway_client()?
        .knowledge_document_reindex(&document_id, &KnowledgeDocumentReindexRequest { force: false })
        .await
}

async fn retrieve_async(
    body: KnowledgeRetrieveRequest,
) -> Result<KnowledgeRetrieveResponse, String> {
    crate::app::gateway_client()?.knowledge_retrieve(&body).await
}

async fn embedding_test_async(
    body: KnowledgeEmbeddingTestRequest,
) -> Result<KnowledgeEmbeddingTestResponse, String> {
    crate::app::gateway_client()?.knowledge_embedding_test(&body).await
}

fn set_success(app: &mut App, message: impl Into<String>) -> Task<Message> {
    let message = message.into();
    app.knowledge.notification = Some(message.clone());
    app.knowledge.notification_is_error = false;
    Task::batch([app.show_success_toast(message), clear_notification_task()])
}

fn set_error(app: &mut App, message: impl Into<String>) -> Task<Message> {
    let message = message.into();
    app.knowledge.notification = Some(message.clone());
    app.knowledge.notification_is_error = true;
    Task::batch([app.show_error_toast(message), clear_notification_task()])
}

fn clear_notification_task() -> Task<Message> {
    crate::app::message::after(
        KNOWLEDGE_NOTIFICATION_DURATION,
        Message::Knowledge(KnowledgeToolMessage::ClearNotification),
    )
}

#[cfg(test)]
#[path = "knowledge_tool_tests.rs"]
mod knowledge_tool_tests;
