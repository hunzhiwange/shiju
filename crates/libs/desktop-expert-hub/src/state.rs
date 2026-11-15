use iced::widget::text_editor;
use std::collections::HashSet;
use {vw_api_knowledge::KnowledgeChunkDto, vw_api_knowledge::KnowledgeChunkingMode, vw_api_knowledge::KnowledgeDatasetDto, vw_api_knowledge::KnowledgeDocumentDto, vw_api_knowledge::KnowledgeDocumentStatus, vw_api_knowledge::KnowledgeEmbeddingPresetDto, vw_api_knowledge::KnowledgeEmbeddingStatusDto, vw_api_knowledge::KnowledgeEmbeddingTestResponse, vw_api_knowledge::KnowledgeIndexJobStatus, vw_api_knowledge::KnowledgeIndexingMode, vw_api_knowledge::KnowledgeRetrievalMode, vw_api_knowledge::KnowledgeRuntimeStatus};

/// 知识库详情页签。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KnowledgeDetailTab {
    #[default]
    Documents,
    AddDocument,
    Retrieval,
    Settings,
}

/// 添加文档页的输入方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KnowledgeDocumentInputTab {
    #[default]
    Files,
    Text,
}

/// 创建知识库弹窗的配置页签。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KnowledgeCreateDatasetTab {
    #[default]
    Basic,
    Indexing,
    Retrieval,
}

/// 新知识库使用嵌入配置的方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KnowledgeDatasetEmbeddingSelection {
    #[default]
    FollowGlobal,
    LockCurrent,
}

/// 桌面端文件入库草稿，只保存传输所需的文件元数据和 base64 载荷。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeFileImportDraft {
    pub filename: String,
    pub mime_type: Option<String>,
    pub byte_len: u64,
    pub content_base64: String,
}

/// 桌面端正在轮询的异步文件入库作业。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeImportPollingJob {
    pub job_id: String,
    pub dataset_id: String,
    pub document_id: String,
    pub filename: String,
    pub status: KnowledgeIndexJobStatus,
}

impl KnowledgeDetailTab {
    pub const ALL: [Self; 4] =
        [Self::Documents, Self::AddDocument, Self::Retrieval, Self::Settings];

    pub fn title(self) -> &'static str {
        match self {
            Self::Documents => "文档",
            Self::AddDocument => "添加文档",
            Self::Retrieval => "召回测试",
            Self::Settings => "设置",
        }
    }
}

impl KnowledgeDocumentInputTab {
    pub const ALL: [Self; 2] = [Self::Files, Self::Text];

    pub fn title(self) -> &'static str {
        match self {
            Self::Files => "文件",
            Self::Text => "文本",
        }
    }
}

impl KnowledgeCreateDatasetTab {
    pub const ALL: [Self; 3] = [Self::Basic, Self::Indexing, Self::Retrieval];

    pub fn title(self) -> &'static str {
        match self {
            Self::Basic => "基础信息",
            Self::Indexing => "分段与索引",
            Self::Retrieval => "检索设置",
        }
    }
}

