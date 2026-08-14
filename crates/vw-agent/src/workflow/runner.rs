//! Workflow 图调度与节点执行。

use super::code_runner::run_code_node;
use super::conditions::{conditions_match, select_case_handle};
use super::model::{
    WorkflowEdge, WorkflowGraph, WorkflowNode, array_field, edge_handle_field, parse_workflow_yaml,
    string_field,
};
use super::template::{render_jinja_value_template, render_template, value_to_text};
use super::variables::{VariablePool, selector_from_value};
use crate::providers::traits::{StreamOptions, TokenUsage};
use crate::providers::{ChatMessage, ChatRequest, Provider};
use futures_util::StreamExt;
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::{sync::mpsc, task::JoinSet};
use uuid::Uuid;
use vw_api_types::workflow::{
    WorkflowHumanActionDto, WorkflowNodeRunDto, WorkflowNodeRunStatus, WorkflowPauseDto,
    WorkflowResumeRequest, WorkflowRunRequest, WorkflowRunResponse, WorkflowRunStatus,
};

const HTTP_REQUEST_DEFAULT_TIMEOUT_SECS: u64 = 30;
const HTTP_REQUEST_MAX_TIMEOUT_SECS: u64 = 60;
const HTTP_REQUEST_MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const WORKFLOW_DEBUG_MAX_CHARS: usize = 4_000;
const WORKFLOW_NODE_DELTA_MAX_CHARS: usize = 12_000;
const LOOP_DEFAULT_MAX_COUNT: u32 = 100;
const LOOP_MAX_COUNT: u32 = 1_000;

