//! # Workflow 状态模型
//!
//! 该模块定义 workflow 状态对象，以及编辑器草稿、历史快照和上下文菜单等运行时状态结构。

use super::model::{
    LoadedWorkflow, WorkflowAppMeta, WorkflowConnectionDraft, WorkflowConnectionEndpoint,
    WorkflowConversationVariable, WorkflowDocument, WorkflowEdge, WorkflowEnvironmentVariable,
    WorkflowHandleKind, WorkflowNode, create_node_from_type, default_node_data_yaml,
    node_data_yaml, pretty_block_type, rebuild_node_from_parts, yaml_map,
};
use iced::{Point, Vector, widget::text_editor};
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

mod app_ui;
mod canvas_ops;
mod core;
mod draft_build;
mod ids;
mod if_else_helpers;
mod node_editor_branches;
mod node_editor_integrations;
mod node_editor_start;
mod node_ops;
mod node_validation;
mod start_variable_drafts;
mod start_variable_validation;
mod variables;
mod visual_sync;
mod yaml_helpers;

#[cfg(test)]
#[path = "app_ui_tests.rs"]
mod app_ui_tests;
#[cfg(test)]
#[path = "canvas_ops_tests.rs"]
mod canvas_ops_tests;
#[cfg(test)]
#[path = "core_tests.rs"]
mod core_tests;
#[cfg(test)]
#[path = "draft_build_tests.rs"]
mod draft_build_tests;
#[cfg(test)]
#[path = "ids_tests.rs"]
mod ids_tests;
#[cfg(test)]
#[path = "if_else_helpers_tests.rs"]
mod if_else_helpers_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "node_editor_branches_tests.rs"]
mod node_editor_branches_tests;
#[cfg(test)]
#[path = "node_editor_integrations_tests.rs"]
mod node_editor_integrations_tests;

#[cfg(test)]
use app_ui::organized_node_positions;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use core::workflow_run_inputs_from_document;
use draft_build::*;
use ids::*;
use if_else_helpers::*;
use node_validation::*;
use start_variable_drafts::*;
pub(super) use start_variable_validation::*;
use visual_sync::*;
use yaml_helpers::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