/// 桌面端知识库工作台状态。
#[derive(Debug)]
pub struct KnowledgeUiState {
    pub datasets: Vec<KnowledgeDatasetDto>,
    pub documents: Vec<KnowledgeDocumentDto>,
    pub retrieve_results: Vec<KnowledgeChunkDto>,
    pub loaded_chunks: Vec<KnowledgeChunkDto>,
    pub runtime_status: Option<KnowledgeRuntimeStatus>,
    pub embedding_status: Option<KnowledgeEmbeddingStatusDto>,
    pub embedding_presets: Vec<KnowledgeEmbeddingPresetDto>,
    pub embedding_test_result: Option<KnowledgeEmbeddingTestResponse>,
    pub selected_dataset_id: Option<String>,
    pub selected_document_id: Option<String>,
    pub selected_chunk_id: Option<String>,
    pub active_tab: KnowledgeDetailTab,
    pub document_input_tab: KnowledgeDocumentInputTab,
    pub create_dataset_tab: KnowledgeCreateDatasetTab,
    pub showing_dataset_list: bool,
    pub create_dataset_modal_open: bool,
    pub document_preview_open: bool,
    pub dataset_search_query: String,
    pub document_search_query: String,
    pub document_status_filter: HashSet<KnowledgeDocumentStatus>,
    pub document_status_menu_open: bool,
    pub dataset_menu_open_id: Option<String>,
    pub document_page: usize,
    pub document_page_size: usize,
    pub selected_document_ids: HashSet<String>,
    pub rename_document_id: Option<String>,
    pub rename_document_name_input: String,
    pub dataset_name_input: String,
    pub dataset_description_input: String,
    pub dataset_chunking_mode: KnowledgeChunkingMode,
    pub dataset_indexing_mode: KnowledgeIndexingMode,
    pub dataset_retrieval_mode: KnowledgeRetrievalMode,
    pub dataset_embedding_selection: KnowledgeDatasetEmbeddingSelection,
    pub dataset_keyword_count_input: String,
    pub dataset_top_k_input: String,
    pub dataset_score_threshold_enabled: bool,
    pub dataset_score_threshold_input: String,
    pub dataset_rerank_enabled: bool,
    pub dataset_rerank_model_input: String,
    pub document_name_input: String,
    pub document_content_editor: text_editor::Content,
    pub file_imports: Vec<KnowledgeFileImportDraft>,
    pub import_polling_jobs: Vec<KnowledgeImportPollingJob>,
    pub file_drop_hovered: bool,
    pub pending_delete_dataset_id: Option<String>,
    pub pending_delete_document_id: Option<String>,
    pub retrieve_query_input: String,
    pub retrieve_top_k_input: String,
    pub retrieve_score_threshold_enabled: bool,
    pub retrieve_score_threshold_input: String,
    pub retrieve_include_context: bool,
    pub loading_status: bool,
    pub loading_embedding_status: bool,
    pub loading_embedding_presets: bool,
    pub loading_datasets: bool,
    pub loading_documents: bool,
    pub loading_chunks: bool,
    pub creating_dataset: bool,
    pub creating_document: bool,
    pub selecting_import_file: bool,
    pub importing_file: bool,
    pub deleting: bool,
    pub retrieving: bool,
    pub testing_embedding: bool,
    pub reindexing_document_id: Option<String>,
    pub updating_document_id: Option<String>,
    pub batch_updating_documents: bool,
    pub pending_batch_document_action:
        Option<vw_api_knowledge::KnowledgeDocumentsBatchAction>,
    pub notification: Option<String>,
    pub notification_feedback: Option<desktop_i18n::workflow::Feedback>,
    pub notification_is_error: bool,
}

impl Default for KnowledgeUiState {
    fn default() -> Self {
        Self {
            datasets: Vec::new(),
            documents: Vec::new(),
            retrieve_results: Vec::new(),
            loaded_chunks: Vec::new(),
            runtime_status: None,
            embedding_status: None,
            embedding_presets: Vec::new(),
            embedding_test_result: None,
            selected_dataset_id: None,
            selected_document_id: None,
            selected_chunk_id: None,
            active_tab: KnowledgeDetailTab::Documents,
            document_input_tab: KnowledgeDocumentInputTab::Files,
            create_dataset_tab: KnowledgeCreateDatasetTab::Basic,
            showing_dataset_list: true,
            create_dataset_modal_open: false,
            document_preview_open: false,
            dataset_search_query: String::new(),
            document_search_query: String::new(),
            document_status_filter: HashSet::new(),
            document_status_menu_open: false,
            dataset_menu_open_id: None,
            document_page: 1,
            document_page_size: 10,
            selected_document_ids: HashSet::new(),
            rename_document_id: None,
            rename_document_name_input: String::new(),
            dataset_name_input: String::new(),
            dataset_description_input: String::new(),
            dataset_chunking_mode: KnowledgeChunkingMode::General,
            dataset_indexing_mode: KnowledgeIndexingMode::Economy,
            dataset_retrieval_mode: KnowledgeRetrievalMode::FullText,
            dataset_embedding_selection: KnowledgeDatasetEmbeddingSelection::FollowGlobal,
            dataset_keyword_count_input: "10".to_string(),
            dataset_top_k_input: "10".to_string(),
            dataset_score_threshold_enabled: false,
            dataset_score_threshold_input: "0.15".to_string(),
            dataset_rerank_enabled: false,
            dataset_rerank_model_input: String::new(),
            document_name_input: String::new(),
            document_content_editor: text_editor::Content::new(),
            file_imports: Vec::new(),
            import_polling_jobs: Vec::new(),
            file_drop_hovered: false,
            pending_delete_dataset_id: None,
            pending_delete_document_id: None,
            retrieve_query_input: String::new(),
            retrieve_top_k_input: "10".to_string(),
            retrieve_score_threshold_enabled: false,
            retrieve_score_threshold_input: "0.15".to_string(),
            retrieve_include_context: false,
            loading_status: false,
            loading_embedding_status: false,
            loading_embedding_presets: false,
            loading_datasets: false,
            loading_documents: false,
            loading_chunks: false,
            creating_dataset: false,
            creating_document: false,
            selecting_import_file: false,
            importing_file: false,
            deleting: false,
            retrieving: false,
            testing_embedding: false,
            reindexing_document_id: None,
            updating_document_id: None,
            batch_updating_documents: false,
            pending_batch_document_action: None,
            notification: None,
            notification_feedback: None,
            notification_is_error: false,
        }
    }
}

