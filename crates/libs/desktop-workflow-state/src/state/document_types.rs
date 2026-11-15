//! 工作流文档编辑条目、历史快照与已保存应用摘要。

use desktop_workflow_model::{
    WorkflowAppMeta, WorkflowConnectionDraft, WorkflowConversationVariable, WorkflowDocument,
    WorkflowEnvironmentVariable,
};
use iced::Vector;
use serde_yaml::Value;

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