#[derive(Clone)]
pub struct WorkflowRuntime {
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

struct NodeExecution {
    status: WorkflowNodeRunStatus,
    inputs: BTreeMap<String, Value>,
    outputs: BTreeMap<String, Value>,
    selected_handle: Option<String>,
    answer: Option<String>,
    error: Option<String>,
    elapsed_ms: u64,
}

struct ParameterDefinition {
    name: String,
    type_name: String,
    description: String,
    required: bool,
}

struct QuestionClass {
    id: String,
    name: String,
    description: String,
}

struct WorkflowExecutionState {
    source: String,
    graph: WorkflowGraph,
    pool: VariablePool,
    active_nodes: BTreeSet<String>,
    executed_nodes: BTreeSet<String>,
    activated_edges: BTreeSet<usize>,
    selected_handles: BTreeMap<String, String>,
    node_results: Vec<WorkflowNodeRunDto>,
    last_answer: Option<String>,
    last_outputs: BTreeMap<String, Value>,
}

struct CompletedWorkflowNode {
    node: WorkflowNode,
    node_index: u32,
    execution: NodeExecution,
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
pub enum WorkflowRunEvent {
    WorkflowStarted { run_id: String },
    NodeStarted(WorkflowNodeStartedEvent),
    NodeDelta(WorkflowNodeDeltaEvent),
    NodeFinished(WorkflowNodeFinishedEvent),
    WorkflowFinished(WorkflowRunResponse),
}

pub async fn run_workflow(
    runtime: WorkflowRuntime,
    request: WorkflowRunRequest,
) -> Result<WorkflowRunResponse, String> {
    run_workflow_with_events(runtime, request, |_| {}).await
}

pub async fn run_workflow_with_events<F>(
    runtime: WorkflowRuntime,
    request: WorkflowRunRequest,
    mut on_event: F,
) -> Result<WorkflowRunResponse, String>
where
    F: FnMut(WorkflowRunEvent) + Send,
{
    let run_id = Uuid::new_v4().to_string();
    on_event(WorkflowRunEvent::WorkflowStarted { run_id: run_id.clone() });
    let max_steps = request.max_steps.clamp(1, 10_000);
    let source = resolve_workflow_source(&request)?;
    let graph = parse_workflow_yaml(&source)?;
    let state = WorkflowExecutionState {
        source,
        pool: initialize_variable_pool(&graph, &request, &run_id),
        active_nodes: graph.start_node_ids.iter().cloned().collect::<BTreeSet<_>>(),
        graph,
        executed_nodes: BTreeSet::new(),
        activated_edges: BTreeSet::new(),
        selected_handles: BTreeMap::new(),
        node_results: Vec::new(),
        last_answer: None,
        last_outputs: BTreeMap::new(),
    };

    let response = continue_workflow(runtime, run_id, state, max_steps, &mut on_event).await?;
    on_event(WorkflowRunEvent::WorkflowFinished(response.clone()));
    Ok(response)
}

pub async fn resume_workflow(
    runtime: WorkflowRuntime,
    request: WorkflowResumeRequest,
) -> Result<WorkflowRunResponse, String> {
    let store =
        runtime.pause_store.as_ref().ok_or_else(|| "workflow pause store 未配置".to_string())?;
    let paused = store
        .load(&request.run_id)
        .await?
        .ok_or_else(|| "workflow 暂停上下文不存在".to_string())?;
    if paused.form_token != request.form_token {
        return Err("workflow resume form_token 不匹配".to_string());
    }
    let graph = parse_workflow_yaml(&paused.source)?;
    let node = graph
        .nodes
        .get(&paused.pending_node_id)
        .cloned()
        .ok_or_else(|| format!("workflow 暂停节点不存在: {}", paused.pending_node_id))?;
    let action = resolve_human_input_action(&node, request.action.as_deref())?;
    validate_human_input_values(&node, &request.form_values)?;

    let mut pool = VariablePool::from_values(paused.pool_values);
    for (key, value) in &request.form_values {
        pool.insert_node_output(&node.id, key, value.clone());
    }
    pool.insert_node_output(&node.id, "action", Value::String(action.clone()));

    let mut active_nodes = paused.active_nodes;
    let mut executed_nodes = paused.executed_nodes;
    let mut activated_edges = paused.activated_edges;
    let mut selected_handles = paused.selected_handles;
    let mut node_results = paused.node_results;
    let outputs = human_input_resume_outputs(&request.form_values, &action);
    node_results.push(WorkflowNodeRunDto {
        node_id: node.id.clone(),
        node_type: node.node_type.clone(),
        title: node.title.clone(),
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::new(),
        outputs: redact_map(&outputs),
        selected_handle: Some(action.clone()),
        error: None,
        elapsed_ms: 0,
    });
    executed_nodes.insert(node.id.clone());
    selected_handles.insert(node.id.clone(), action);
    activate_outgoing_edges(
        &graph,
        &node,
        &selected_handles,
        &mut activated_edges,
        &mut active_nodes,
    );
    store.delete(&request.run_id).await?;

    let state = WorkflowExecutionState {
        source: paused.source,
        graph,
        pool,
        active_nodes,
        executed_nodes,
        activated_edges,
        selected_handles,
        node_results,
        last_answer: paused.last_answer,
        last_outputs: outputs,
    };
    let mut ignore_event = |_| {};
    continue_workflow(runtime, request.run_id, state, paused.max_steps, &mut ignore_event).await
}

async fn continue_workflow<F>(
    runtime: WorkflowRuntime,
    run_id: String,
    mut state: WorkflowExecutionState,
    max_steps: u32,
    on_event: &mut F,
) -> Result<WorkflowRunResponse, String>
where
    F: FnMut(WorkflowRunEvent) + Send,
{
    let mut executed_steps = 0_u32;
    while executed_steps < max_steps {
        let ready_node_ids = next_ready_nodes(
            &state.graph,
            &state.active_nodes,
            &state.executed_nodes,
            &state.activated_edges,
            &state.selected_handles,
        );
        if ready_node_ids.is_empty() {
            break;
        }

        let remaining_steps =
            usize::try_from(max_steps.saturating_sub(executed_steps)).unwrap_or(usize::MAX);
        let batch_ids = workflow_execution_batch(&state.graph, ready_node_ids, remaining_steps);
        if batch_ids.is_empty() {
            break;
        }

        let mut completed_nodes = if batch_ids.len() > 1 {
            let nodes = batch_ids
                .iter()
                .enumerate()
                .map(|(offset, node_id)| {
                    let node = state
                        .graph
                        .nodes
                        .get(node_id)
                        .cloned()
                        .ok_or_else(|| format!("workflow 节点不存在: {node_id}"))?;
                    let node_index =
                        u32::try_from(state.node_results.len() + offset + 1).unwrap_or(u32::MAX);
                    Ok((node, node_index))
                })
                .collect::<Result<Vec<_>, String>>()?;
            execute_parallel_node_batch(
                &runtime,
                &state.graph,
                &state.pool,
                &run_id,
                nodes,
                on_event,
            )
            .await?
        } else {
            let node_id = &batch_ids[0];
            let Some(node) = state.graph.nodes.get(node_id).cloned() else {
                return Err(format!("workflow 节点不存在: {node_id}"));
            };
            let node_index = u32::try_from(state.node_results.len() + 1).unwrap_or(u32::MAX);
            emit_workflow_node_started(&run_id, &node, node_index, on_event);
            let execution =
                execute_node(&runtime, &state.graph, &node, &mut state.pool, node_index, on_event)
                    .await;
            vec![CompletedWorkflowNode { node, node_index, execution }]
        };
        executed_steps =
            executed_steps.saturating_add(u32::try_from(completed_nodes.len()).unwrap_or(u32::MAX));

        completed_nodes.sort_by(|left, right| left.node_index.cmp(&right.node_index));
        for completed in completed_nodes {
            let applied = apply_completed_workflow_node(&run_id, &mut state, completed, on_event);
            if applied.failed {
                return Ok(WorkflowRunResponse {
                    run_id,
                    status: WorkflowRunStatus::Failed,
                    answer: state.last_answer,
                    outputs: redact_map(&state.last_outputs),
                    nodes: state.node_results,
                    error: applied.error,
                    pause: None,
                });
            }

            if applied.paused {
                return pause_workflow(&runtime, run_id, state, &applied.node, max_steps).await;
            }
        }
    }

    let pending = state.active_nodes.difference(&state.executed_nodes).next().cloned();
    if let Some(node_id) = pending {
        return Ok(WorkflowRunResponse {
            run_id,
            status: WorkflowRunStatus::Failed,
            answer: state.last_answer,
            outputs: redact_map(&state.last_outputs),
            nodes: state.node_results,
            error: Some(format!("workflow 执行被阻塞，节点未就绪: {node_id}")),
            pause: None,
        });
    }

    Ok(WorkflowRunResponse {
        run_id,
        status: WorkflowRunStatus::Succeeded,
        answer: state.last_answer,
        outputs: redact_map(&state.last_outputs),
        nodes: state.node_results,
        error: None,
        pause: None,
    })
}

struct AppliedWorkflowNode {
    node: WorkflowNode,
    failed: bool,
    paused: bool,
    error: Option<String>,
}

fn apply_completed_workflow_node<F>(
    run_id: &str,
    state: &mut WorkflowExecutionState,
    completed: CompletedWorkflowNode,
    on_event: &mut F,
) -> AppliedWorkflowNode
where
    F: FnMut(WorkflowRunEvent) + Send,
{
    let CompletedWorkflowNode { node, node_index, execution } = completed;

    let debug_inputs =
        debug_json_value(&Value::Object(redact_map(&execution.inputs).into_iter().collect()));
    let debug_outputs =
        debug_json_value(&Value::Object(redact_map(&execution.outputs).into_iter().collect()));
    tracing::debug!(
        target: "vw_agent::workflow",
        run_id = %run_id,
        node_id = %node.id,
        node_type = %node.node_type,
        title = %node.title,
        status = ?execution.status,
        elapsed_ms = execution.elapsed_ms,
        selected_handle = ?execution.selected_handle,
        error = ?execution.error,
        inputs = %debug_inputs,
        outputs = %debug_outputs,
        "workflow node finished"
    );
    for (key, value) in &execution.outputs {
        state.pool.insert_node_output(&node.id, key, value.clone());
    }
    if let Some(handle) = execution.selected_handle.as_ref() {
        state.selected_handles.insert(node.id.clone(), handle.clone());
    }
    if let Some(answer) = execution.answer.clone() {
        state.last_answer = Some(answer);
    }
    state.last_outputs = execution.outputs.clone();
    if let Some(delta) = workflow_node_delta_from_execution(&node, node_index, &execution) {
        on_event(WorkflowRunEvent::NodeDelta(delta));
    }
    let failed = execution.status == WorkflowNodeRunStatus::Failed;
    let paused = execution.status == WorkflowNodeRunStatus::Paused;
    let error = execution.error.clone();

    let node_result = WorkflowNodeRunDto {
        node_id: node.id.clone(),
        node_type: node.node_type.clone(),
        title: node.title.clone(),
        status: execution.status,
        inputs: redact_map(&execution.inputs),
        outputs: redact_map(&execution.outputs),
        selected_handle: execution.selected_handle,
        error: execution.error,
        elapsed_ms: execution.elapsed_ms,
    };
    on_event(WorkflowRunEvent::NodeFinished(WorkflowNodeFinishedEvent {
        node: node_result.clone(),
        index: node_index,
    }));
    state.node_results.push(node_result);

    if !failed && !paused {
        state.executed_nodes.insert(node.id.clone());

        activate_outgoing_edges(
            &state.graph,
            &node,
            &state.selected_handles,
            &mut state.activated_edges,
            &mut state.active_nodes,
        );
    }

    AppliedWorkflowNode { node, failed, paused, error }
}

async fn execute_parallel_node_batch<F>(
    runtime: &WorkflowRuntime,
    graph: &WorkflowGraph,
    pool: &VariablePool,
    run_id: &str,
    nodes: Vec<(WorkflowNode, u32)>,
    on_event: &mut F,
) -> Result<Vec<CompletedWorkflowNode>, String>
where
    F: FnMut(WorkflowRunEvent) + Send,
{
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let mut tasks = JoinSet::new();

    for (node, node_index) in nodes {
        emit_workflow_node_started(run_id, &node, node_index, on_event);

        let runtime = runtime.clone();
        let graph = graph.clone();
        let mut pool = pool.clone();
        let event_tx = event_tx.clone();
        tasks.spawn(async move {
            let mut send_event = move |event| {
                let _ = event_tx.send(event);
            };
            let execution =
                execute_node(&runtime, &graph, &node, &mut pool, node_index, &mut send_event).await;
            CompletedWorkflowNode { node, node_index, execution }
        });
    }
    drop(event_tx);

    let mut completed_nodes = Vec::new();
    while !tasks.is_empty() {
        tokio::select! {
            event = event_rx.recv() => {
                if let Some(event) = event {
                    on_event(event);
                }
            }
            completed = tasks.join_next() => {
                match completed {
                    Some(Ok(completed)) => completed_nodes.push(completed),
                    Some(Err(error)) => {
                        return Err(format!("workflow 并行节点任务失败: {error}"));
                    }
                    None => break,
                }
            }
        }
    }
    while let Some(event) = event_rx.recv().await {
        on_event(event);
    }

    Ok(completed_nodes)
}

fn emit_workflow_node_started<F>(
    run_id: &str,
    node: &WorkflowNode,
    node_index: u32,
    on_event: &mut F,
) where
    F: FnMut(WorkflowRunEvent) + Send,
{
    on_event(WorkflowRunEvent::NodeStarted(WorkflowNodeStartedEvent {
        node_id: node.id.clone(),
        node_type: node.node_type.clone(),
        title: node.title.clone(),
        index: node_index,
    }));
    tracing::debug!(
        target: "vw_agent::workflow",
        run_id = %run_id,
        node_id = %node.id,
        node_type = %node.node_type,
        title = %node.title,
        "workflow node started"
    );
}

fn resolve_workflow_source(request: &WorkflowRunRequest) -> Result<String, String> {
    if let Some(source) = request.workflow_yaml.as_deref().filter(|value| !value.trim().is_empty())
    {
        return Ok(source.to_string());
    }
    Err("application_workflow 必须提供".to_string())
}

fn initialize_variable_pool(
    graph: &WorkflowGraph,
    request: &WorkflowRunRequest,
    run_id: &str,
) -> VariablePool {
    let mut pool = VariablePool::default();
    let query = request
        .query
        .clone()
        .or_else(|| request.inputs.get("query").and_then(Value::as_str).map(ToOwned::to_owned))
        .unwrap_or_default();
    pool.insert_selector(&["sys".to_string(), "query".to_string()], Value::String(query.clone()));
    pool.insert_selector(
        &["sys".to_string(), "workflow_run_id".to_string()],
        Value::String(run_id.to_string()),
    );

    for start_id in &graph.start_node_ids {
        if let Some(node) = graph.nodes.get(start_id) {
            for variable in array_field(&node.data, "variables") {
                if let Some(name) = string_field(variable, "variable") {
                    let value = request
                        .inputs
                        .get(name)
                        .cloned()
                        .or_else(|| (name == "query").then(|| Value::String(query.clone())))
                        .unwrap_or(Value::Null);
                    pool.insert_node_output(start_id, name, value);
                }
            }
        }
    }
    pool
}

async fn pause_workflow(
    runtime: &WorkflowRuntime,
    run_id: String,
    state: WorkflowExecutionState,
    node: &WorkflowNode,
    max_steps: u32,
) -> Result<WorkflowRunResponse, String> {
    let pause = human_input_pause(node, Uuid::new_v4().to_string());
    let Some(store) = runtime.pause_store.as_ref() else {
        return Ok(WorkflowRunResponse {
            run_id,
            status: WorkflowRunStatus::Failed,
            answer: state.last_answer,
            outputs: redact_map(&state.last_outputs),
            nodes: state.node_results,
            error: Some("workflow pause store 未配置".to_string()),
            pause: None,
        });
    };

    let response_nodes = state.node_results.clone();
    let mut stored_node_results = state.node_results;
    stored_node_results.pop();
    store
        .save(WorkflowPauseState {
            run_id: run_id.clone(),
            form_token: pause.form_token.clone(),
            source: state.source,
            pending_node_id: node.id.clone(),
            pool_values: state.pool.values(),
            active_nodes: state.active_nodes,
            executed_nodes: state.executed_nodes,
            activated_edges: state.activated_edges,
            selected_handles: state.selected_handles,
            node_results: stored_node_results,
            last_answer: state.last_answer.clone(),
            last_outputs: state.last_outputs.clone(),
            max_steps,
        })
        .await?;

    Ok(WorkflowRunResponse {
        run_id,
        status: WorkflowRunStatus::Paused,
        answer: state.last_answer,
        outputs: redact_map(&state.last_outputs),
        nodes: response_nodes,
        error: None,
        pause: Some(pause),
    })
}

fn human_input_pause(node: &WorkflowNode, form_token: String) -> WorkflowPauseDto {
    WorkflowPauseDto {
        node_id: node.id.clone(),
        title: node.title.clone(),
        form_token,
        form: node.data.get("form").cloned().unwrap_or(Value::Null),
        actions: human_input_actions(node),
    }
}

fn human_input_actions(node: &WorkflowNode) -> Vec<WorkflowHumanActionDto> {
    array_field(&node.data, "actions")
        .iter()
        .filter_map(|action| {
            let id = string_field(action, "id")
                .or_else(|| string_field(action, "action"))
                .or_else(|| string_field(action, "value"))?
                .trim()
                .to_string();
            if id.is_empty() {
                return None;
            }
            let label = string_field(action, "label").unwrap_or(id.as_str()).trim().to_string();
            Some(WorkflowHumanActionDto { id, label })
        })
        .collect()
}

fn resolve_human_input_action(node: &WorkflowNode, action: Option<&str>) -> Result<String, String> {
    let actions = human_input_actions(node);
    if actions.is_empty() {
        return Ok(action.unwrap_or("source").trim().to_string());
    }
    let action = action
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "human-input resume 缺少 action".to_string())?;
    if actions.iter().any(|item| item.id == action) {
        Ok(action.to_string())
    } else {
        Err(format!("human-input resume action 不存在: {action}"))
    }
}

fn validate_human_input_values(
    node: &WorkflowNode,
    values: &BTreeMap<String, Value>,
) -> Result<(), String> {
    let fields = node
        .data
        .get("form")
        .and_then(|form| form.get("fields"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for field in fields {
        let Some(name) = string_field(field, "name").or_else(|| string_field(field, "variable"))
        else {
            continue;
        };
        if field.get("required").and_then(Value::as_bool).unwrap_or(false)
            && values.get(name).is_none_or(Value::is_null)
        {
            return Err(format!("human-input 字段 {name} 必填"));
        }
    }
    Ok(())
}

fn human_input_resume_outputs(
    form_values: &BTreeMap<String, Value>,
    action: &str,
) -> BTreeMap<String, Value> {
    let mut outputs = form_values.clone();
    outputs.insert("action".to_string(), Value::String(action.to_string()));
    outputs
}

fn next_ready_node(
    graph: &WorkflowGraph,
    active_nodes: &BTreeSet<String>,
    executed_nodes: &BTreeSet<String>,
    activated_edges: &BTreeSet<usize>,
    selected_handles: &BTreeMap<String, String>,
) -> Option<String> {
    next_ready_nodes(graph, active_nodes, executed_nodes, activated_edges, selected_handles)
        .into_iter()
        .next()
}

fn next_ready_nodes(
    graph: &WorkflowGraph,
    active_nodes: &BTreeSet<String>,
    executed_nodes: &BTreeSet<String>,
    activated_edges: &BTreeSet<usize>,
    selected_handles: &BTreeMap<String, String>,
) -> Vec<String> {
    active_nodes
        .iter()
        .filter(|node_id| !executed_nodes.contains(*node_id))
        .filter(|node_id| {
            node_ready(
                graph,
                node_id,
                active_nodes,
                executed_nodes,
                activated_edges,
                selected_handles,
            )
        })
        .cloned()
        .collect()
}

fn workflow_execution_batch(
    graph: &WorkflowGraph,
    ready_node_ids: Vec<String>,
    remaining_steps: usize,
) -> Vec<String> {
    let ready_node_ids = ready_node_ids.into_iter().take(remaining_steps).collect::<Vec<_>>();
    if ready_node_ids.len() < 2 {
        return ready_node_ids;
    }

    if ready_node_ids.iter().all(|node_id| {
        graph.nodes.get(node_id).is_some_and(workflow_node_allows_parallel_execution)
    }) {
        ready_node_ids
    } else {
        ready_node_ids.into_iter().take(1).collect()
    }
}

fn workflow_node_allows_parallel_execution(node: &WorkflowNode) -> bool {
    matches!(node.node_type.as_str(), "llm")
}

fn node_ready(
    graph: &WorkflowGraph,
    node_id: &str,
    active_nodes: &BTreeSet<String>,
    executed_nodes: &BTreeSet<String>,
    activated_edges: &BTreeSet<usize>,
    selected_handles: &BTreeMap<String, String>,
) -> bool {
    let incoming = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| edge.target == node_id)
        .collect::<Vec<_>>();
    if incoming.is_empty() {
        return true;
    }

    incoming
        .into_iter()
        .filter(|(_, edge)| {
            edge_required(graph, edge, active_nodes, executed_nodes, selected_handles)
        })
        .all(|(index, edge)| {
            executed_nodes.contains(&edge.source) && activated_edges.contains(&index)
        })
}

fn edge_required(
    graph: &WorkflowGraph,
    edge: &WorkflowEdge,
    active_nodes: &BTreeSet<String>,
    executed_nodes: &BTreeSet<String>,
    selected_handles: &BTreeMap<String, String>,
) -> bool {
    if !active_nodes.contains(&edge.source) {
        return false;
    }
    let Some(source) = graph.nodes.get(&edge.source) else {
        return false;
    };
    if !executed_nodes.contains(&edge.source) {
        return source.node_type != "if-else";
    }
    if source.node_type == "if-else" || selected_handles.contains_key(&edge.source) {
        return selected_handles
            .get(&edge.source)
            .is_some_and(|handle| edge.source_handle.as_deref() == Some(handle.as_str()));
    }
    true
}

fn activate_outgoing_edges(
    graph: &WorkflowGraph,
    node: &WorkflowNode,
    selected_handles: &BTreeMap<String, String>,
    activated_edges: &mut BTreeSet<usize>,
    active_nodes: &mut BTreeSet<String>,
) {
    let selected_handle = selected_handles.get(&node.id);
    for (index, edge) in graph.edges.iter().enumerate().filter(|(_, edge)| edge.source == node.id) {
        let activate = if node.node_type == "if-else" || selected_handle.is_some() {
            selected_handle
                .is_some_and(|handle| edge.source_handle.as_deref() == Some(handle.as_str()))
        } else {
            true
        };
        if activate {
            activated_edges.insert(index);
            active_nodes.insert(edge.target.clone());
        }
    }
}

async fn execute_node<F>(
    runtime: &WorkflowRuntime,
    graph: &WorkflowGraph,
    node: &WorkflowNode,
    pool: &mut VariablePool,
    node_index: u32,
    on_event: &mut F,
) -> NodeExecution
where
    F: FnMut(WorkflowRunEvent) + Send,
{
    let started = Instant::now();
    let result = match node.node_type.as_str() {
        "start" => Ok(execute_start_node(node, pool)),
        "llm" => execute_llm_node(runtime, node, pool, node_index, on_event).await,
        "if-else" => execute_if_else_node(node, pool),
        "code" => execute_code_node(node, pool).await,
        "answer" => Ok(execute_answer_node(node, pool)),
        "template" | "template-transform" => execute_template_node(node, pool),
        "output" | "end" => execute_output_node(node, pool),
        "variable-aggregator" => execute_variable_aggregator_node(node, pool),
        "list-operator" => execute_list_operator_node(node, pool),
        "http-request" => execute_http_request_node(node, pool).await,
        "parameter-extractor" => execute_parameter_extractor_node(runtime, node, pool).await,
        "question-classifier" => execute_question_classifier_node(runtime, graph, node, pool).await,
        "iteration" => execute_iteration_node(runtime, node, pool).await,
        "loop" => execute_loop_node(runtime, node, pool).await,
        "knowledge-retrieval" => execute_knowledge_retrieval_node(runtime, node, pool).await,
        "document-extractor" => execute_document_extractor_node(runtime, node, pool).await,
        "variable-assigner" => execute_variable_assigner_node(node, pool),
        "tool" => execute_tool_node(runtime, node, pool).await,
        "agent" => execute_agent_node(runtime, node, pool).await,
        "human-input" => execute_human_input_node(node),
        other => Err(format!("不支持的 workflow 节点类型: {other}")),
    };

    match result {
        Ok(mut execution) => {
            execution.elapsed_ms = elapsed_ms(started);
            execution
        }
        Err(error) => NodeExecution {
            status: WorkflowNodeRunStatus::Failed,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            selected_handle: None,
            answer: None,
            error: Some(error),
            elapsed_ms: elapsed_ms(started),
        },
    }
}

fn execute_start_node(node: &WorkflowNode, pool: &VariablePool) -> NodeExecution {
    let outputs = pool.node_outputs(&node.id);
    NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::new(),
        outputs,
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    }
}

async fn execute_llm_node<F>(
    runtime: &WorkflowRuntime,
    node: &WorkflowNode,
    pool: &VariablePool,
    node_index: u32,
    on_event: &mut F,
) -> Result<NodeExecution, String>
where
    F: FnMut(WorkflowRunEvent) + Send,
{
    let messages = build_llm_messages(node, pool);
    let requested_model = workflow_node_model(node, runtime.model.as_str());
    let (text, usage) = if runtime.provider.supports_streaming() {
        run_llm_chat_streaming(runtime, node, node_index, &messages, &requested_model, on_event)
            .await?
    } else {
        let response =
            run_llm_chat_with_model_fallback(runtime, &messages, &requested_model).await?;
        (response.text.unwrap_or_default(), workflow_token_usage(response.usage.as_ref()))
    };
    let mut outputs = llm_outputs(text);
    if let Some(usage) = usage {
        outputs.insert("usage".to_string(), usage);
    }
    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::from([(
            "messages".to_string(),
            serde_json::to_value(&messages).unwrap_or(Value::Null),
        )]),
        outputs,
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

fn workflow_node_model(node: &WorkflowNode, runtime_model: &str) -> String {
    let Some(model) = node.data.get("model") else {
        return runtime_model.to_string();
    };
    let Some(name) = string_field(model, "name").map(str::trim).filter(|value| !value.is_empty())
    else {
        return runtime_model.to_string();
    };
    if name.contains('/') {
        return name.to_string();
    }

    string_field(model, "provider")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map_or_else(|| name.to_string(), |provider| format!("{provider}/{name}"))
}

async fn run_llm_chat_streaming<F>(
    runtime: &WorkflowRuntime,
    node: &WorkflowNode,
    node_index: u32,
    messages: &[ChatMessage],
    requested_model: &str,
    on_event: &mut F,
) -> Result<(String, Option<Value>), String>
where
    F: FnMut(WorkflowRunEvent) + Send,
{
    let mut text = String::new();
    let mut output_tokens = 0_u64;
    let mut stream = runtime.provider.stream_chat_with_history(
        messages,
        requested_model,
        runtime.temperature,
        StreamOptions::new(true).with_token_count(),
    );

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("LLM 节点调用失败: {error}"))?;
        if chunk.is_final {
            if !chunk.delta.trim().is_empty() {
                return Err(format!("LLM 节点调用失败: {}", chunk.delta));
            }
            break;
        }
        if chunk.delta.is_empty() {
            continue;
        }
        output_tokens = output_tokens.saturating_add(u64::try_from(chunk.token_count).unwrap_or(0));
        text.push_str(&chunk.delta);
        on_event(WorkflowRunEvent::NodeDelta(WorkflowNodeDeltaEvent {
            node_id: node.id.clone(),
            node_type: node.node_type.clone(),
            title: node.title.clone(),
            index: node_index,
            text: chunk.delta,
            replace: false,
        }));
    }