impl KnowledgeUiState {
    pub fn selected_dataset(&self) -> Option<&KnowledgeDatasetDto> {
        let selected_id = self.selected_dataset_id.as_deref()?;
        self.datasets.iter().find(|dataset| dataset.id == selected_id)
    }

    pub fn selected_document(&self) -> Option<&KnowledgeDocumentDto> {
        let selected_id = self.selected_document_id.as_deref()?;
        self.documents.iter().find(|document| document.id == selected_id)
    }

    pub fn selected_chunk(&self) -> Option<&KnowledgeChunkDto> {
        let selected_id = self.selected_chunk_id.as_deref()?;
        self.loaded_chunks.iter().find(|chunk| chunk.id == selected_id)
    }

    pub fn is_busy(&self) -> bool {
        self.loading_status
            || self.loading_embedding_status
            || self.loading_embedding_presets
            || self.loading_datasets
            || self.loading_documents
            || self.loading_chunks
            || self.creating_dataset
            || self.creating_document
            || self.selecting_import_file
            || self.importing_file
            || self.deleting
            || self.retrieving
            || self.testing_embedding
            || self.reindexing_document_id.is_some()
            || self.updating_document_id.is_some()
            || self.batch_updating_documents
    }

    pub fn select_first_dataset_if_needed(&mut self) {
        if self.selected_dataset_id.as_ref().is_some_and(|selected_id| {
            self.datasets.iter().any(|dataset| dataset.id == *selected_id)
        }) {
            return;
        }
        self.selected_dataset_id = self.datasets.first().map(|dataset| dataset.id.clone());
    }

    pub fn select_first_chunk_if_needed(&mut self) {
        if self.selected_chunk_id.as_ref().is_some_and(|selected_id| {
            self.loaded_chunks.iter().any(|chunk| chunk.id == *selected_id)
        }) {
            return;
        }
        self.selected_chunk_id = self.loaded_chunks.first().map(|chunk| chunk.id.clone());
    }

    pub fn show_dataset_list(&mut self) {
        self.showing_dataset_list = true;
    }

    pub fn show_dataset_detail(&mut self) {
        self.showing_dataset_list = false;
    }

    pub fn sync_retrieve_defaults_from_selected_dataset(&mut self) {
        let Some(dataset) = self.selected_dataset().cloned() else {
            return;
        };
        self.retrieve_top_k_input = dataset.top_k.clamp(1, 50).to_string();
        self.retrieve_score_threshold_enabled = dataset.score_threshold_enabled;
        self.retrieve_score_threshold_input =
            format_score_threshold(dataset.score_threshold.clamp(0.0, 1.0));
    }

    pub fn toggle_document_status_filter(&mut self, status: KnowledgeDocumentStatus) {
        if !self.document_status_filter.remove(&status) {
            self.document_status_filter.insert(status);
        }
        self.document_page = 1;
    }

    pub fn clear_document_status_filter(&mut self) {
        self.document_status_filter.clear();
        self.document_page = 1;
    }

    pub fn toggle_document_status_menu(&mut self) {
        self.document_status_menu_open = !self.document_status_menu_open;
    }

    pub fn close_document_status_menu(&mut self) {
        self.document_status_menu_open = false;
    }