const WORKFLOW_HISTORY_LIMIT: usize = 50;

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowHistorySnapshot {
    pub meta: WorkflowAppMeta,
    pub document: WorkflowDocument,
    pub environment_variables: Vec<WorkflowEnvironmentVariable>,
    pub conversation_variables: Vec<WorkflowConversationVariable>,
    pub pan: Vector,
    pub zoom: f32,
    pub selected_node_id: Option<String>,
    pub selected_edge_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowAppEntry {
    pub id: String,
    pub local_uuid: Option<String>,
    pub meta: WorkflowAppMeta,
    pub source_path: Option<String>,
    pub raw_root: Value,
    pub document: WorkflowDocument,
    pub environment_variables: Vec<WorkflowEnvironmentVariable>,
    pub conversation_variables: Vec<WorkflowConversationVariable>,
    pub pan: Vector,
    pub zoom: f32,
    pub selected_node_id: Option<String>,
    pub selected_edge_id: Option<String>,
    pub connection_draft: Option<WorkflowConnectionDraft>,
    pub is_dirty: bool,
    pub undo_stack: Vec<WorkflowHistorySnapshot>,
    pub redo_stack: Vec<WorkflowHistorySnapshot>,
    pub saved_snapshot: WorkflowHistorySnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSavedAppSummary {
    pub uuid: String,
    pub name: String,
    pub description: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkflowRunSubscriptionRequest {
    pub uuid: String,
    pub query: String,
    pub inputs: BTreeMap<String, serde_json::Value>,
    pub user: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WorkflowRunPromptDraft {
    pub uuid: String,
    pub query: String,
    pub loading: bool,
    pub inputs: Vec<WorkflowRunInputDraft>,
    pub error: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WorkflowRunInputDraft {
    pub name: String,
    pub label: String,
    pub input_type: String,
    pub required: bool,
    pub hidden: bool,
    pub options: Vec<String>,
    pub value: String,
    pub placeholder: String,
    pub hint: String,
    pub max_length_input: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowRunStatus {
    Running,
    Succeeded,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunPreview {
    pub uuid: String,
    pub run_id: Option<String>,
    pub status: WorkflowRunStatus,
    pub nodes: Vec<WorkflowRunNodePreview>,
    pub text_output: String,
    pub error: Option<WorkflowRunErrorPreview>,
    pub is_finished: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunNodePreview {
    pub id: String,
    pub title: String,
    pub status: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunErrorPreview {
    pub status: u32,
    pub code: String,
    pub message: String,
}

/// 运行预览中“查看详细”弹窗的状态：保存标题与可滚动、可选中复制的完整内容。
#[derive(Debug, Default)]
pub struct WorkflowRunDetailDialog {
    pub title: String,
    pub content: text_editor::Content,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowAppEditorMode {
    Create,
    Edit(String),
}

#[derive(Debug, Clone)]
pub struct WorkflowAppEditorDraft {
    pub mode: WorkflowAppEditorMode,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub use_icon_as_answer_icon: bool,
    pub max_active_requests_input: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowNodeEditorMode {
    Create,
    Edit(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowNodeEditorTab {
    Basic,
    Description,
    Visual,
    AdvancedDsl,
}

#[derive(Debug)]
pub struct WorkflowNodeEditorDraft {
    pub mode: WorkflowNodeEditorMode,
    pub active_tab: WorkflowNodeEditorTab,
    pub block_type: String,
    pub title: String,
    pub description: String,
    pub description_editor: text_editor::Content,
    pub position: Point,
    pub visual_draft: Option<WorkflowNodeVisualDraft>,
    pub validation: WorkflowNodeEditorValidation,
    pub show_raw_data_editor: bool,
    pub raw_data_editor: text_editor::Content,
    pub hovered_start_variable_index: Option<usize>,
    pub start_variable_focus_index: Option<usize>,
    pub start_variable_editor: Option<WorkflowStartVariableEditorDraft>,
}

#[derive(Debug)]
pub enum WorkflowNodeVisualDraft {
    Start {
        variables: Vec<WorkflowStartVariableDraft>,
    },
    Llm {
        provider: String,
        model_name: String,
        model_mode: String,
        enable_thinking: bool,
        context_enabled: bool,
        context_selector_input: String,
        system_prompt_editor: text_editor::Content,
        user_prompt_editor: text_editor::Content,
        vision_enabled: bool,
    },
    Answer {
        answer_editor: text_editor::Content,
    },
    IfElse {
        cases: Vec<WorkflowIfElseCaseDraft>,
    },
    KnowledgeRetrieval {
        query_selector_input: String,
        query_attachment_selector_input: String,
        dataset_ids_input: String,
        retrieval_mode: String,
        top_k_input: String,
        score_threshold_enabled: bool,
        score_threshold_input: String,
        reranking_enable: bool,
        single_model_provider: String,
        single_model_name: String,
        single_model_mode: String,
    },
    Tool {
        provider_id: String,
        provider_type: String,
        provider_name: String,
        tool_name: String,
        tool_label: String,
        tool_description: String,
        credential_id: String,
        plugin_unique_identifier: String,
        tool_parameters_editor: text_editor::Content,
        tool_configurations_editor: text_editor::Content,
    },
    Agent {
        strategy_provider_name: String,
        strategy_name: String,
        strategy_label: String,
        plugin_unique_identifier: String,
        output_schema_editor: text_editor::Content,
        parameters_editor: text_editor::Content,
        memory_enabled: bool,
        memory_window_size_input: String,
        memory_prompt_editor: text_editor::Content,
    },
    Code {
        language: String,
        inputs: Vec<WorkflowCodeVariableDraft>,
        code_editor: text_editor::Content,
        outputs: Vec<WorkflowCodeOutputDraft>,
        retry_config: WorkflowNodeRetryDraft,
        error_strategy: String,
        default_value_editor: text_editor::Content,
    },
}

#[derive(Debug, Clone)]
pub struct WorkflowCodeVariableDraft {
    pub variable: String,
    pub value_type: String,
    pub selector: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowCodeOutputDraft {
    pub key: String,
    pub value_type: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowVariableSelectorDraft {
    pub path: Vec<String>,
}

impl WorkflowVariableSelectorDraft {
    pub fn from_input(input: &str) -> Self {
        Self {
            path: input
                .split('.')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(str::to_string)
                .collect(),
        }
    }

    pub fn to_input(&self) -> String {
        self.path.join(".")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowNamedValueDraft {
    pub id: String,
    pub name: String,
    pub value_type: String,
    pub selector: WorkflowVariableSelectorDraft,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowKeyValueDraft {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowModelDraft {
    pub provider: String,
    pub name: String,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowNodeFieldEdit {
    VariableSelectorChanged(String),
    NamedValueAdded,
    NamedValueRemoved(usize),
    NamedValueNameChanged(usize, String),
    NamedValueTypeChanged(usize, String),
    NamedValueSelectorChanged(usize, String),
    KeyValueAdded,
    KeyValueRemoved(usize),
    KeyValueKeyChanged(usize, String),
    KeyValueValueChanged(usize, String),
    ModelProviderChanged(String),
    ModelNameChanged(String),
    ModelModeChanged(String),
    RetryEnabledChanged(bool),
    RetryMaxRetriesChanged(u8),
    RetryIntervalChanged(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowNodeRetryDraft {
    pub enabled: bool,
    pub max_retries: u8,
    pub retry_interval: u16,
}

#[derive(Debug, Clone)]
pub struct WorkflowStartVariableDraft {
    pub raw_variable: Value,
    pub label: String,
    pub variable: String,
    pub input_type: String,
    pub required: bool,
    pub hidden: bool,
    pub options: Vec<String>,
    pub allowed_file_types: Vec<String>,
    pub allowed_file_extensions: Vec<String>,
    pub allowed_file_extensions_input: String,
    pub allowed_file_upload_methods: Vec<String>,
    pub default_value: String,
    pub default_file_values: Vec<String>,
    pub placeholder: String,
    pub hint: String,
    pub max_length_input: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStartVariableEditorMode {
    Create,
    Edit(usize),
}

#[derive(Debug, Clone)]
pub struct WorkflowStartVariableEditorDraft {
    pub mode: WorkflowStartVariableEditorMode,
    pub variable: WorkflowStartVariableDraft,
    pub default_value_editor: text_editor::Content,
    pub default_file_url_input: String,
    pub show_default_file_url_input: bool,
}

#[derive(Debug)]
pub struct WorkflowIfElseCaseDraft {
    pub raw_case: Value,
    pub case_id: String,
    pub logical_operator: String,
    pub conditions: Vec<WorkflowIfElseConditionDraft>,
}

#[derive(Debug)]
pub struct WorkflowIfElseConditionDraft {
    pub raw_condition: Value,
    pub variable_selector_input: String,
    pub comparison_operator: String,
    pub compare_value: String,
    pub var_type: String,
}

#[derive(Debug, Default)]
pub struct WorkflowNodeEditorValidation {
    pub field_errors: Vec<WorkflowNodeValidationError>,
}

#[derive(Debug)]
pub struct WorkflowNodeValidationError {
    pub path: String,
    pub message: String,
}

impl WorkflowNodeEditorValidation {
    pub fn has_errors(&self) -> bool {
        !self.field_errors.is_empty()
    }

    pub fn first_error_for(&self, path: &str) -> Option<&str> {
        self.field_errors.iter().find(|item| item.path == path).map(|item| item.message.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowVariablePanelKind {
    System,
    Environment,
    Conversation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowVariableEditorMode {
    CreateEnvironment,
    EditEnvironment(String),
    CreateConversation,
    EditConversation(String),
}

#[derive(Debug)]
pub struct WorkflowVariableEditorDraft {
    pub mode: WorkflowVariableEditorMode,
    pub name: String,
    pub description: String,
    pub value_type: String,
    pub raw_value_editor: text_editor::Content,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowCanvasContextMenuTarget {
    Canvas,
    Node(String),
    NodeInsert(String),
    Edge(String),
}

#[derive(Debug, Clone)]
pub struct WorkflowCanvasContextMenu {
    pub target: WorkflowCanvasContextMenuTarget,
    pub anchor: Point,
    pub world: Point,
}

#[derive(Debug, Default)]
pub struct WorkflowState {
    pub apps: Vec<WorkflowAppEntry>,
    pub active_app_id: Option<String>,
    pub saved_apps: Vec<WorkflowSavedAppSummary>,
    pub saved_apps_loading: bool,
    pub saved_apps_loaded: bool,
    pub saved_apps_error: Option<String>,
    pub opening_saved_app_uuid: Option<String>,
    pub deleting_saved_app_uuid: Option<String>,
    pub confirm_delete_saved_app_uuid: Option<String>,
    pub saved_app_actions_menu_uuid: Option<String>,
    pub saved_app_search_query: String,
    pub copied_saved_app_uuid: Option<String>,
    pub saved_app_run_prompt: Option<WorkflowRunPromptDraft>,
    pub running_saved_app_uuid: Option<String>,
    pub running_saved_app_query: String,
    pub running_saved_app_inputs: BTreeMap<String, serde_json::Value>,
    pub saved_app_run_preview: Option<WorkflowRunPreview>,
    /// 各应用上一次运行填写的参数草稿，按 uuid 缓存，方便下次打开运行弹窗时回填。
    pub last_run_drafts: HashMap<String, WorkflowRunPromptDraft>,
    /// 运行预览中“查看详细”弹窗，展开节点或输出的完整内容。
    pub saved_app_run_detail: Option<WorkflowRunDetailDialog>,
    /// 运行预览错误详情是否刚被复制，用于在按钮上短暂显示 ✓。
    pub run_error_copied: bool,
    pub active_is_dirty: bool,
    pub app_editor: Option<WorkflowAppEditorDraft>,
    pub node_editor: Option<WorkflowNodeEditorDraft>,
    pub node_editor_dsl_context_menu_open: bool,
    pub node_editor_dsl_context_menu_pos: Option<(f32, f32)>,
    pub node_editor_dsl_undo_stack: Vec<String>,
    pub node_editor_dsl_redo_stack: Vec<String>,
    pub variable_panel: Option<WorkflowVariablePanelKind>,
    pub variable_editor: Option<WorkflowVariableEditorDraft>,
    pub context_menu: Option<WorkflowCanvasContextMenu>,
    pub quick_insert_panel_open: bool,
    pub action_menu_open: bool,
    pub zoom_menu_open: bool,
    pub source_name: String,
    pub local_uuid: Option<String>,
    pub source_path: Option<String>,
    pub document: WorkflowDocument,
    pub environment_variables: Vec<WorkflowEnvironmentVariable>,
    pub conversation_variables: Vec<WorkflowConversationVariable>,
    pub pan: Vector,
    pub zoom: f32,
    pub selected_node_id: Option<String>,
    pub selected_edge_id: Option<String>,
    pub connection_draft: Option<WorkflowConnectionDraft>,
    pub undo_stack: Vec<WorkflowHistorySnapshot>,
    pub redo_stack: Vec<WorkflowHistorySnapshot>,
    pub saved_snapshot: Option<WorkflowHistorySnapshot>,
    pub dragging_node_id: Option<String>,
    pub drag_pending_snapshot: Option<WorkflowHistorySnapshot>,
    pub status_message: Option<String>,
    pub error_message: Option<String>,
}