    Ok((text, workflow_estimated_token_usage(output_tokens)))
}

async fn run_llm_chat_with_model_fallback(
    runtime: &WorkflowRuntime,
    messages: &[ChatMessage],
    requested_model: &str,
) -> Result<crate::providers::ChatResponse, String> {
    let first = runtime
        .provider
        .chat(ChatRequest { messages, tools: None }, requested_model, runtime.temperature)
        .await;
    match first {
        Ok(response) => Ok(response),
        Err(error)
            if requested_model != runtime.model && looks_like_unavailable_workflow_model(&error) =>
        {
            runtime
                .provider
                .chat(ChatRequest { messages, tools: None }, runtime.model.as_str(), runtime.temperature)
                .await
                .map_err(|fallback_error| {
                    format!(
                        "LLM 节点调用失败: {fallback_error}; Dify 节点模型 {requested_model} 不可用: {error}"
                    )
                })
        }
        Err(error) => Err(format!("LLM 节点调用失败: {error}")),
    }
}

fn looks_like_unavailable_workflow_model(error: &anyhow::Error) -> bool {
    let message = error.to_string();
    ["未找到模型", "model not found", "unknown model", "unsupported model"]
        .iter()
        .any(|marker| message.to_ascii_lowercase().contains(marker))
}

fn build_llm_messages(node: &WorkflowNode, pool: &VariablePool) -> Vec<ChatMessage> {
    let render_pool = llm_render_pool(node, pool);
    let pool = &render_pool;
    let mut messages = Vec::new();
    for item in array_field(&node.data, "prompt_template") {
        let role = string_field(item, "role").unwrap_or("user");
        let text = render_template(string_field(item, "text").unwrap_or_default(), pool);
        match role {
            "system" => messages.push(ChatMessage::system(text)),
            "assistant" => messages.push(ChatMessage::assistant(text)),
            _ => messages.push(ChatMessage::user(text)),
        }
    }
    if messages.is_empty() {
        let query = pool
            .get_selector(&["sys".to_string(), "query".to_string()])
            .map(value_to_text)
            .unwrap_or_default();
        messages.push(ChatMessage::user(query));
    }
    messages
}

/// 构造用于渲染 LLM prompt 的变量池：在原池基础上注入 Dify 的 `context` 变量。
///
/// Dify 的 LLM 节点通过 `data.context.variable_selector` 间接引用上游（通常是
/// knowledge-retrieval）节点的输出，并在 prompt 里以 `{{#context#}}` 展开。引擎此前
/// 不解析该字段，导致 `{{#context#}}` 始终为空，知识库检索结果无法进入 prompt。
fn llm_render_pool(node: &WorkflowNode, pool: &VariablePool) -> VariablePool {
    let Some(text) = llm_context_text(node, pool) else {
        return pool.clone();
    };
    let mut render_pool = pool.clone();
    render_pool.insert_selector(&["context".to_string()], Value::String(text));
    render_pool
}

/// 解析 LLM 节点的 `data.context`，返回拼接后的上下文纯文本。
///
/// 仅在 `enabled == true` 且 `variable_selector` 指向的值存在时返回 `Some`。检索结果为
/// chunk 数组时，提取每个元素的 `content` 拼接为纯文本（与 Dify 行为一致），避免把带转义
/// 的原始 JSON 灌给模型。
fn llm_context_text(node: &WorkflowNode, pool: &VariablePool) -> Option<String> {
    let context = node.data.get("context")?;
    if !context.get("enabled").and_then(Value::as_bool).unwrap_or(false) {
        return None;
    }
    let selector = selector_from_value(context.get("variable_selector")?);
    if selector.is_empty() {
        return None;
    }
    let value = pool.get_selector(&selector)?;
    Some(context_value_to_text(value))
}

/// 把 `context` 引用到的值转为纯文本：数组逐项提取 `content` 拼接，其余按值文本化。
fn context_value_to_text(value: &Value) -> String {
    match value {
        Value::Array(items) => items
            .iter()
            .map(|item| {
                item.get("content")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| value_to_text(item))
            })
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n"),
        other => value_to_text(other),
    }
}

