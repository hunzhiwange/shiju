//! 工作流执行运行时、提供方与事件的单一类型所有者。

use model_types::{ChatMessage, Provider};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use vw_api_workflow::{WorkflowNodeRunDto, WorkflowRunResponse};

#[derive(Clone)]
pub struct WorkflowRuntime {
    pub video_edit_budget: workflow_video_edit::budget::VideoEditRunBudget,
    pub provider: Arc<dyn Provider>,
    pub knowledge_provider: Option<Arc<dyn WorkflowKnowledgeProvider>>,
    pub document_extractor: Option<Arc<dyn WorkflowDocumentExtractor>>,
    pub tool_provider: Option<Arc<dyn WorkflowToolProvider>>,
    pub agent_provider: Option<Arc<dyn WorkflowAgentProvider>>,
    pub pause_store: Option<Arc<dyn WorkflowPauseStore>>,
    pub model: String,
    pub temperature: f64,
}

#[async_trait::async_trait]
pub trait WorkflowKnowledgeProvider: Send + Sync {
    async fn retrieve(
        &self,
        request: WorkflowKnowledgeRequest,
    ) -> Result<Vec<WorkflowKnowledgeChunk>, String>;
}

#[derive(Debug, Clone)]
pub struct WorkflowKnowledgeRequest {
    pub query: String,
    pub dataset_ids: Vec<String>,
    pub top_k: usize,
    pub score_threshold: Option<f64>,
    pub metadata_filter: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct WorkflowKnowledgeChunk {
    pub content: String,
    pub title: String,
    pub metadata: Value,
    pub score: Option<f64>,
}

#[async_trait::async_trait]
pub trait WorkflowDocumentExtractor: Send + Sync {
    async fn extract(
        &self,
        request: WorkflowDocumentRequest,
    ) -> Result<Vec<WorkflowExtractedDocument>, String>;
}

#[derive(Debug, Clone)]
pub struct WorkflowDocumentRequest {
    pub files: Vec<WorkflowDocumentFile>,
}

#[derive(Debug, Clone)]
pub struct WorkflowDocumentFile {
    pub name: String,
    pub mime_type: String,
    pub path: Option<String>,
    pub url: Option<String>,
    pub size: Option<u64>,
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct WorkflowExtractedDocument {
    pub text: String,
    pub file: Value,
}

#[async_trait::async_trait]
pub trait WorkflowToolProvider: Send + Sync {
    async fn call(&self, request: WorkflowToolRequest) -> Result<WorkflowToolResult, String>;
}

#[derive(Debug, Clone)]
pub struct WorkflowToolRequest {
    pub provider: String,
    pub tool_name: String,
    pub action: Option<String>,
    pub credential_id: Option<String>,
    pub inputs: BTreeMap<String, Value>,
}

#[derive(Debug, Clone)]
pub struct WorkflowToolResult {
    pub result: Value,
    pub text: Option<String>,
    pub json: Option<Value>,
    pub files: Vec<Value>,
}

#[async_trait::async_trait]
pub trait WorkflowAgentProvider: Send + Sync {
    async fn run(
        &self,
        request: WorkflowAgentRequest,
        tool_provider: Arc<dyn WorkflowToolProvider>,
    ) -> Result<WorkflowAgentResult, String>;
}

#[derive(Debug, Clone)]
pub struct WorkflowAgentRequest {
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<WorkflowAgentTool>,
    pub max_iterations: u32,
    pub model: String,
    pub temperature: f64,
    pub strategy: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowAgentTool {
    pub provider: String,
    pub tool_name: String,
    pub action: Option<String>,
    pub credential_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowAgentResult {
    pub answer: String,
    pub tool_outputs: Vec<Value>,
    pub reasoning: Option<String>,
    pub iterations: u32,
    pub success: bool,
}

#[async_trait::async_trait]
pub trait WorkflowPauseStore: Send + Sync {
    async fn save(&self, state: WorkflowPauseState) -> Result<(), String>;
    async fn load(&self, run_id: &str) -> Result<Option<WorkflowPauseState>, String>;
    async fn delete(&self, run_id: &str) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct WorkflowPauseState {
    pub run_id: String,
    pub form_token: String,
    pub source: String,
    pub pending_node_id: String,
    pub pool_values: BTreeMap<String, Value>,
    pub active_nodes: BTreeSet<String>,
    pub executed_nodes: BTreeSet<String>,
    pub activated_edges: BTreeSet<usize>,
    pub selected_handles: BTreeMap<String, String>,
    pub node_results: Vec<WorkflowNodeRunDto>,
    pub last_answer: Option<String>,
    pub last_outputs: BTreeMap<String, Value>,
    pub max_steps: u32,
}


#[derive(Debug, Clone)]
pub struct WorkflowNodeStartedEvent {
    pub node_id: String,
    pub node_type: String,
    pub title: String,
    pub index: u32,
}

#[derive(Debug, Clone)]
pub struct WorkflowNodeFinishedEvent {
    pub node: WorkflowNodeRunDto,
    pub index: u32,
}

#[derive(Debug, Clone)]
pub struct WorkflowNodeDeltaEvent {
    pub node_id: String,
    pub node_type: String,
    pub title: String,
    pub index: u32,
    pub text: String,
    pub replace: bool,
}

#[derive(Debug, Clone)]
pub struct WorkflowChildContext {
    pub run_id: String,
    pub parent_node_id: String,
    pub chunk_index: usize,
    pub chunk_id: String,
    pub node_id: String,
    pub call_id: String,
}

#[derive(Debug, Clone)]
pub enum WorkflowRunEvent {
    Child { context: WorkflowChildContext, event: Box<WorkflowRunEvent> },
    WorkflowStarted { run_id: String },
    NodeStarted(WorkflowNodeStartedEvent),
    NodeDelta(WorkflowNodeDeltaEvent),
    NodeFinished(WorkflowNodeFinishedEvent),
    WorkflowFinished(WorkflowRunResponse),
}