    pub fn toggle_dataset_menu(&mut self, dataset_id: &str) {
        if self.dataset_menu_open_id.as_deref() == Some(dataset_id) {
            self.dataset_menu_open_id = None;
        } else {
            self.dataset_menu_open_id = Some(dataset_id.to_string());
        }
    }

    pub fn close_dataset_menu(&mut self) {
        self.dataset_menu_open_id = None;
    }

    pub fn dataset_by_id(&self, dataset_id: &str) -> Option<&KnowledgeDatasetDto> {
        self.datasets.iter().find(|dataset| dataset.id == dataset_id)
    }

    pub fn filtered_document_ids(&self) -> Vec<String> {
        self.filtered_documents().into_iter().map(|document| document.id.clone()).collect()
    }

    pub fn visible_document_ids(&self) -> Vec<String> {
        let filtered_documents = self.filtered_documents();
        if filtered_documents.is_empty() {
            return Vec::new();
        }
        let page_size = self.document_page_size.max(1);
        let total_pages = filtered_documents.len().div_ceil(page_size).max(1);
        let current_page = self.document_page.clamp(1, total_pages);
        let start_index = (current_page - 1) * page_size;
        filtered_documents
            .into_iter()
            .skip(start_index)
            .take(page_size)
            .map(|document| document.id.clone())
            .collect()
    }

    pub fn toggle_document_selection(&mut self, document_id: &str) {
        if !self.documents.iter().any(|document| document.id == document_id) {
            return;
        }
        if !self.selected_document_ids.insert(document_id.to_string()) {
            self.selected_document_ids.remove(document_id);
        }
    }

    pub fn toggle_select_all_visible_documents(&mut self) {
        let visible_ids = self.visible_document_ids();
        if visible_ids.is_empty() {
            return;
        }
        let all_visible_selected =
            visible_ids.iter().all(|document_id| self.selected_document_ids.contains(document_id));
        for document_id in visible_ids {
            if all_visible_selected {
                self.selected_document_ids.remove(&document_id);
            } else {
                self.selected_document_ids.insert(document_id);
            }
        }
    }

    pub fn invert_visible_document_selection(&mut self) {
        let visible_ids = self.visible_document_ids();
        if visible_ids.is_empty() {
            return;
        }
        for document_id in visible_ids {
            if !self.selected_document_ids.insert(document_id.clone()) {
                self.selected_document_ids.remove(&document_id);
            }
        }
    }

    pub fn clear_document_selection(&mut self) {
        self.selected_document_ids.clear();
    }

    pub fn open_document_rename_modal(&mut self, document_id: &str) {
        let Some(document) = self.documents.iter().find(|document| document.id == document_id)
        else {
            return;
        };
        self.rename_document_id = Some(document.id.clone());
        self.rename_document_name_input = document.name.clone();
    }

    pub fn close_document_rename_modal(&mut self) {
        self.rename_document_id = None;
        self.rename_document_name_input.clear();
    }

    pub fn prune_document_selection(&mut self) {
        self.selected_document_ids.retain(|document_id| {
            self.documents.iter().any(|document| &document.id == document_id)
        });
        if self.rename_document_id.as_ref().is_some_and(|document_id| {
            !self.documents.iter().any(|document| &document.id == document_id)
        }) {
            self.close_document_rename_modal();
        }
    }

    fn filtered_documents(&self) -> Vec<&KnowledgeDocumentDto> {
        let query = self.document_search_query.trim().to_lowercase();
        self.documents
            .iter()
            .filter(|document| {
                if !query.is_empty() && !document.name.to_lowercase().contains(&query) {
                    return false;
                }
                let status = effective_document_status(document);
                if self.document_status_filter.is_empty() {
                    // 未选择任何状态时，默认隐藏已归档文档。
                    status != KnowledgeDocumentStatus::Archived
                } else {
                    self.document_status_filter.contains(&status)
                }
            })
            .collect()
    }
}

fn format_score_threshold(value: f64) -> String {
    let mut formatted = format!("{value:.2}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.push('0');
    }
    formatted
}

fn effective_document_status(document: &KnowledgeDocumentDto) -> KnowledgeDocumentStatus {
    if !document.enabled && document.status != KnowledgeDocumentStatus::Archived {
        KnowledgeDocumentStatus::Disabled
    } else {
        document.status.clone()
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