fn execute_if_else_node(node: &WorkflowNode, pool: &VariablePool) -> Result<NodeExecution, String> {
    let selected = select_case_handle(array_field(&node.data, "cases"), pool)?;
    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::new(),
        outputs: BTreeMap::from([("selected_handle".to_string(), Value::String(selected.clone()))]),
        selected_handle: Some(selected),
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

async fn execute_code_node(
    node: &WorkflowNode,
    pool: &VariablePool,
) -> Result<NodeExecution, String> {
    let inputs = code_inputs(node, pool);
    let language = string_field(&node.data, "code_language").unwrap_or("python3");
    let code = string_field(&node.data, "code").ok_or_else(|| "code 节点缺少 code".to_string())?;
    let outputs = run_code_node(language, code, &inputs).await?;
    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs,
        outputs,
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

fn code_inputs(node: &WorkflowNode, pool: &VariablePool) -> BTreeMap<String, Value> {
    array_field(&node.data, "variables")
        .iter()
        .filter_map(|variable| {
            let name = string_field(variable, "variable")?.to_string();
            let selector =
                variable.get("value_selector").map(selector_from_value).unwrap_or_default();
            let value = pool.get_selector(&selector).cloned().unwrap_or(Value::Null);
            Some((name, value))
        })
        .collect()
}

fn execute_answer_node(node: &WorkflowNode, pool: &VariablePool) -> NodeExecution {
    let answer = render_template(string_field(&node.data, "answer").unwrap_or_default(), pool);
    NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::new(),
        outputs: answer_outputs(answer.clone()),
        selected_handle: None,
        answer: Some(answer),
        error: None,
        elapsed_ms: 0,
    }
}

fn execute_human_input_node(node: &WorkflowNode) -> Result<NodeExecution, String> {
    let form = node.data.get("form").cloned().unwrap_or(Value::Null);
    if form.is_null() && array_field(&node.data, "actions").is_empty() {
        return Err("human-input 节点缺少 form 或 actions".to_string());
    }
    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Paused,
        inputs: BTreeMap::new(),
        outputs: BTreeMap::from([
            ("form".to_string(), form),
            (
                "actions".to_string(),
                Value::Array(
                    human_input_actions(node)
                        .into_iter()
                        .map(|action| {
                            serde_json::json!({
                                "id": action.id,
                                "label": action.label
                            })
                        })
                        .collect(),
                ),
            ),
        ]),
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

fn execute_template_node(
    node: &WorkflowNode,
    pool: &VariablePool,
) -> Result<NodeExecution, String> {
    let template = string_field(&node.data, "template")
        .ok_or_else(|| "template 节点缺少 template".to_string())?;
    let inputs = template_inputs(node, pool);
    let output = render_jinja_value_template(template, &inputs)?;
    let value = Value::String(output);

    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs,
        outputs: BTreeMap::from([
            ("output".to_string(), value.clone()),
            ("result".to_string(), value),
        ]),
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

fn template_inputs(node: &WorkflowNode, pool: &VariablePool) -> BTreeMap<String, Value> {
    array_field(&node.data, "variables")
        .iter()
        .filter_map(|variable| {
            let name = string_field(variable, "variable")?.to_string();
            let selector =
                variable.get("value_selector").map(selector_from_value).unwrap_or_default();
            let value = pool.get_selector(&selector).cloned().unwrap_or(Value::Null);
            Some((name, value))
        })
        .collect()
}

fn execute_output_node(node: &WorkflowNode, pool: &VariablePool) -> Result<NodeExecution, String> {
    let mut outputs = BTreeMap::new();
    let items = output_items(&node.data);
    if items.is_empty() {
        return Err(format!("{} 节点缺少输出配置", node.node_type));
    }

    for item in items {
        let name = output_name(item)
            .ok_or_else(|| format!("{} 节点输出项缺少 variable/key/name", node.node_type))?;
        let selector = item
            .get("value_selector")
            .map(selector_from_value)
            .filter(|selector| !selector.is_empty())
            .ok_or_else(|| format!("{} 节点输出项 {name} 缺少 value_selector", node.node_type))?;
        let value = pool.get_selector(&selector).cloned().unwrap_or(Value::Null);
        outputs.insert(name, value);
    }

    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::new(),
        outputs,
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

fn execute_variable_aggregator_node(
    node: &WorkflowNode,
    pool: &VariablePool,
) -> Result<NodeExecution, String> {
    let items = variable_aggregator_items(&node.data);
    if items.is_empty() {
        return Err("variable-aggregator 节点缺少变量配置".to_string());
    }

    let mut inputs = BTreeMap::new();
    let mut outputs = BTreeMap::new();
    for item in items {
        let name = output_name(item)
            .ok_or_else(|| "variable-aggregator 节点变量项缺少 variable/key/name".to_string())?;
        let selectors = aggregator_selectors(item);
        if selectors.is_empty() {
            return Err(format!("variable-aggregator 节点变量项 {name} 缺少 selectors"));
        }

        let mut input_values = Vec::new();
        let mut selected = Value::Null;
        let mut expected_type = None;
        for selector in selectors {
            let value = pool.get_selector(&selector).cloned().unwrap_or(Value::Null);
            if !value.is_null() {
                let value_type = value_type_name(&value);
                if let Some(expected_type) = expected_type {
                    if expected_type != value_type {
                        return Err(format!(
                            "variable-aggregator 节点变量项 {name} 候选值类型不一致: {expected_type} 和 {value_type}"
                        ));
                    }
                } else {
                    expected_type = Some(value_type);
                }
                if selected.is_null() {
                    selected = value.clone();
                }
            }
            input_values.push(value);
        }

        inputs.insert(name.clone(), Value::Array(input_values));
        outputs.insert(name, selected);
    }

    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs,
        outputs,
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

fn execute_list_operator_node(
    node: &WorkflowNode,
    pool: &VariablePool,
) -> Result<NodeExecution, String> {
    let selector = list_operator_input_selector(&node.data)
        .ok_or_else(|| "list-operator 节点缺少 input_selector".to_string())?;
    let input = pool.get_selector(&selector).cloned().unwrap_or(Value::Null);
    let Value::Array(items) = input.clone() else {
        return Err("list-operator 节点输入必须是数组".to_string());
    };

    let mut result = filter_list_items(&items, node.data.get("filter"))?;
    sort_list_items(&mut result, node.data.get("sort"))?;

    let first_record = result.first().cloned().unwrap_or(Value::Null);
    let last_record = result.last().cloned().unwrap_or(Value::Null);
    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::from([("input".to_string(), input)]),
        outputs: BTreeMap::from([
            ("result".to_string(), Value::Array(result)),
            ("first_record".to_string(), first_record),
            ("last_record".to_string(), last_record),
        ]),
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

async fn execute_http_request_node(
    node: &WorkflowNode,
    pool: &VariablePool,
) -> Result<NodeExecution, String> {
    let template_values = template_inputs(node, pool);
    let method = http_request_method(string_field(&node.data, "method").unwrap_or("GET"))?;
    let raw_url =
        string_field(&node.data, "url").ok_or_else(|| "http-request 节点缺少 url".to_string())?;
    let url = render_http_string(raw_url, pool, &template_values)?;
    validate_http_request_url(&url)?;
    let timeout_secs = http_request_timeout_secs(node.data.get("timeout"))?;
    let headers = http_request_headers(node, pool, &template_values)?;
    let params =
        render_http_value(node.data.get("params").unwrap_or(&Value::Null), pool, &template_values)?;
    let body =
        render_http_value(node.data.get("body").unwrap_or(&Value::Null), pool, &template_values)?;
    let url = append_http_request_params(&url, &params)?;
    let method_text = method.as_str().to_string();
    let debug_headers =
        debug_json_value(&serde_json::to_value(redact_string_map(&headers)).unwrap_or(Value::Null));
    let debug_params = debug_json_value(&redact_value("params", &params));
    let debug_body = debug_json_value(&redact_value("body", &body));
    tracing::debug!(
        target: "vw_agent::workflow::http_request",
        node_id = %node.id,
        title = %node.title,
        method = %method_text,
        url = %redact_url_for_log(&url),
        headers = %debug_headers,
        params = %debug_params,
        body = %debug_body,
        timeout_secs,
        "workflow http-request sending"
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("VibeWindow Workflow HTTP Request")
        .build()
        .map_err(|error| format!("初始化 http-request client 失败: {error}"))?;
    let mut request = client.request(method, url.clone());
    for (key, value) in &headers {
        request = request.header(key, value);
    }
    request = apply_http_request_body(request, &body);

    let response =
        request.send().await.map_err(|error| format!("http-request 请求失败: {error}"))?;
    if response.content_length().is_some_and(|length| length > HTTP_REQUEST_MAX_BODY_BYTES as u64) {
        return Err(format!(
            "http-request 响应体超过限制: 最大 {} bytes",
            HTTP_REQUEST_MAX_BODY_BYTES
        ));
    }

    let status_code = response.status().as_u16();
    let response_headers = http_response_headers(response.headers());
    let body_bytes =
        response.bytes().await.map_err(|error| format!("读取 http-request 响应失败: {error}"))?;
    if body_bytes.len() > HTTP_REQUEST_MAX_BODY_BYTES {
        return Err(format!(
            "http-request 响应体超过限制: 最大 {} bytes",
            HTTP_REQUEST_MAX_BODY_BYTES
        ));
    }
    let body_text = String::from_utf8_lossy(&body_bytes).into_owned();
    let mut outputs = BTreeMap::from([
        ("status_code".to_string(), Value::Number(status_code.into())),
        ("headers".to_string(), serde_json::to_value(&response_headers).unwrap_or(Value::Null)),
        ("body".to_string(), Value::String(body_text.clone())),
    ]);
    if let Ok(json) = serde_json::from_str::<Value>(&body_text) {
        outputs.insert("json".to_string(), json);
    }
    let debug_response_headers = debug_json_value(
        &serde_json::to_value(redact_string_map(&response_headers)).unwrap_or(Value::Null),
    );
    let debug_response_body = debug_body_text(&body_text);
    tracing::debug!(
        target: "vw_agent::workflow::http_request",
        node_id = %node.id,
        title = %node.title,
        method = %method_text,
        url = %redact_url_for_log(&url),
        status_code,
        headers = %debug_response_headers,
        body = %debug_response_body,
        "workflow http-request received"
    );

    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::from([
            (
                "method".to_string(),
                Value::String(node.data["method"].as_str().unwrap_or("GET").to_string()),
            ),
            ("url".to_string(), Value::String(raw_url.to_string())),
            ("headers".to_string(), serde_json::to_value(headers).unwrap_or(Value::Null)),
            ("params".to_string(), params),
            ("body".to_string(), body),
        ]),
        outputs,
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

async fn execute_iteration_node(
    runtime: &WorkflowRuntime,
    node: &WorkflowNode,
    pool: &VariablePool,
) -> Result<NodeExecution, String> {
    reject_parallel_iteration(node)?;
    let selector = list_operator_input_selector(&node.data)
        .ok_or_else(|| "iteration 节点缺少 input_selector".to_string())?;
    let input = pool.get_selector(&selector).cloned().unwrap_or(Value::Null);
    let Value::Array(items) = input.clone() else {
        return Err("iteration 节点输入必须是数组".to_string());
    };
    let output_selector = node
        .data
        .get("output_selector")
        .map(selector_from_value)
        .filter(|selector| !selector.is_empty())
        .ok_or_else(|| "iteration 节点缺少 output_selector".to_string())?;
    let subgraph = iteration_subgraph(node)?;
    let error_strategy = iteration_error_strategy(node);

    let mut result = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let mut local_pool = pool.clone();
        local_pool.insert_node_output(&node.id, "item", item.clone());
        local_pool.insert_node_output(&node.id, "index", Value::Number(index.into()));

        match execute_workflow_subgraph(runtime, &subgraph, &mut local_pool, "iteration 子图").await
        {
            Ok(()) => result
                .push(local_pool.get_selector(&output_selector).cloned().unwrap_or(Value::Null)),
            Err(error) => match error_strategy {
                "continue_on_error" => result.push(Value::Null),
                "remove_failed" => {}
                _ => return Err(format!("iteration 子图执行失败: {error}")),
            },
        }
    }

    let value = Value::Array(result);
    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::from([("input".to_string(), input)]),
        outputs: BTreeMap::from([
            ("output".to_string(), value.clone()),
            ("result".to_string(), value),
        ]),
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

async fn execute_loop_node(
    runtime: &WorkflowRuntime,
    node: &WorkflowNode,
    pool: &VariablePool,
) -> Result<NodeExecution, String> {
    let max_count = loop_max_count(&node.data)?;
    let subgraph = loop_subgraph(node)?;
    let output_selector = node
        .data
        .get("output_selector")
        .map(selector_from_value)
        .filter(|selector| !selector.is_empty())
        .ok_or_else(|| "loop 节点缺少 output_selector".to_string())?;
    let has_stop_condition = loop_has_stop_condition(&node.data);

    let mut result = Vec::new();
    let mut last_output = Value::Null;
    let mut iterations = 0_u32;
    let mut stopped = false;

    for index in 0..max_count {
        let mut local_pool = pool.clone();
        local_pool.insert_node_output(&node.id, "index", Value::Number(index.into()));
        local_pool.insert_node_output(&node.id, "loop_count", Value::Number((index + 1).into()));
        local_pool.insert_node_output(&node.id, "last_output", last_output.clone());

        execute_workflow_subgraph(runtime, &subgraph, &mut local_pool, "loop 子图").await?;
        last_output = local_pool.get_selector(&output_selector).cloned().unwrap_or(Value::Null);
        result.push(last_output.clone());
        iterations = index + 1;

        if loop_should_stop(&node.data, &local_pool)? {
            stopped = true;
            break;
        }
    }

    if has_stop_condition && !stopped {
        return Err(format!("loop 节点达到最大次数 {max_count} 后仍未满足终止条件"));
    }

    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::from([("max_count".to_string(), Value::Number(max_count.into()))]),
        outputs: BTreeMap::from([
            ("result".to_string(), Value::Array(result)),
            ("last_output".to_string(), last_output),
            ("iterations".to_string(), Value::Number(iterations.into())),
        ]),
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

async fn execute_knowledge_retrieval_node(
    runtime: &WorkflowRuntime,
    node: &WorkflowNode,
    pool: &VariablePool,
) -> Result<NodeExecution, String> {
    let provider = runtime
        .knowledge_provider
        .as_ref()
        .ok_or_else(|| "knowledge provider 未配置".to_string())?;
    let selector = parameter_extractor_input_selector(&node.data)
        .ok_or_else(|| "knowledge-retrieval 节点缺少 query_selector".to_string())?;
    let query = pool
        .get_selector(&selector)
        .or_else(|| pool.get_selector(&system_variable_fallback_selector(&selector)?))
        .map(value_to_text)
        .unwrap_or_default();
    let request = WorkflowKnowledgeRequest {
        query: query.clone(),
        dataset_ids: string_array_field(&node.data, "dataset_ids"),
        top_k: knowledge_retrieval_field(&node.data, "top_k").and_then(Value::as_u64).unwrap_or(3)
            as usize,
        score_threshold: knowledge_retrieval_field(&node.data, "score_threshold")
            .and_then(Value::as_f64),
        metadata_filter: node.data.get("metadata_filter").cloned(),
    };
    let chunks = provider.retrieve(request.clone()).await?;
    let result = chunks.into_iter().map(knowledge_chunk_value).collect::<Vec<_>>();

    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::from([
            ("query".to_string(), Value::String(query)),
            (
                "dataset_ids".to_string(),
                Value::Array(request.dataset_ids.into_iter().map(Value::String).collect()),
            ),
            ("top_k".to_string(), Value::Number(request.top_k.into())),
            (
                "score_threshold".to_string(),
                request
                    .score_threshold
                    .and_then(serde_json::Number::from_f64)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            ),
            ("metadata_filter".to_string(), request.metadata_filter.unwrap_or(Value::Null)),
        ]),
        outputs: BTreeMap::from([("result".to_string(), Value::Array(result))]),
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

async fn execute_document_extractor_node(
    runtime: &WorkflowRuntime,
    node: &WorkflowNode,
    pool: &VariablePool,
) -> Result<NodeExecution, String> {
    let selector = list_operator_input_selector(&node.data)
        .ok_or_else(|| "document-extractor 节点缺少 input_selector".to_string())?;
    let input = pool.get_selector(&selector).cloned().unwrap_or(Value::Null);
    let files = document_files_from_value(&input)?;
    let extracted = if files.iter().all(can_extract_inline_document) {
        files.iter().map(extract_inline_document).collect::<Result<Vec<_>, _>>()?
    } else {
        let extractor = runtime
            .document_extractor
            .as_ref()
            .ok_or_else(|| document_extractor_missing_provider_error(&files))?;
        extractor.extract(WorkflowDocumentRequest { files: files.clone() }).await?
    };
    let text =
        extracted.iter().map(|document| document.text.as_str()).collect::<Vec<_>>().join("\n\n");
    let files_value = extracted.into_iter().map(|document| document.file).collect::<Vec<_>>();

    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::from([("input".to_string(), input)]),
        outputs: BTreeMap::from([
            ("text".to_string(), Value::String(text.clone())),
            ("result".to_string(), Value::String(text)),
            ("files".to_string(), Value::Array(files_value)),
        ]),
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

fn execute_variable_assigner_node(
    node: &WorkflowNode,
    pool: &mut VariablePool,
) -> Result<NodeExecution, String> {
    reject_persistent_variable_assigner(node)?;
    let assignments = array_field(&node.data, "assignments");
    if assignments.is_empty() {
        return Err("variable-assigner 节点缺少 assignments".to_string());
    }

    let mut outputs = BTreeMap::new();
    let mut inputs = BTreeMap::new();
    for assignment in assignments {
        reject_persistent_assignment(assignment)?;
        let name = output_name(assignment)
            .ok_or_else(|| "variable-assigner assignment 缺少 variable/key/name".to_string())?;
        let target_selector = variable_assigner_target_selector(assignment, &name);
        let current = pool.get_selector(&target_selector).cloned().unwrap_or(Value::Null);
        let operation = string_field(assignment, "operation")
            .or_else(|| string_field(assignment, "operator"))
            .unwrap_or("overwrite")
            .trim()
            .to_ascii_lowercase();
        let value = variable_assigner_value(assignment, pool);
        let updated = apply_variable_assignment(&operation, current, value)?;

        pool.insert_selector(&target_selector, updated.clone());
        outputs.insert(name.clone(), updated);
        inputs.insert(name, Value::String(operation));
    }

    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs,
        outputs,
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

fn reject_persistent_variable_assigner(node: &WorkflowNode) -> Result<(), String> {
    for field in ["persist", "persistent", "conversation_variable_persistent"] {
        if node.data.get(field).and_then(Value::as_bool).unwrap_or(false) {
            return Err("variable-assigner 持久化会话变量未支持".to_string());
        }
    }
    Ok(())
}

fn reject_persistent_assignment(assignment: &Value) -> Result<(), String> {
    for field in ["persist", "persistent"] {
        if assignment.get(field).and_then(Value::as_bool).unwrap_or(false) {
            return Err("variable-assigner 持久化会话变量未支持".to_string());
        }
    }
    Ok(())
}

fn variable_assigner_target_selector(assignment: &Value, name: &str) -> Vec<String> {
    ["target_selector", "variable_selector"]
        .iter()
        .find_map(|field| assignment.get(field).map(selector_from_value))
        .filter(|selector| !selector.is_empty())
        .unwrap_or_else(|| vec!["conversation".to_string(), name.to_string()])
}

fn variable_assigner_value(assignment: &Value, pool: &VariablePool) -> Value {
    assignment
        .get("value_selector")
        .map(selector_from_value)
        .and_then(|selector| pool.get_selector(&selector).cloned())
        .or_else(|| assignment.get("value").cloned())
        .unwrap_or(Value::Null)
}

fn apply_variable_assignment(
    operation: &str,
    current: Value,
    value: Value,
) -> Result<Value, String> {
    match operation {
        "overwrite" | "set" => Ok(value),
        "clear" => Ok(Value::Null),
        "add" | "+" => apply_number_assignment(current, value, |left, right| left + right),
        "subtract" | "-" => apply_number_assignment(current, value, |left, right| left - right),
        "multiply" | "*" => apply_number_assignment(current, value, |left, right| left * right),
        "divide" | "/" => {
            let right = number_value(&value)
                .ok_or_else(|| "variable-assigner divide 需要数字 value".to_string())?;
            if right == 0.0 {
                return Err("variable-assigner divide 不能除以 0".to_string());
            }
            apply_number_assignment(current, value, |left, right| left / right)
        }
        "append" => {
            let mut items = array_or_empty(current)?;
            items.push(value);
            Ok(Value::Array(items))
        }
        "extend" => {
            let mut items = array_or_empty(current)?;
            let Value::Array(next_items) = value else {
                return Err("variable-assigner extend 需要数组 value".to_string());
            };
            items.extend(next_items);
            Ok(Value::Array(items))
        }
        "remove_first" => {
            let mut items = array_or_empty(current)?;
            if !items.is_empty() {
                items.remove(0);
            }
            Ok(Value::Array(items))
        }
        "remove_last" => {
            let mut items = array_or_empty(current)?;
            items.pop();
            Ok(Value::Array(items))
        }
        other => Err(format!("不支持的 variable-assigner operation: {other}")),
    }
}

fn apply_number_assignment(
    current: Value,
    value: Value,
    op: impl Fn(f64, f64) -> f64,
) -> Result<Value, String> {
    let left = if current.is_null() {
        0.0
    } else {
        number_value(&current).ok_or_else(|| "variable-assigner 当前值不是数字".to_string())?
    };
    let right =
        number_value(&value).ok_or_else(|| "variable-assigner value 不是数字".to_string())?;
    serde_json::Number::from_f64(op(left, right))
        .map(Value::Number)
        .ok_or_else(|| "variable-assigner 数字结果无效".to_string())
}

fn number_value(value: &Value) -> Option<f64> {
    match value {
        Value::Number(value) => value.as_f64(),
        Value::String(value) => value.parse::<f64>().ok(),
        _ => None,
    }
}

fn array_or_empty(value: Value) -> Result<Vec<Value>, String> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(items) => Ok(items),
        _ => Err("variable-assigner 当前值不是数组".to_string()),
    }
}

async fn execute_tool_node(
    runtime: &WorkflowRuntime,
    node: &WorkflowNode,
    pool: &VariablePool,
) -> Result<NodeExecution, String> {
    let provider = runtime
        .tool_provider
        .as_ref()
        .ok_or_else(|| "workflow tool provider 未配置".to_string())?;
    let tool_provider = string_field(&node.data, "provider")
        .or_else(|| string_field(&node.data, "provider_name"))
        .ok_or_else(|| "tool 节点缺少 provider".to_string())?
        .trim()
        .to_string();
    if tool_provider.is_empty() {
        return Err("tool 节点 provider 不能为空".to_string());
    }
    let tool_name = string_field(&node.data, "tool_name")
        .or_else(|| string_field(&node.data, "tool"))
        .or_else(|| string_field(&node.data, "name"))
        .ok_or_else(|| "tool 节点缺少 tool_name".to_string())?
        .trim()
        .to_string();
    if tool_name.is_empty() {
        return Err("tool 节点 tool_name 不能为空".to_string());
    }

    let inputs = workflow_tool_inputs(node.data.get("inputs"), pool)?;
    let request = WorkflowToolRequest {
        provider: tool_provider,
        tool_name,
        action: workflow_tool_action(&node.data),
        credential_id: workflow_tool_credential_id(&node.data),
        inputs: inputs.clone(),
    };
    let result = provider.call(request).await?;

    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs,
        outputs: workflow_tool_outputs(result),
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

fn workflow_tool_action(data: &Value) -> Option<String> {
    string_field(data, "action")
        .or_else(|| string_field(data, "tool_action"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn workflow_tool_credential_id(data: &Value) -> Option<String> {
    string_field(data, "credential_id")
        .or_else(|| string_field(data, "credentialId"))
        .or_else(|| data.get("credential").and_then(|credential| string_field(credential, "id")))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn workflow_tool_inputs(
    inputs: Option<&Value>,
    pool: &VariablePool,
) -> Result<BTreeMap<String, Value>, String> {
    let Some(inputs) = inputs else {
        return Ok(BTreeMap::new());
    };
    match inputs {
        Value::Object(object) => {
            object.iter().map(|(name, mapping)| workflow_tool_input(name, mapping, pool)).collect()
        }
        Value::Array(items) => items
            .iter()
            .map(|item| {
                let name = output_name(item)
                    .ok_or_else(|| "tool 节点 inputs 项缺少 variable/key/name".to_string())?;
                workflow_tool_input(&name, item, pool)
            })
            .collect(),
        _ => Err("tool 节点 inputs 必须是对象或数组".to_string()),
    }
}

fn workflow_tool_input(
    name: &str,
    mapping: &Value,
    pool: &VariablePool,
) -> Result<(String, Value), String> {
    let value = if let Some(selector) = mapping
        .get("value_selector")
        .map(selector_from_value)
        .filter(|selector| !selector.is_empty())
    {
        pool.get_selector(&selector).cloned().unwrap_or(Value::Null)
    } else if let Some(value) = mapping.get("value") {
        value.clone()
    } else if mapping.is_object() {
        Value::Null
    } else {
        mapping.clone()
    };
    if let Some(type_name) =
        string_field(mapping, "type").or_else(|| string_field(mapping, "input_type"))
    {
        validate_workflow_tool_input_type(name, &value, type_name)?;
    }
    Ok((name.to_string(), value))
}

fn validate_workflow_tool_input_type(
    name: &str,
    value: &Value,
    type_name: &str,
) -> Result<(), String> {
    let expected = type_name.trim().to_ascii_lowercase();
    let matches = match expected.as_str() {
        "string" => value.is_string(),
        "number" => value.is_number(),
        "boolean" | "bool" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "any" => true,
        other => return Err(format!("tool 节点参数 {name} 不支持的类型: {other}")),
    };
    if matches {
        Ok(())
    } else {
        Err(format!(
            "tool 节点参数 {name} 类型不匹配: 期望 {expected}, 实际 {}",
            value_type_name(value)
        ))
    }
}

fn workflow_tool_outputs(result: WorkflowToolResult) -> BTreeMap<String, Value> {
    let text = result.text.unwrap_or_else(|| value_to_text(&result.result));
    let json = result.json.unwrap_or_else(|| result.result.clone());
    BTreeMap::from([
        ("result".to_string(), result.result),
        ("text".to_string(), Value::String(text)),
        ("json".to_string(), json),
        ("files".to_string(), Value::Array(result.files)),
    ])
}

async fn execute_agent_node(
    runtime: &WorkflowRuntime,
    node: &WorkflowNode,
    pool: &VariablePool,
) -> Result<NodeExecution, String> {
    let strategy = string_field(&node.data, "strategy")
        .unwrap_or("function_calling")
        .trim()
        .to_ascii_lowercase();
    if strategy != "function_calling" {
        return Err(format!("agent 节点暂不支持策略: {strategy}"));
    }
    let agent_provider = runtime
        .agent_provider
        .as_ref()
        .ok_or_else(|| "workflow agent provider 未配置".to_string())?;
    let tool_provider = runtime
        .tool_provider
        .as_ref()
        .cloned()
        .ok_or_else(|| "workflow tool provider 未配置".to_string())?;
    let tools = workflow_agent_tools(&node.data)?;
    if tools.is_empty() {
        return Err("agent 节点缺少 tools".to_string());
    }
    let max_iterations = workflow_agent_max_iterations(&node.data)?;
    let requested_model = workflow_node_model(node, runtime.model.as_str());
    let messages = build_llm_messages(node, pool);
    let request = WorkflowAgentRequest {
        messages: messages.clone(),
        tools: tools.clone(),
        max_iterations,
        model: requested_model,
        temperature: runtime.temperature,
        strategy,
    };
    let result = agent_provider.run(request, tool_provider).await?;
    if result.iterations > max_iterations {
        return Err(format!(
            "agent 节点超过最大迭代次数: {} > {}",
            result.iterations, max_iterations
        ));
    }

    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::from([
            ("messages".to_string(), serde_json::to_value(&messages).unwrap_or(Value::Null)),
            ("tools".to_string(), Value::Array(workflow_agent_tools_value(&tools))),
            ("max_iterations".to_string(), Value::Number(max_iterations.into())),
        ]),
        outputs: workflow_agent_outputs(result),
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

fn workflow_agent_max_iterations(data: &Value) -> Result<u32, String> {
    let max_iterations = data.get("max_iterations").and_then(Value::as_u64).unwrap_or(3);
    if max_iterations == 0 {
        return Err("agent 节点 max_iterations 必须大于 0".to_string());
    }
    u32::try_from(max_iterations).map_err(|_| "agent 节点 max_iterations 过大".to_string())
}

fn workflow_agent_tools(data: &Value) -> Result<Vec<WorkflowAgentTool>, String> {
    array_field(data, "tools")
        .iter()
        .map(|item| {
            let provider = string_field(item, "provider")
                .or_else(|| string_field(item, "provider_name"))
                .ok_or_else(|| "agent 节点 tool 缺少 provider".to_string())?
                .trim()
                .to_string();
            let tool_name = string_field(item, "tool_name")
                .or_else(|| string_field(item, "tool"))
                .or_else(|| string_field(item, "name"))
                .ok_or_else(|| "agent 节点 tool 缺少 tool_name".to_string())?
                .trim()
                .to_string();
            if provider.is_empty() || tool_name.is_empty() {
                return Err("agent 节点 tool provider/tool_name 不能为空".to_string());
            }
            Ok(WorkflowAgentTool {
                provider,
                tool_name,
                action: workflow_tool_action(item),
                credential_id: workflow_tool_credential_id(item),
            })
        })
        .collect()
}

fn workflow_agent_tools_value(tools: &[WorkflowAgentTool]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            let mut value = serde_json::Map::new();
            value.insert("provider".to_string(), Value::String(tool.provider.clone()));
            value.insert("tool_name".to_string(), Value::String(tool.tool_name.clone()));
            if let Some(action) = tool.action.as_ref() {
                value.insert("action".to_string(), Value::String(action.clone()));
            }
            if let Some(credential_id) = tool.credential_id.as_ref() {
                value.insert("credential_id".to_string(), Value::String(credential_id.clone()));
            }
            Value::Object(value)
        })
        .collect()
}

fn workflow_agent_outputs(result: WorkflowAgentResult) -> BTreeMap<String, Value> {
    BTreeMap::from([
        ("answer".to_string(), Value::String(result.answer.clone())),
        ("text".to_string(), Value::String(result.answer)),
        ("tool_outputs".to_string(), Value::Array(result.tool_outputs)),
        ("reasoning".to_string(), result.reasoning.map(Value::String).unwrap_or(Value::Null)),
        ("iterations".to_string(), Value::Number(result.iterations.into())),
        ("success".to_string(), Value::Bool(result.success)),
    ])
}

fn document_files_from_value(value: &Value) -> Result<Vec<WorkflowDocumentFile>, String> {
    match value {
        Value::Array(items) => items.iter().map(document_file_from_value).collect(),
        Value::Object(_) => Ok(vec![document_file_from_value(value)?]),
        Value::Null => Err("document-extractor 输入不能为空".to_string()),
        _ => Err("document-extractor 输入必须是文件对象或文件数组".to_string()),
    }
}

fn document_file_from_value(value: &Value) -> Result<WorkflowDocumentFile, String> {
    let Some(object) = value.as_object() else {
        return Err("document-extractor 文件项必须是对象".to_string());
    };
    let name = object.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    let mime_type = object
        .get("mime_type")
        .or_else(|| object.get("mimeType"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let path = object.get("path").and_then(Value::as_str).map(ToOwned::to_owned);
    let url = object.get("url").and_then(Value::as_str).map(ToOwned::to_owned);
    let size = object.get("size").and_then(Value::as_u64);
    Ok(WorkflowDocumentFile { name, mime_type, path, url, size, raw: value.clone() })
}

fn can_extract_inline_document(file: &WorkflowDocumentFile) -> bool {
    file.raw.get("text").and_then(Value::as_str).is_some() && is_inline_text_document(file)
}

fn is_inline_text_document(file: &WorkflowDocumentFile) -> bool {
    let mime = file.mime_type.as_str();
    matches!(
        mime,
        "" | "text/plain"
            | "text/markdown"
            | "text/x-markdown"
            | "application/json"
            | "application/yaml"
            | "application/x-yaml"
            | "text/yaml"
            | "text/x-yaml"
    ) || file.name.ends_with(".txt")
        || file.name.ends_with(".md")
        || file.name.ends_with(".json")
        || file.name.ends_with(".yaml")
        || file.name.ends_with(".yml")
}

fn extract_inline_document(
    file: &WorkflowDocumentFile,
) -> Result<WorkflowExtractedDocument, String> {
    if file.path.is_some() || file.url.is_some() {
        return Err("document-extractor 内置解析不读取 path/url".to_string());
    }
    let Some(text) = file.raw.get("text").and_then(Value::as_str) else {
        return Err(format!("document-extractor 文件 {} 缺少 text", file.name));
    };
    Ok(WorkflowExtractedDocument { text: text.to_string(), file: file.raw.clone() })
}

fn document_extractor_missing_provider_error(files: &[WorkflowDocumentFile]) -> String {
    if let Some(file) = files.iter().find(|file| {
        file.raw.get("text").and_then(Value::as_str).is_some() && !is_inline_text_document(file)
    }) {
        return format!(
            "document-extractor 不支持的文件格式: {}",
            if file.mime_type.is_empty() { file.name.as_str() } else { file.mime_type.as_str() }
        );
    }
    "document extractor provider 未配置".to_string()
}

fn string_array_field(data: &Value, field: &str) -> Vec<String> {
    array_field(data, field)
        .iter()
        .filter_map(|value| value.as_str().map(str::trim))
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn knowledge_chunk_value(chunk: WorkflowKnowledgeChunk) -> Value {
    let mut value = serde_json::Map::new();
    value.insert("content".to_string(), Value::String(chunk.content));
    value.insert("title".to_string(), Value::String(chunk.title));
    value.insert("metadata".to_string(), chunk.metadata);
    value.insert(
        "score".to_string(),
        chunk
            .score
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );
    Value::Object(value)
}

fn reject_parallel_iteration(node: &WorkflowNode) -> Result<(), String> {
    let parallel_enabled = node.data.get("parallel").and_then(Value::as_bool).unwrap_or(false)
        || string_field(&node.data, "mode")
            .is_some_and(|mode| mode.eq_ignore_ascii_case("parallel"));
    if parallel_enabled {
        return Err("iteration 节点暂不支持并行模式".to_string());
    }
    Ok(())
}

fn iteration_subgraph(node: &WorkflowNode) -> Result<WorkflowGraph, String> {
    let graph = node
        .data
        .get("graph")
        .or_else(|| node.data.get("children"))
        .or_else(|| node.data.get("sub_graph"))
        .ok_or_else(|| "iteration 节点缺少内部子图 graph/children/sub_graph".to_string())?;
    workflow_graph_from_value(graph, "iteration 子图")
}

fn loop_subgraph(node: &WorkflowNode) -> Result<WorkflowGraph, String> {
    let graph = node
        .data
        .get("graph")
        .or_else(|| node.data.get("children"))
        .or_else(|| node.data.get("sub_graph"))
        .ok_or_else(|| "loop 节点缺少内部子图 graph/children/sub_graph".to_string())?;
    workflow_graph_from_value(graph, "loop 子图")
}

fn workflow_graph_from_value(graph: &Value, context: &str) -> Result<WorkflowGraph, String> {
    let nodes = graph
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{context}.nodes 必须是数组"))?;
    let edges = graph.get("edges").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);

    let mut parsed_nodes = BTreeMap::new();
    for node in nodes {
        let id =
            string_field(node, "id").ok_or_else(|| format!("{context} node 缺少 id"))?.to_string();
        let data = node.get("data").cloned().unwrap_or_else(|| Value::Object(Default::default()));
        let node_type = string_field(&data, "type")
            .or_else(|| string_field(node, "type"))
            .unwrap_or("custom")
            .trim()
            .to_string();
        let title = string_field(&data, "title")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(node_type.as_str())
            .trim()
            .to_string();
        parsed_nodes.insert(id.clone(), WorkflowNode { id, node_type, title, data });
    }

    let parsed_edges = edges
        .iter()
        .filter_map(|edge| {
            let source = string_field(edge, "source")?.to_string();
            let target = string_field(edge, "target")?.to_string();
            Some(WorkflowEdge {
                source,
                target,
                source_handle: edge_handle_field(edge, "sourceHandle"),
            })
        })
        .collect::<Vec<_>>();
    let mut start_node_ids = parsed_nodes.keys().cloned().collect::<BTreeSet<_>>();
    for edge in &parsed_edges {
        start_node_ids.remove(&edge.target);
    }
    if parsed_nodes.is_empty() {
        return Err(format!("{context} 缺少节点"));
    }

    Ok(WorkflowGraph {
        nodes: parsed_nodes,
        edges: parsed_edges,
        start_node_ids: start_node_ids.into_iter().collect(),
    })
}

fn iteration_error_strategy(node: &WorkflowNode) -> &str {
    string_field(&node.data, "error_strategy")
        .or_else(|| string_field(&node.data, "error_handle_mode"))
        .unwrap_or("terminate")
}

fn loop_max_count(data: &Value) -> Result<u32, String> {
    let raw = data
        .get("max_count")
        .or_else(|| data.get("max_iterations"))
        .and_then(Value::as_u64)
        .unwrap_or(LOOP_DEFAULT_MAX_COUNT as u64);
    if raw == 0 {
        return Err("loop 节点 max_count 必须大于 0".to_string());
    }
    if raw > LOOP_MAX_COUNT as u64 {
        return Err(format!("loop 节点 max_count 不能超过 {LOOP_MAX_COUNT}"));
    }
    u32::try_from(raw).map_err(|_| "loop 节点 max_count 过大".to_string())
}

fn loop_has_stop_condition(data: &Value) -> bool {
    loop_condition_fields(data).is_some()
        || loop_stop_selector(data).is_some()
        || !array_field(data, "cases").is_empty()
}

fn loop_should_stop(data: &Value, pool: &VariablePool) -> Result<bool, String> {
    if let Some((conditions, logical_operator)) = loop_condition_fields(data) {
        if conditions_match(conditions, logical_operator, pool)? {
            return Ok(true);
        }
    }
    if let Some(selector) = loop_stop_selector(data) {
        let value = pool.get_selector(&selector).unwrap_or(&Value::Null);
        if workflow_value_truthy(value) {
            return Ok(true);
        }
    }
    let cases = array_field(data, "cases");
    if !cases.is_empty() && select_case_handle(cases, pool)? != "false" {
        return Ok(true);
    }
    Ok(false)
}

fn loop_condition_fields(data: &Value) -> Option<(&[Value], &str)> {
    for field in ["break_conditions", "termination_conditions", "conditions"] {
        let conditions = array_field(data, field);
        if !conditions.is_empty() {
            return Some((conditions, string_field(data, "logical_operator").unwrap_or("and")));
        }
    }
    for field in ["condition", "loop_condition", "exit_condition"] {
        if let Some(condition) = data.get(field) {
            let conditions = array_field(condition, "conditions");
            if !conditions.is_empty() {
                return Some((
                    conditions,
                    string_field(condition, "logical_operator").unwrap_or("and"),
                ));
            }
        }
    }
    None
}

fn loop_stop_selector(data: &Value) -> Option<Vec<String>> {
    for field in ["stop_selector", "break_selector", "termination_selector"] {
        let selector = data.get(field).map(selector_from_value).unwrap_or_default();
        if !selector.is_empty() {
            return Some(selector);
        }
    }
    None
}

fn workflow_value_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => {
            let value = value.trim();
            !value.is_empty()
                && !value.eq_ignore_ascii_case("false")
                && !value.eq_ignore_ascii_case("null")
                && value != "0"
        }
        Value::Array(items) => !items.is_empty(),
        Value::Object(object) => !object.is_empty(),
    }
}

async fn execute_workflow_subgraph(
    runtime: &WorkflowRuntime,
    graph: &WorkflowGraph,
    pool: &mut VariablePool,
    context: &str,
) -> Result<(), String> {
    let max_steps = graph.nodes.len().max(1) * 4;
    let mut active_nodes = graph.start_node_ids.iter().cloned().collect::<BTreeSet<_>>();
    let mut executed_nodes = BTreeSet::new();
    let mut activated_edges = BTreeSet::new();
    let mut selected_handles = BTreeMap::new();

    for _ in 0..max_steps {
        let Some(node_id) = next_ready_node(
            graph,
            &active_nodes,
            &executed_nodes,
            &activated_edges,
            &selected_handles,
        ) else {
            break;
        };
        let Some(node) = graph.nodes.get(&node_id).cloned() else {
            return Err(format!("{context}节点不存在: {node_id}"));
        };
        let mut ignore_event = |_| {};
        let execution =
            Box::pin(execute_node(runtime, graph, &node, pool, 0, &mut ignore_event)).await;
        for (key, value) in &execution.outputs {
            pool.insert_node_output(&node.id, key, value.clone());
        }
        if let Some(handle) = execution.selected_handle.as_ref() {
            selected_handles.insert(node.id.clone(), handle.clone());
        }
        let failed = execution.status == WorkflowNodeRunStatus::Failed;
        executed_nodes.insert(node.id.clone());
        if failed {
            return Err(execution.error.unwrap_or_else(|| format!("{context}节点失败")));
        }
        activate_outgoing_edges(
            graph,
            &node,
            &selected_handles,
            &mut activated_edges,
            &mut active_nodes,
        );
    }

    if let Some(node_id) = active_nodes.difference(&executed_nodes).next() {
        return Err(format!("{context}执行被阻塞，节点未就绪: {node_id}"));
    }
    Ok(())
}

async fn execute_parameter_extractor_node(
    runtime: &WorkflowRuntime,
    node: &WorkflowNode,
    pool: &VariablePool,
) -> Result<NodeExecution, String> {
    let selector = parameter_extractor_input_selector(&node.data)
        .ok_or_else(|| "parameter-extractor 节点缺少 input_selector".to_string())?;
    let input = pool.get_selector(&selector).map(value_to_text).unwrap_or_default();
    let parameters = parameter_definitions(&node.data)?;
    if parameters.is_empty() {
        return Err("parameter-extractor 节点缺少 parameters".to_string());
    }

    let messages = parameter_extractor_messages(&input, &parameters);
    let requested_model = workflow_node_model(node, runtime.model.as_str());
    let response = run_llm_chat_with_model_fallback(runtime, &messages, &requested_model).await?;
    let text = response.text.unwrap_or_default();
    let extracted = parse_parameter_extractor_json(&text)?;
    let mut outputs = parameter_extractor_outputs(&parameters, &extracted);
    let missing_required = missing_required_parameters(&parameters, &outputs);
    if missing_required.is_empty() {
        outputs.insert("__is_success".to_string(), Value::Number(1.into()));
        outputs.insert("__reason".to_string(), Value::String(String::new()));
    } else {
        outputs.insert("__is_success".to_string(), Value::Number(0.into()));
        outputs.insert(
            "__reason".to_string(),
            Value::String(format!("缺少必填参数: {}", missing_required.join(", "))),
        );
    }

    Ok(NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::from([
            ("input".to_string(), Value::String(input)),
            (
                "parameters".to_string(),
                serde_json::to_value(parameter_prompt_schema(&parameters)).unwrap_or(Value::Null),
            ),
        ]),
        outputs,
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    })
}

async fn execute_question_classifier_node(
    runtime: &WorkflowRuntime,
    graph: &WorkflowGraph,
    node: &WorkflowNode,
    pool: &VariablePool,
) -> Result<NodeExecution, String> {
    let selector = parameter_extractor_input_selector(&node.data)
        .ok_or_else(|| "question-classifier 节点缺少 input_selector".to_string())?;
    let input = pool.get_selector(&selector).map(value_to_text).unwrap_or_default();
    let classes = question_classes(&node.data)?;
    if classes.is_empty() {
        return Err("question-classifier 节点缺少 classes/topics".to_string());
    }

    let messages = question_classifier_messages(&input, &classes);
    let requested_model = workflow_node_model(node, runtime.model.as_str());
    let response = run_llm_chat_with_model_fallback(runtime, &messages, &requested_model).await?;
    let text = response.text.unwrap_or_default();
    let class_id = normalize_classifier_response(&text);

    if let Some(class) = classes.iter().find(|class| class.id == class_id) {
        return Ok(question_classifier_execution(
            &input,
            class.id.clone(),
            class.name.clone(),
            text,
            class.id.clone(),
        ));
    }

    if let Some(handle) = question_classifier_fallback_handle(graph, &node.id) {
        return Ok(question_classifier_execution(
            &input,
            handle.clone(),
            String::new(),
            text,
            handle,
        ));
    }

    Err(format!("question-classifier 返回未知分类 id: {class_id}，且没有 default/false 出边"))
}

fn question_classes(data: &Value) -> Result<Vec<QuestionClass>, String> {
    let items = {
        let classes = array_field(data, "classes");
        if classes.is_empty() { array_field(data, "topics") } else { classes }
    };
    items
        .iter()
        .map(|item| {
            let id = string_field(item, "id")
                .or_else(|| string_field(item, "name"))
                .ok_or_else(|| "question-classifier 分类缺少 id/name".to_string())?
                .trim()
                .to_string();
            if id.is_empty() {
                return Err("question-classifier 分类 id 不能为空".to_string());
            }
            let name = string_field(item, "name").unwrap_or(id.as_str()).trim().to_string();
            let description = string_field(item, "description").unwrap_or_default().to_string();
            Ok(QuestionClass { id, name, description })
        })
        .collect()
}

fn question_classifier_messages(input: &str, classes: &[QuestionClass]) -> Vec<ChatMessage> {
    let class_lines = classes
        .iter()
        .map(|class| {
            format!(
                "- id: {}\n  name: {}\n  description: {}",
                class.id, class.name, class.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let prompt = format!(
        "根据分类列表判断用户输入属于哪个分类。只返回一个分类 id，不要输出 Markdown、JSON 或解释。\n分类列表:\n{class_lines}\n用户输入: {input}"
    );
    vec![ChatMessage::user(prompt)]
}

fn normalize_classifier_response(text: &str) -> String {
    let trimmed = text.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if let Some(value) = value.as_str() {
            return value.trim().to_string();
        }
        if let Some(value) = value
            .get("class_id")
            .or_else(|| value.get("id"))
            .or_else(|| value.get("category"))
            .and_then(Value::as_str)
        {
            return value.trim().to_string();
        }
    }
    trimmed.trim_matches('"').trim_matches('\'').trim().to_string()
}

fn question_classifier_fallback_handle(graph: &WorkflowGraph, node_id: &str) -> Option<String> {
    for fallback in ["default", "false"] {
        if graph
            .edges
            .iter()
            .any(|edge| edge.source == node_id && edge.source_handle.as_deref() == Some(fallback))
        {
            return Some(fallback.to_string());
        }
    }
    None
}

fn question_classifier_execution(
    input: &str,
    class_id: String,
    class_name: String,
    text: String,
    selected_handle: String,
) -> NodeExecution {
    NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::from([("input".to_string(), Value::String(input.to_string()))]),
        outputs: BTreeMap::from([
            ("class_id".to_string(), Value::String(class_id)),
            ("class_name".to_string(), Value::String(class_name.clone())),
            ("class_label".to_string(), Value::String(class_name)),
            ("text".to_string(), Value::String(text)),
        ]),
        selected_handle: Some(selected_handle),
        answer: None,
        error: None,
        elapsed_ms: 0,
    }
}

/// 读取 knowledge-retrieval 检索参数：顶层字段优先，缺失时回落到 Dify 原生的
/// `multiple_retrieval_config` 嵌套字段。
fn knowledge_retrieval_field<'a>(data: &'a Value, field: &str) -> Option<&'a Value> {
    data.get(field)
        .filter(|value| !value.is_null())
        .or_else(|| data.get("multiple_retrieval_config").and_then(|config| config.get(field)))
}

/// Dify 的 knowledge-retrieval `query_variable_selector` 常形如
/// `[<start_node_id>, "sys.query"]`，其末段是点号系统变量名而非节点局部变量。
/// 当原选择器命中不到节点输出时，回落到系统变量选择器（如 `["sys", "query"]`）。
fn system_variable_fallback_selector(selector: &[String]) -> Option<Vec<String>> {
    let last = selector.last()?;
    let (namespace, name) = last.split_once('.')?;
    if namespace != "sys" || name.is_empty() {
        return None;
    }
    Some(vec![namespace.to_string(), name.to_string()])
}

fn parameter_extractor_input_selector(data: &Value) -> Option<Vec<String>> {
    ["input_selector", "query_selector", "variable_selector", "query_variable_selector"]
        .iter()
        .find_map(|field| data.get(field).map(selector_from_value))
        .filter(|selector| !selector.is_empty())
}

fn parameter_definitions(data: &Value) -> Result<Vec<ParameterDefinition>, String> {
    array_field(data, "parameters")
        .iter()
        .map(|parameter| {
            let name = string_field(parameter, "name")
                .or_else(|| string_field(parameter, "variable"))
                .ok_or_else(|| "parameter-extractor 参数缺少 name".to_string())?
                .trim()
                .to_string();
            if name.is_empty() {
                return Err("parameter-extractor 参数 name 不能为空".to_string());
            }
            let type_name = string_field(parameter, "type").unwrap_or("string").trim().to_string();
            validate_parameter_type(&type_name)?;
            let description =
                string_field(parameter, "description").unwrap_or_default().to_string();
            let required = parameter.get("required").and_then(Value::as_bool).unwrap_or(false);
            Ok(ParameterDefinition { name, type_name, description, required })
        })
        .collect()
}

fn validate_parameter_type(type_name: &str) -> Result<(), String> {
    match type_name {
        "string" | "number" | "boolean" | "array" | "object" => Ok(()),
        other => Err(format!("不支持的 parameter-extractor 参数类型: {other}")),
    }
}

fn parameter_extractor_messages(
    input: &str,
    parameters: &[ParameterDefinition],
) -> Vec<ChatMessage> {
    let schema = parameter_prompt_schema(parameters);
    let prompt = format!(
        "从用户输入中提取参数，只返回一个 JSON object，不要输出 Markdown 或解释。\n参数定义: {}\n用户输入: {}",
        serde_json::to_string(&schema).unwrap_or_else(|_| "[]".to_string()),
        input
    );
    vec![ChatMessage::user(prompt)]
}

fn parameter_prompt_schema(parameters: &[ParameterDefinition]) -> Vec<Value> {
    parameters
        .iter()
        .map(|parameter| {
            let mut value = serde_json::Map::new();
            value.insert("name".to_string(), Value::String(parameter.name.clone()));
            value.insert("type".to_string(), Value::String(parameter.type_name.clone()));
            value.insert("required".to_string(), Value::Bool(parameter.required));
            value.insert("description".to_string(), Value::String(parameter.description.clone()));
            Value::Object(value)
        })
        .collect()
}

fn parse_parameter_extractor_json(text: &str) -> Result<Value, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("parameter-extractor LLM 返回为空，无法解析 JSON".to_string());
    }
    serde_json::from_str::<Value>(trimmed)
        .map_err(|error| format!("parameter-extractor LLM 返回非 JSON: {error}"))
        .and_then(|value| {
            if value.is_object() {
                Ok(value)
            } else {
                Err("parameter-extractor LLM 返回 JSON 必须是 object".to_string())
            }
        })
}

fn parameter_extractor_outputs(
    parameters: &[ParameterDefinition],
    extracted: &Value,
) -> BTreeMap<String, Value> {
    let mut outputs = BTreeMap::new();
    for parameter in parameters {
        let value = extracted.get(&parameter.name).cloned().unwrap_or(Value::Null);
        outputs.insert(parameter.name.clone(), value);
    }
    outputs
}

fn missing_required_parameters(
    parameters: &[ParameterDefinition],
    outputs: &BTreeMap<String, Value>,
) -> Vec<String> {
    parameters
        .iter()
        .filter(|parameter| parameter.required)
        .filter(|parameter| {
            outputs
                .get(&parameter.name)
                .is_none_or(|value| value.is_null() || value.as_str().is_some_and(str::is_empty))
        })
        .map(|parameter| parameter.name.clone())
        .collect()
}

fn http_request_method(method: &str) -> Result<reqwest::Method, String> {
    match method.trim().to_ascii_uppercase().as_str() {
        "GET" => Ok(reqwest::Method::GET),
        "POST" => Ok(reqwest::Method::POST),
        "PUT" => Ok(reqwest::Method::PUT),
        "PATCH" => Ok(reqwest::Method::PATCH),
        "DELETE" => Ok(reqwest::Method::DELETE),
        "HEAD" => Ok(reqwest::Method::HEAD),
        other => Err(format!("不支持的 http-request method: {other}")),
    }
}

fn validate_http_request_url(url: &str) -> Result<(), String> {
    let parsed =
        reqwest::Url::parse(url).map_err(|error| format!("http-request URL 无效: {error}"))?;
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        scheme => Err(format!("http-request 仅支持 http/https URL，当前 scheme: {scheme}")),
    }
}

fn http_request_timeout_secs(value: Option<&Value>) -> Result<u64, String> {
    let timeout = value.and_then(Value::as_u64).unwrap_or(HTTP_REQUEST_DEFAULT_TIMEOUT_SECS);
    if timeout == 0 {
        return Err("http-request timeout 必须大于 0".to_string());
    }
    Ok(timeout.min(HTTP_REQUEST_MAX_TIMEOUT_SECS))
}

fn http_request_headers(
    node: &WorkflowNode,
    pool: &VariablePool,
    template_values: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, String>, String> {
    let mut headers = BTreeMap::new();
    if let Some(value) = node.data.get("headers") {
        for (key, value) in http_string_map(value, "headers")? {
            headers.insert(key, render_http_string(&value, pool, template_values)?);
        }
    }
    if let Some(auth) = node.data.get("authorization").or_else(|| node.data.get("auth")) {
        for (key, value) in http_string_map(auth, "authorization")? {
            headers.insert(key, render_http_string(&value, pool, template_values)?);
        }
    }
    Ok(headers)
}

fn http_string_map(value: &Value, field: &str) -> Result<BTreeMap<String, String>, String> {
    let Some(object) = value.as_object() else {
        return Err(format!("http-request {field} 必须是对象"));
    };
    let mut map = BTreeMap::new();
    for (key, value) in object {
        map.insert(key.clone(), value_to_text(value));
    }
    Ok(map)
}

fn render_http_value(
    value: &Value,
    pool: &VariablePool,
    template_values: &BTreeMap<String, Value>,
) -> Result<Value, String> {
    match value {
        Value::String(value) => {
            Ok(Value::String(render_http_string(value, pool, template_values)?))
        }
        Value::Array(items) => items
            .iter()
            .map(|item| render_http_value(item, pool, template_values))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(object) => object
            .iter()
            .map(|(key, value)| Ok((key.clone(), render_http_value(value, pool, template_values)?)))
            .collect::<Result<serde_json::Map<_, _>, String>>()
            .map(Value::Object),
        other => Ok(other.clone()),
    }
}

fn render_http_string(
    value: &str,
    pool: &VariablePool,
    template_values: &BTreeMap<String, Value>,
) -> Result<String, String> {
    let value = render_template(value, pool);
    render_jinja_value_template(&value, template_values)
}

fn append_http_request_params(raw_url: &str, params: &Value) -> Result<String, String> {
    if params.is_null() {
        return Ok(raw_url.to_string());
    }
    let mut url =
        reqwest::Url::parse(raw_url).map_err(|error| format!("http-request URL 无效: {error}"))?;
    let pairs = http_query_pairs(params)?;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in pairs {
            query.append_pair(&key, &value);
        }
    }
    Ok(url.to_string())
}

fn http_query_pairs(params: &Value) -> Result<Vec<(String, String)>, String> {
    let mut pairs = Vec::new();
    match params {
        Value::Object(object) => {
            for (key, value) in object {
                push_http_query_value(&mut pairs, key, value);
            }
        }
        Value::Array(items) => {
            for item in items {
                let Some(pair) = item.as_array().filter(|pair| pair.len() == 2) else {
                    return Err("http-request params 数组项必须是 [key, value]".to_string());
                };
                let Some(key) = pair[0].as_str() else {
                    return Err("http-request params key 必须是字符串".to_string());
                };
                push_http_query_value(&mut pairs, key, &pair[1]);
            }
        }
        _ => return Err("http-request params 必须是对象或键值数组".to_string()),
    }
    Ok(pairs)
}

fn push_http_query_value(pairs: &mut Vec<(String, String)>, key: &str, value: &Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                pairs.push((key.to_string(), value_to_text(item)));
            }
        }
        other => pairs.push((key.to_string(), value_to_text(other))),
    }
}

fn apply_http_request_body(
    builder: reqwest::RequestBuilder,
    body: &Value,
) -> reqwest::RequestBuilder {
    match body {
        Value::Null => builder,
        Value::String(value) => builder.body(value.clone()),
        other => builder.json(other),
    }
}

fn http_response_headers(headers: &reqwest::header::HeaderMap) -> BTreeMap<String, String> {
    headers
        .iter()
        .filter_map(|(key, value)| Some((key.to_string(), value.to_str().ok()?.to_string())))
        .collect()
}

fn list_operator_input_selector(data: &Value) -> Option<Vec<String>> {
    ["input_selector", "list_selector", "variable_selector"]
        .iter()
        .find_map(|field| data.get(field).map(selector_from_value))
        .filter(|selector| !selector.is_empty())
}

fn filter_list_items(items: &[Value], filter: Option<&Value>) -> Result<Vec<Value>, String> {
    let Some(filter) = filter else {
        return Ok(items.to_vec());
    };
    let field = string_field(filter, "field")
        .filter(|field| !field.trim().is_empty())
        .ok_or_else(|| "list-operator filter 缺少 field".to_string())?;
    let operator = string_field(filter, "operator").unwrap_or("=").trim().to_ascii_lowercase();
    let expected = filter.get("value").unwrap_or(&Value::Null);
    let mut filtered = Vec::new();
    for item in items {
        let actual = value_at_field_path(item, field).unwrap_or(&Value::Null);
        if list_filter_matches(actual, &operator, expected)? {
            filtered.push(item.clone());
        }
    }
    Ok(filtered)
}

fn list_filter_matches(actual: &Value, operator: &str, expected: &Value) -> Result<bool, String> {
    let matches = match operator {
        "=" | "is" | "equals" => values_equal_for_list(actual, expected),
        "contains" => value_to_text(actual).contains(&value_to_text(expected)),
        "in" => expected
            .as_array()
            .is_some_and(|items| items.iter().any(|item| values_equal_for_list(actual, item))),
        other => return Err(format!("不支持的 list-operator 过滤运算符: {other}")),
    };
    Ok(matches)
}

fn sort_list_items(items: &mut [Value], sort: Option<&Value>) -> Result<(), String> {
    let Some(sort) = sort else {
        return Ok(());
    };
    let field = string_field(sort, "field")
        .filter(|field| !field.trim().is_empty())
        .ok_or_else(|| "list-operator sort 缺少 field".to_string())?;
    let descending = string_field(sort, "order")
        .or_else(|| string_field(sort, "direction"))
        .unwrap_or("asc")
        .eq_ignore_ascii_case("desc");

    items.sort_by(|left, right| {
        let ordering = compare_list_sort_values(
            value_at_field_path(left, field).unwrap_or(&Value::Null),
            value_at_field_path(right, field).unwrap_or(&Value::Null),
        );
        if descending { ordering.reverse() } else { ordering }
    });
    Ok(())
}

fn compare_list_sort_values(left: &Value, right: &Value) -> Ordering {
    match (left.as_f64(), right.as_f64()) {
        (Some(left), Some(right)) => left.partial_cmp(&right).unwrap_or(Ordering::Equal),
        _ => value_to_text(left).cmp(&value_to_text(right)),
    }
}

fn value_at_field_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in path.split('.').filter(|part| !part.is_empty()) {
        current = current.get(part)?;
    }
    Some(current)
}

fn values_equal_for_list(actual: &Value, expected: &Value) -> bool {
    actual == expected || value_to_text(actual).trim() == value_to_text(expected).trim()
}

fn variable_aggregator_items(data: &Value) -> &[Value] {
    for field in ["variables", "groups"] {
        let items = array_field(data, field);
        if !items.is_empty() {
            return items;
        }
    }
    &[]
}

fn aggregator_selectors(item: &Value) -> Vec<Vec<String>> {
    ["selectors", "value_selectors", "variable_selectors"]
        .iter()
        .find_map(|field| item.get(field).and_then(Value::as_array))
        .map(|selectors| {
            selectors
                .iter()
                .map(selector_from_value)
                .filter(|selector| !selector.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn output_items(data: &Value) -> &[Value] {
    for field in ["outputs", "output_variables", "variables"] {
        let items = array_field(data, field);
        if !items.is_empty() {
            return items;
        }
    }
    &[]
}

fn output_name(item: &Value) -> Option<String> {
    ["variable", "key", "name"]
        .iter()
        .find_map(|field| string_field(item, field))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn llm_outputs(text: String) -> BTreeMap<String, Value> {
    let value = Value::String(text.clone());
    let mut outputs = BTreeMap::from([
        ("text".to_string(), value.clone()),
        ("answer".to_string(), value.clone()),
        ("result".to_string(), value),
    ]);

    if let Ok(json) = serde_json::from_str::<Value>(&text) {
        outputs.insert("json".to_string(), json);
    }

    outputs
}

fn workflow_token_usage(usage: Option<&TokenUsage>) -> Option<Value> {
    let usage = usage?;
    if usage.input_tokens.is_none()
        && usage.output_tokens.is_none()
        && usage.cached_tokens.is_none()
        && usage.reasoning_tokens.is_none()
    {
        return None;
    }

    let input_tokens = usage.input_tokens.unwrap_or(0);
    let output_tokens = usage.output_tokens.unwrap_or(0);
    let cached_tokens = usage.cached_tokens.unwrap_or(0);
    let reasoning_tokens = usage.reasoning_tokens.unwrap_or(0);
    let total_tokens = input_tokens.saturating_add(output_tokens).saturating_add(reasoning_tokens);

    Some(serde_json::json!({
        "prompt_tokens": input_tokens,
        "completion_tokens": output_tokens,
        "total_tokens": total_tokens,
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
        "cached_tokens": cached_tokens,
        "reasoning_tokens": reasoning_tokens,
    }))
}

fn workflow_estimated_token_usage(output_tokens: u64) -> Option<Value> {
    if output_tokens == 0 {
        return None;
    }
    Some(serde_json::json!({
        "prompt_tokens": 0,
        "completion_tokens": output_tokens,
        "total_tokens": output_tokens,
        "input_tokens": 0,
        "output_tokens": output_tokens,
        "cached_tokens": 0,
        "reasoning_tokens": 0,
    }))
}

fn workflow_node_delta_from_execution(
    node: &WorkflowNode,
    index: u32,
    execution: &NodeExecution,
) -> Option<WorkflowNodeDeltaEvent> {
    if node.node_type == "llm" {
        return None;
    }
    let text = truncate_workflow_node_delta(&workflow_node_delta_text(execution)?);
    Some(WorkflowNodeDeltaEvent {
        node_id: node.id.clone(),
        node_type: node.node_type.clone(),
        title: node.title.clone(),
        index,
        text,
        replace: true,
    })
}

fn truncate_workflow_node_delta(text: &str) -> String {
    let char_count = text.chars().count();
    if char_count <= WORKFLOW_NODE_DELTA_MAX_CHARS {
        return text.to_string();
    }

    let head_len = WORKFLOW_NODE_DELTA_MAX_CHARS * 2 / 3;
    let tail_len = WORKFLOW_NODE_DELTA_MAX_CHARS.saturating_sub(head_len);
    let head = text.chars().take(head_len).collect::<String>();
    let tail =
        text.chars().rev().take(tail_len).collect::<String>().chars().rev().collect::<String>();
    let omitted = char_count.saturating_sub(head_len + tail_len);
    format!("{head}\n\n... 已截断 {omitted} 字符，仅保留节点输出预览 ...\n\n{tail}")
}

fn workflow_node_delta_text(execution: &NodeExecution) -> Option<String> {
    if let Some(error) = execution.error.as_deref().map(str::trim).filter(|value| !value.is_empty())
    {
        return Some(error.to_string());
    }
    if let Some(answer) =
        execution.answer.as_deref().map(str::trim).filter(|value| !value.is_empty())
    {
        return Some(answer.to_string());
    }
    for key in ["answer", "text", "result"] {
        if let Some(text) = execution
            .outputs
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(text.to_string());
        }
    }
    if execution.outputs.is_empty() {
        return None;
    }
    serde_json::to_string(&redact_map(&execution.outputs)).ok()
}

fn answer_outputs(answer: String) -> BTreeMap<String, Value> {
    let value = Value::String(answer);
    BTreeMap::from([("answer".to_string(), value.clone()), ("text".to_string(), value)])
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn redact_map(values: &BTreeMap<String, Value>) -> BTreeMap<String, Value> {
    values.iter().map(|(key, value)| (key.clone(), redact_value(key, value))).collect()
}

fn redact_string_map(values: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|(key, value)| {
            let value =
                if is_sensitive_key(key) { "[REDACTED]".to_string() } else { value.clone() };
            (key.clone(), value)
        })
        .collect()
}

fn redact_value(key: &str, value: &Value) -> Value {
    let lower = key.to_ascii_lowercase();
    if ["token", "secret", "password", "api_key", "authorization", "auth", "skey"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return Value::String("[REDACTED]".to_string());
    }
    match value {
        Value::Object(object) => Value::Object(
            object.iter().map(|(key, value)| (key.clone(), redact_value(key, value))).collect(),
        ),
        Value::Array(items) => {
            Value::Array(items.iter().map(|item| redact_value(key, item)).collect())
        }
        other => other.clone(),
    }
}

fn debug_json_value(value: &Value) -> String {
    let text =
        serde_json::to_string(value).unwrap_or_else(|_| "<json serialize failed>".to_string());
    truncate_debug_text(text)
}

fn debug_body_text(text: &str) -> String {
    let value = serde_json::from_str::<Value>(text)
        .map(|value| redact_value("body", &value))
        .unwrap_or_else(|_| Value::String(text.to_string()));
    debug_json_value(&value)
}

fn truncate_debug_text(value: String) -> String {
    if value.chars().count() <= WORKFLOW_DEBUG_MAX_CHARS {
        return value;
    }
    let mut truncated: String = value.chars().take(WORKFLOW_DEBUG_MAX_CHARS).collect();
    truncated.push_str("...");
    truncated
}

fn redact_url_for_log(raw_url: &str) -> String {
    let Ok(mut url) = reqwest::Url::parse(raw_url) else {
        return truncate_debug_text(raw_url.to_string());
    };
    if !url.username().is_empty() {
        let _ = url.set_username("[REDACTED]");
    }
    if url.password().is_some() {
        let _ = url.set_password(Some("[REDACTED]"));
    }
    let query_pairs = url
        .query_pairs()
        .map(|(key, value)| {
            let value =
                if is_sensitive_key(&key) { "[REDACTED]".to_string() } else { value.into_owned() };
            (key.into_owned(), value)
        })
        .collect::<Vec<_>>();
    if !query_pairs.is_empty() {
        url.set_query(None);
        {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in query_pairs {
                pairs.append_pair(&key, &value);
            }
        }
    }
    truncate_debug_text(url.to_string())
}

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    ["token", "secret", "password", "api_key", "authorization", "auth", "skey", "cookie"]
        .iter()
        .any(|marker| lower.contains(marker))
}

#[cfg(test)]
#[path = "runner_tests.rs"]
mod runner_tests;
