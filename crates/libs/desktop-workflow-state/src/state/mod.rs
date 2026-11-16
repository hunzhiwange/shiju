//! # Workflow 状态模型
//!
//! 该模块定义 workflow 状态对象，以及编辑器草稿、历史快照和上下文菜单等运行时状态结构。

use desktop_workflow_model::{
    LoadedWorkflow, WorkflowAppMeta, WorkflowConnectionDraft, WorkflowConnectionEndpoint,
    WorkflowConversationVariable, WorkflowDocument, WorkflowEdge, WorkflowEnvironmentVariable,
    WorkflowHandleKind, WorkflowNode, create_node_from_type, default_node_data_yaml,
    node_data_yaml, pretty_block_type, rebuild_node_from_parts, yaml_map,
};
use iced::{Point, Vector, widget::text_editor};
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

mod app_ui;
mod canvas_ops;
mod core;
mod document_types;
mod draft_build;
mod history;
mod ids;
mod if_else_helpers;
mod node_editor_branches;
mod node_editor_integrations;
mod node_editor_start;
mod node_ops;
mod node_validation;
mod run_preview;
mod run_prompt;
mod run_types;
mod saved_apps;
mod start_variable_drafts;
mod start_variable_validation;
mod variable_types;
mod variables;
pub mod video_edit;
mod visual_sync;
mod yaml_helpers;

#[cfg(test)]
#[path = "app_ui_tests.rs"]
mod app_ui_tests;
#[cfg(test)]
#[path = "canvas_ops_tests.rs"]
mod canvas_ops_tests;
#[cfg(test)]
#[path = "classification_nodes_tests.rs"]
mod classification_nodes_tests;
#[cfg(test)]
#[path = "container_nodes_tests.rs"]
mod container_nodes_tests;
#[cfg(test)]
#[path = "core_tests.rs"]
mod core_tests;
#[cfg(test)]
#[path = "data_nodes_tests.rs"]
mod data_nodes_tests;
#[cfg(test)]
#[path = "draft_build_tests.rs"]
mod draft_build_tests;
#[cfg(test)]
#[path = "existing_nodes_tests.rs"]
mod existing_nodes_tests;
#[cfg(test)]
#[path = "http_request_tests.rs"]
mod http_request_tests;
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
#[path = "output_and_text_nodes_tests.rs"]
mod output_and_text_nodes_tests;

#[cfg(test)]
use app_ui::organized_node_positions;
#[cfg(not(target_arch = "wasm32"))]
pub use run_prompt::workflow_run_inputs_from_document;
use draft_build::*;
use ids::*;
use if_else_helpers::*;
use node_validation::*;
use start_variable_drafts::*;
pub use start_variable_validation::*;
pub(crate) use visual_sync::apply_visual_draft_to_yaml;
use visual_sync::*;
use yaml_helpers::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use document_types::*;
pub use run_types::*;
pub use variable_types::*;

const WORKFLOW_HISTORY_LIMIT: usize = 50;

pub use crate::drafts::*;

pub use crate::app_ui::{WorkflowCanvasContextMenu, WorkflowCanvasContextMenuTarget};

#[derive(Debug, Default)]
pub struct WorkflowState {
    pub tenant_drafts: HashMap<vw_api_transport::TenantScope, Box<WorkflowState>>,
    pub video_edit: video_edit::VideoEditUiState,
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
    pub localized_status: Option<desktop_i18n::format::Message>,
    pub error_message: Option<String>,
    pub localized_error: Option<desktop_i18n::workflow::Feedback>,
}
