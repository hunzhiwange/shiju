//! Workflow 网关路由。

use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post},
};
use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap},
    convert::Infallible,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use vw_api_types::TenantScope;
use vw_api_types::workflow::{
    WorkflowNodeRunDto, WorkflowNodeRunStatus, WorkflowPauseDto, WorkflowRecord,
    WorkflowRecordDeleteResponse, WorkflowRecordSummary, WorkflowRecordUpsertBody,
    WorkflowResumeRequest, WorkflowRunRequest, WorkflowRunResponse, WorkflowRunStatus,
};

use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::state::AppState;
use crate::app::agent::tools::{
    ToolCallError, ToolRuntimeContext, ToolUseContext, execute_tool_call,
};
use crate::global_database::GlobalDatabase;
use crate::workflow::{
    WorkflowNodeDeltaEvent, WorkflowNodeStartedEvent, WorkflowRunEvent, WorkflowRuntime,
    WorkflowToolProvider, WorkflowToolRequest, WorkflowToolResult, resume_workflow, run_workflow,
    run_workflow_with_events,
};

const WORKFLOW_SSE_NODE_OUTPUT_MAX_CHARS: usize = 12_000;
const WORKFLOW_INPUT_FULL_ACCESS: &str = "__vw_full_access";
const WORKFLOW_INPUT_ROOT: &str = "__vw_root";

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/workflow/resume", post(workflow_resume))
        .route("/workflow/applications/chat-messages", post(workflow_applications_chat_messages))
        .route(
            "/workflow/applications/{uuid}/chat-messages",
            post(workflow_application_chat_messages),
        )
        .merge(applications_router())
}

pub(crate) fn applications_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/workflow/applications",
            get(workflow_applications_list).post(workflow_application_create),
        )
        .route(
            "/workflow/applications/{uuid}",
            get(workflow_application_get)
                .put(workflow_application_update)
                .delete(workflow_application_delete),
        )
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum VibeWindowResponseMode {
    Streaming,
    #[default]
    Blocking,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct VibeWindowChatRequest {
    query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    application_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    application_workflow: Option<String>,
    #[serde(default)]
    inputs: BTreeMap<String, Value>,
    #[serde(default)]
    response_mode: VibeWindowResponseMode,
    user: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    conversation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    files: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    auto_generate_name: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VibeWindowUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VibeWindowResponseMetadata {
    usage: VibeWindowUsage,
    retriever_resources: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VibeWindowChatResponse {
    event: String,
    task_id: String,
    id: String,
    message_id: String,
    conversation_id: String,
    mode: String,
    answer: String,
    metadata: VibeWindowResponseMetadata,
    created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
struct VibeWindowWorkflowStartedData {
    id: String,
    workflow_id: String,
    sequence_number: Option<u32>,
    inputs: Value,
    created_at: u64,
}

#[derive(Debug, Clone, Serialize)]
struct VibeWindowNodeStartedData {
    id: String,
    node_id: String,
    node_type: String,
    title: String,
    index: u32,
    predecessor_node_id: Option<String>,
    inputs: Option<Value>,
    created_at: u64,
    extras: HashMap<String, Value>,
    parallel_id: Option<String>,
    parallel_start_node_id: Option<String>,
    parent_parallel_id: Option<String>,
    parent_parallel_start_node_id: Option<String>,
    iteration_id: Option<String>,
    loop_id: Option<String>,
    parallel_run_id: Option<String>,
    agent_strategy: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
struct VibeWindowNodeFinishedData {
    id: String,
    node_id: String,
    index: u32,
    predecessor_node_id: Option<String>,
    inputs: Option<Value>,
    process_data: Option<Value>,
    outputs: Option<Value>,
    status: String,
    error: Option<String>,
    elapsed_time: Option<f64>,
    created_at: u64,
}

#[derive(Debug, Clone, Serialize)]
struct VibeWindowTextChunkData {
    text: String,
    from_variable_selector: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct VibeWindowWorkflowFinishedData {
    id: String,
    workflow_id: String,
    status: String,
    outputs: Option<Value>,
    error: Option<String>,
    elapsed_time: Option<f64>,
    total_tokens: Option<u32>,
    total_steps: u32,
    created_at: u64,
    finished_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pause: Option<WorkflowPauseDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum VibeWindowSseEvent {
    WorkflowStarted {
        task_id: String,
        workflow_run_id: String,
        data: VibeWindowWorkflowStartedData,
    },
    NodeStarted {
        task_id: String,
        workflow_run_id: String,
        data: VibeWindowNodeStartedData,
    },
    NodeFinished {
        task_id: String,
        workflow_run_id: String,
        data: VibeWindowNodeFinishedData,
    },
    TextChunk {
        task_id: String,
        workflow_run_id: String,
        data: VibeWindowTextChunkData,
    },
    Message {
        id: String,
        task_id: String,
        message_id: String,
        conversation_id: String,
        answer: String,
        created_at: i64,
    },
    WorkflowFinished {
        task_id: String,
        workflow_run_id: String,
        data: VibeWindowWorkflowFinishedData,
    },
    MessageEnd {
        id: String,
        task_id: String,
        message_id: String,
        conversation_id: String,
        metadata: HashMap<String, Value>,
    },
    Error {
        task_id: String,
        status: u32,
        code: String,
        message: String,
    },
}

async fn workflow_applications_chat_messages(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    Json(body): Json<VibeWindowChatRequest>,
) -> Result<Response, ApiError> {
    chat_messages_inner(state, tenant, body).await
}

async fn chat_messages_inner(
    state: AppState,
    tenant: TenantScope,
    body: VibeWindowChatRequest,
) -> Result<Response, ApiError> {
    if body.user.trim().is_empty() {
        return Err(ApiError::bad_request("user is required"));
    }
    if body.query.trim().is_empty() {
        return Err(ApiError::bad_request("query is required"));
    }

    let response_mode = body.response_mode.clone();
    let conversation_id = body.conversation_id.clone();
    let mut request = chat_request_to_workflow_request(body)?;

    match response_mode {
        VibeWindowResponseMode::Blocking => {
            hydrate_workflow_yaml_from_uuid(workflow_database()?, tenant.clone(), &mut request)
                .await?;
            let runtime = workflow_runtime(&state, tenant, &request)?;
            strip_workflow_runtime_inputs(&mut request);
            let response = run_workflow(runtime, request).await.map_err(ApiError::bad_request)?;
            let chat_response = workflow_response_to_chat_response(&response, conversation_id);
            Ok(Json(chat_response).into_response())
        }
        VibeWindowResponseMode::Streaming => {
            Ok(workflow_streaming_response(state, tenant, request, conversation_id))
        }
    }
}

async fn workflow_application_chat_messages(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    Path(uuid): Path<String>,
    Json(mut body): Json<VibeWindowChatRequest>,
) -> Result<Response, ApiError> {
    apply_path_chat_workflow_uuid(&mut body, &uuid)?;
    chat_messages_inner(state, tenant, body).await
}

async fn workflow_resume(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    Json(body): Json<WorkflowResumeRequest>,
) -> Result<Json<WorkflowRunResponse>, ApiError> {
    let runtime = workflow_runtime(&state, tenant, &WorkflowRunRequest::default())?;
    let response = resume_workflow(runtime, body).await.map_err(ApiError::bad_request)?;
    Ok(Json(response))
}

async fn workflow_applications_list(
    Extension(tenant): Extension<TenantScope>,
) -> Result<Json<Vec<WorkflowRecordSummary>>, ApiError> {
    let records = super::workflow_store::list_records(workflow_database()?, tenant).await?;
    Ok(Json(records))
}

async fn workflow_application_get(
    Extension(tenant): Extension<TenantScope>,
    Path(uuid): Path<String>,
) -> Result<Json<WorkflowRecord>, ApiError> {
    let record = super::workflow_store::get_record(workflow_database()?, tenant, uuid).await?;
    Ok(Json(record))
}

async fn workflow_application_create(
    Extension(tenant): Extension<TenantScope>,
    Json(body): Json<WorkflowRecordUpsertBody>,
) -> Result<Json<WorkflowRecord>, ApiError> {
    let record = super::workflow_store::create_record(workflow_database()?, tenant, body).await?;
    Ok(Json(record))
}

async fn workflow_application_update(
    Extension(tenant): Extension<TenantScope>,
    Path(uuid): Path<String>,
    Json(body): Json<WorkflowRecordUpsertBody>,
) -> Result<Json<WorkflowRecord>, ApiError> {
    let record =
        super::workflow_store::update_record(workflow_database()?, tenant, uuid, body).await?;
    Ok(Json(record))
}

async fn workflow_application_delete(
    Extension(tenant): Extension<TenantScope>,
    Path(uuid): Path<String>,
) -> Result<Json<WorkflowRecordDeleteResponse>, ApiError> {
    let response = super::workflow_store::delete_record(workflow_database()?, tenant, uuid).await?;
    Ok(Json(response))
}

fn apply_path_chat_workflow_uuid(
    body: &mut VibeWindowChatRequest,
    uuid: &str,
) -> Result<(), ApiError> {
    let path_uuid = validate_path_workflow_uuid(uuid)?;
    if body.application_uuid.is_some() {
        return Err(ApiError::bad_request("application_uuid is not allowed when uuid is in path"));
    }
    if body.inputs.contains_key("application_uuid") {
        return Err(ApiError::bad_request("inputs.application_uuid is not allowed; use path uuid"));
    }
    if body.inputs.contains_key("workflow_uuid") {
        return Err(ApiError::bad_request(
            "inputs.workflow_uuid is no longer supported; use path uuid",
        ));
    }
    if body
        .application_workflow
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        return Err(ApiError::bad_request(
            "application_workflow is not allowed when uuid is in path",
        ));
    }
    if body.inputs.contains_key("application_workflow") {
        return Err(ApiError::bad_request(
            "inputs.application_workflow is not allowed when uuid is in path",
        ));
    }
    if body.inputs.contains_key("workflow_yaml") {
        return Err(ApiError::bad_request(
            "inputs.workflow_yaml is no longer supported; use path uuid",
        ));
    }
    body.application_uuid = Some(path_uuid.to_string());
    Ok(())
}

fn validate_path_workflow_uuid(uuid: &str) -> Result<&str, ApiError> {
    let trimmed = uuid.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request("workflow uuid is required"));
    }
    Ok(trimmed)
}

async fn hydrate_workflow_yaml_from_uuid(
    database: GlobalDatabase,
    tenant: TenantScope,
    body: &mut WorkflowRunRequest,
) -> Result<(), ApiError> {
    let Some(uuid) = body.workflow_uuid.clone() else {
        return Ok(());
    };
    if body.workflow_yaml.is_some() {
        return Ok(());
    }

    let record = super::workflow_store::get_record(database, tenant, uuid).await?;
    body.workflow_yaml = Some(record.workflow_yaml);
    Ok(())
}

fn workflow_database() -> Result<GlobalDatabase, ApiError> {
    GlobalDatabase::for_home(&crate::global::paths().home).map_err(|error| {
        ApiError::internal(format!("initialize workflow global database failed: {error:#}"))
    })
}

fn workflow_runtime(
    state: &AppState,
    tenant: TenantScope,
    request: &WorkflowRunRequest,
) -> Result<WorkflowRuntime, ApiError> {
    Ok(WorkflowRuntime {
        provider: state.provider.clone(),
        knowledge_provider: Some(Arc::new(super::knowledge::knowledge_store(state, tenant)?)),
        document_extractor: None,
        tool_provider: Some(Arc::new(GatewayWorkflowToolProvider {
            ctx: workflow_tool_runtime_context(request),
        })),
        agent_provider: None,
        pause_store: None,
        model: state.model.clone(),
        temperature: state.temperature,
    })
}

struct GatewayWorkflowToolProvider {
    ctx: ToolRuntimeContext,
}

#[async_trait::async_trait]
impl WorkflowToolProvider for GatewayWorkflowToolProvider {
    async fn call(&self, request: WorkflowToolRequest) -> Result<WorkflowToolResult, String> {
        let input = Value::Object(request.inputs.into_iter().collect());
        let input_text = serde_json::to_string(&input)
            .map_err(|error| format!("workflow tool 输入序列化失败: {error}"))?;
        let result = execute_tool_call(request.tool_name.as_str(), &input_text, &self.ctx)
            .map_err(workflow_tool_call_error_text)?;
        let model_result = result.default_model_result();
        let model_text = result.model_text();
        let json = (!result.data.is_null()).then_some(result.data.clone());

        Ok(WorkflowToolResult {
            result: if result.data.is_null() { model_result } else { result.data },
            text: (!model_text.trim().is_empty()).then_some(model_text),
            json,
            files: Vec::new(),
        })
    }
}

fn workflow_tool_call_error_text(error: ToolCallError) -> String {
    match error {
        ToolCallError::Denied { message, .. } | ToolCallError::Failed(message) => message,
    }
}

fn workflow_tool_runtime_context(request: &WorkflowRunRequest) -> ToolRuntimeContext {
    let root = workflow_request_root(request);
    let full_access = workflow_request_full_access(request);
    let mut tool_use_context = ToolUseContext::new("workflow", root.clone())
        .with_full_access_enabled(full_access)
        .with_channel("workflow");
    if full_access {
        tool_use_context = tool_use_context.with_bypass_non_cli_approval_for_turn(true);
    }

    ToolRuntimeContext::new("workflow", root).with_tool_use_context(tool_use_context)
}

fn workflow_request_full_access(request: &WorkflowRunRequest) -> bool {
    request.inputs.get(WORKFLOW_INPUT_FULL_ACCESS).and_then(Value::as_bool).unwrap_or(false)
}

fn workflow_request_root(request: &WorkflowRunRequest) -> Option<String> {
    request
        .inputs
        .get(WORKFLOW_INPUT_ROOT)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn strip_workflow_runtime_inputs(request: &mut WorkflowRunRequest) {
    request.inputs.remove(WORKFLOW_INPUT_FULL_ACCESS);
    request.inputs.remove(WORKFLOW_INPUT_ROOT);
}

fn chat_request_to_workflow_request(
    body: VibeWindowChatRequest,
) -> Result<WorkflowRunRequest, ApiError> {
    let mut inputs = body.inputs;
    if inputs.contains_key("application_uuid") {
        return Err(ApiError::bad_request("application_uuid must be top-level"));
    }
    if inputs.contains_key("application_workflow") {
        return Err(ApiError::bad_request("application_workflow must be top-level"));
    }
    if inputs.contains_key("workflow_uuid") {
        return Err(ApiError::bad_request(
            "workflow_uuid is no longer supported; use application_uuid",
        ));
    }
    if inputs.contains_key("workflow_yaml") {
        return Err(ApiError::bad_request(
            "workflow_yaml is no longer supported; use application_workflow",
        ));
    }
    let workflow_uuid = normalized_optional_string(body.application_uuid);
    let workflow_yaml = normalized_optional_string(body.application_workflow);
    if workflow_uuid.is_none() && workflow_yaml.is_none() {
        return Err(ApiError::bad_request("application_uuid or application_workflow is required"));
    }
    let max_steps = inputs
        .remove("max_steps")
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(200);

    Ok(WorkflowRunRequest {
        workflow_uuid,
        workflow_yaml,
        query: Some(body.query),
        inputs,
        max_steps,
    })
}

fn normalized_optional_string(value: Option<String>) -> Option<String> {
    value.map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

fn workflow_streaming_response(
    state: AppState,
    tenant: TenantScope,
    mut request: WorkflowRunRequest,
    conversation_id: Option<String>,
) -> Response {
    let (tx, rx) = mpsc::unbounded_channel::<Event>();
    tokio::spawn(async move {
        let mut context = WorkflowSseContext::new(conversation_id);
        let database = match workflow_database() {
            Ok(database) => database,
            Err(error) => {
                context.send_error(
                    &tx,
                    error.status.as_u16().into(),
                    "workflow_request_error",
                    error.to_string(),
                );
                return;
            }
        };
        if let Err(error) =
            hydrate_workflow_yaml_from_uuid(database, tenant.clone(), &mut request).await
        {
            context.send_error(
                &tx,
                error.status.as_u16().into(),
                "workflow_request_error",
                error.to_string(),
            );
            return;
        }

        let runtime = match workflow_runtime(&state, tenant, &request) {
            Ok(runtime) => runtime,
            Err(error) => {
                context.send_error(
                    &tx,
                    error.status.as_u16().into(),
                    "workflow_runtime_error",
                    error.to_string(),
                );
                return;
            }
        };
        strip_workflow_runtime_inputs(&mut request);
        let result =
            run_workflow_with_events(runtime, request, |event| context.send_event(&tx, event))
                .await;
        if let Err(message) = result {
            context.send_error(
                &tx,
                u32::from(StatusCode::BAD_REQUEST.as_u16()),
                "workflow_run_error",
                message,
            );
        }
    });

    let sse_stream = UnboundedReceiverStream::new(rx).map(|event| Ok::<Event, Infallible>(event));
    Sse::new(sse_stream).keep_alive(KeepAlive::new()).into_response()
}

struct WorkflowSseContext {
    conversation_id: String,
    run_id: Option<String>,
    created_at: u64,
    streamed_text: bool,
    streamed_final_response: bool,
    last_streamed_text: Option<String>,
}

impl WorkflowSseContext {
    fn new(conversation_id: Option<String>) -> Self {
        Self {
            conversation_id: conversation_id.unwrap_or_default(),
            run_id: None,
            created_at: now_unix_u64(),
            streamed_text: false,
            streamed_final_response: false,
            last_streamed_text: None,
        }
    }

    fn send_event(&mut self, tx: &mpsc::UnboundedSender<Event>, event: WorkflowRunEvent) {
        match event {
            WorkflowRunEvent::WorkflowStarted { run_id } => {
                self.run_id = Some(run_id.clone());
                send_sse_payload(
                    tx,
                    VibeWindowSseEvent::WorkflowStarted {
                        task_id: run_id.clone(),
                        workflow_run_id: run_id.clone(),
                        data: workflow_started_data(&run_id, self.created_at),
                    },
                );
            }
            WorkflowRunEvent::NodeStarted(event) => {
                let Some(run_id) = self.run_id.clone() else {
                    return;
                };
                send_sse_payload(
                    tx,
                    VibeWindowSseEvent::NodeStarted {
                        task_id: run_id.clone(),
                        workflow_run_id: run_id,
                        data: node_started_data_from_event(&event, self.created_at),
                    },
                );
            }
            WorkflowRunEvent::NodeFinished(event) => {
                let Some(run_id) = self.run_id.clone() else {
                    return;
                };
                send_sse_payload(
                    tx,
                    VibeWindowSseEvent::NodeFinished {
                        task_id: run_id.clone(),
                        workflow_run_id: run_id,
                        data: node_finished_data(&event.node, event.index, self.created_at),
                    },
                );
            }
            WorkflowRunEvent::NodeDelta(event) => {
                let Some(run_id) = self.run_id.clone() else {
                    return;
                };
                if !workflow_delta_should_stream_text(&event, self.streamed_text) {
                    return;
                }
                self.streamed_text = true;
                self.streamed_final_response |=
                    event.node_type == "llm" && event.node_id == "final_response";
                self.last_streamed_text = Some(event.text.clone());
                send_sse_payload(
                    tx,
                    VibeWindowSseEvent::TextChunk {
                        task_id: run_id.clone(),
                        workflow_run_id: run_id,
                        data: text_chunk_data(&event),
                    },
                );
            }
            WorkflowRunEvent::WorkflowFinished(response) => self.send_finished(tx, response),
        }
    }

    fn send_finished(&mut self, tx: &mpsc::UnboundedSender<Event>, response: WorkflowRunResponse) {
        let task_id = response.run_id.clone();
        self.run_id = Some(task_id.clone());
        let message_id = response.run_id.clone();
        let answer = truncate_sse_node_output(&workflow_answer(&response)).0;
        let already_streamed_answer = self.last_streamed_text.as_deref() == Some(answer.as_str());
        if !answer.is_empty() && !self.streamed_final_response && !already_streamed_answer {
            send_sse_payload(
                tx,
                VibeWindowSseEvent::Message {
                    id: message_id.clone(),
                    task_id: task_id.clone(),
                    message_id: message_id.clone(),
                    conversation_id: self.conversation_id.clone(),
                    answer,
                    created_at: i64::try_from(self.created_at).unwrap_or(i64::MAX),
                },
            );
        }

        send_sse_payload(
            tx,
            VibeWindowSseEvent::WorkflowFinished {
                task_id: task_id.clone(),
                workflow_run_id: task_id.clone(),
                data: workflow_finished_data(&response, self.created_at),
            },
        );
        send_sse_payload(
            tx,
            VibeWindowSseEvent::MessageEnd {
                id: message_id.clone(),
                task_id,
                message_id,
                conversation_id: self.conversation_id.clone(),
                metadata: HashMap::new(),
            },
        );
    }

    fn send_error(
        &self,
        tx: &mpsc::UnboundedSender<Event>,
        status: u32,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        send_sse_payload(
            tx,
            VibeWindowSseEvent::Error {
                task_id: self.run_id.clone().unwrap_or_else(|| "workflow".to_string()),
                status,
                code: code.into(),
                message: message.into(),
            },
        );
    }
}

fn send_sse_payload(tx: &mpsc::UnboundedSender<Event>, event: VibeWindowSseEvent) {
    let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
    let _ = tx.send(Event::default().data(data));
}

fn workflow_response_to_chat_response(
    response: &WorkflowRunResponse,
    conversation_id: Option<String>,
) -> VibeWindowChatResponse {
    let created_at = now_unix_i64();
    let answer = truncate_sse_node_output(&workflow_answer(response)).0;
    VibeWindowChatResponse {
        event: "message".to_string(),
        task_id: response.run_id.clone(),
        id: response.run_id.clone(),
        message_id: response.run_id.clone(),
        conversation_id: conversation_id.unwrap_or_default(),
        mode: "advanced-chat".to_string(),
        answer,
        metadata: empty_metadata(),
        created_at,
    }
}

fn workflow_response_to_sse_response(
    response: WorkflowRunResponse,
    conversation_id: Option<String>,
) -> Response {
    let events = workflow_response_to_sse_events(response, conversation_id);
    let sse_stream = stream::iter(events.into_iter().map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
        Ok::<Event, std::convert::Infallible>(Event::default().data(data))
    }));
    Sse::new(sse_stream).into_response()
}

fn workflow_response_to_sse_events(
    response: WorkflowRunResponse,
    conversation_id: Option<String>,
) -> Vec<VibeWindowSseEvent> {
    let created_at = now_unix_u64();
    let task_id = response.run_id.clone();
    let workflow_run_id = response.run_id.clone();
    let message_id = response.run_id.clone();
    let conversation_id = conversation_id.unwrap_or_default();
    let mut events = vec![VibeWindowSseEvent::WorkflowStarted {
        task_id: task_id.clone(),
        workflow_run_id: workflow_run_id.clone(),
        data: workflow_started_data(&workflow_run_id, created_at),
    }];

    for (index, node) in response.nodes.iter().enumerate() {
        let node_index = u32::try_from(index + 1).unwrap_or(u32::MAX);
        events.push(VibeWindowSseEvent::NodeStarted {
            task_id: task_id.clone(),
            workflow_run_id: workflow_run_id.clone(),
            data: node_started_data(node, node_index, created_at),
        });
        events.push(VibeWindowSseEvent::NodeFinished {
            task_id: task_id.clone(),
            workflow_run_id: workflow_run_id.clone(),
            data: node_finished_data(node, node_index, created_at),
        });
    }

    let answer = truncate_sse_node_output(&workflow_answer(&response)).0;
    if !answer.is_empty() {
        events.push(VibeWindowSseEvent::Message {
            id: message_id.clone(),
            task_id: task_id.clone(),
            message_id: message_id.clone(),
            conversation_id: conversation_id.clone(),
            answer,
            created_at: i64::try_from(created_at).unwrap_or(i64::MAX),
        });
    }

    events.push(VibeWindowSseEvent::WorkflowFinished {
        task_id: task_id.clone(),
        workflow_run_id: workflow_run_id.clone(),
        data: workflow_finished_data(&response, created_at),
    });
    events.push(VibeWindowSseEvent::MessageEnd {
        id: message_id.clone(),
        task_id: task_id.clone(),
        message_id,
        conversation_id,
        metadata: HashMap::new(),
    });

    events
}

fn workflow_started_data(run_id: &str, created_at: u64) -> VibeWindowWorkflowStartedData {
    VibeWindowWorkflowStartedData {
        id: run_id.to_string(),
        workflow_id: "vibewindow-workflow".to_string(),
        sequence_number: Some(1),
        inputs: Value::Object(Default::default()),
        created_at,
    }
}

fn node_started_data_from_event(
    event: &WorkflowNodeStartedEvent,
    created_at: u64,
) -> VibeWindowNodeStartedData {
    VibeWindowNodeStartedData {
        id: format!("{}-started", event.node_id),
        node_id: event.node_id.clone(),
        node_type: event.node_type.clone(),
        title: event.title.clone(),
        index: event.index,
        predecessor_node_id: None,
        inputs: None,
        created_at,
        extras: HashMap::new(),
        parallel_id: None,
        parallel_start_node_id: None,
        parent_parallel_id: None,
        parent_parallel_start_node_id: None,
        iteration_id: None,
        loop_id: None,
        parallel_run_id: None,
        agent_strategy: None,
    }
}

fn node_started_data(
    node: &WorkflowNodeRunDto,
    index: u32,
    created_at: u64,
) -> VibeWindowNodeStartedData {
    VibeWindowNodeStartedData {
        id: format!("{}-started", node.node_id),
        node_id: node.node_id.clone(),
        node_type: node.node_type.clone(),
        title: node.title.clone(),
        index,
        predecessor_node_id: None,
        inputs: Some(json!(node.inputs)),
        created_at,
        extras: HashMap::new(),
        parallel_id: None,
        parallel_start_node_id: None,
        parent_parallel_id: None,
        parent_parallel_start_node_id: None,
        iteration_id: None,
        loop_id: None,
        parallel_run_id: None,
        agent_strategy: None,
    }
}

fn node_finished_data(
    node: &WorkflowNodeRunDto,
    index: u32,
    created_at: u64,
) -> VibeWindowNodeFinishedData {
    VibeWindowNodeFinishedData {
        id: format!("{}-finished", node.node_id),
        node_id: node.node_id.clone(),
        index,
        predecessor_node_id: None,
        inputs: Some(json!(node.inputs)),
        process_data: None,
        outputs: Some(compact_sse_node_outputs(&node.outputs)),
        status: node_status(&node.status).to_string(),
        error: node.error.clone(),
        elapsed_time: Some(node.elapsed_ms as f64 / 1000.0),
        created_at,
    }
}

fn compact_sse_node_outputs(outputs: &BTreeMap<String, Value>) -> Value {
    let preview_source = workflow_sse_output_preview(outputs);
    let (preview, truncated) = truncate_sse_node_output(&preview_source);
    let mut value = json!({
        "text": preview.clone(),
        "answer": preview.clone(),
        "result": preview,
        "truncated": truncated,
    });
    if let Some(usage) = outputs.get("usage") {
        value["usage"] = usage.clone();
    }
    value
}

fn workflow_sse_output_preview(outputs: &BTreeMap<String, Value>) -> String {
    for key in ["answer", "text", "result", "output", "data"] {
        if let Some(text) = outputs
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return text.to_string();
        }
    }

    serde_json::to_string_pretty(outputs)
        .or_else(|_| serde_json::to_string(outputs))
        .unwrap_or_else(|_| "节点执行完成".to_string())
}

fn truncate_sse_node_output(text: &str) -> (String, bool) {
    let char_count = text.chars().count();
    if char_count <= WORKFLOW_SSE_NODE_OUTPUT_MAX_CHARS {
        return (text.to_string(), false);
    }

    let head_len = WORKFLOW_SSE_NODE_OUTPUT_MAX_CHARS * 2 / 3;
    let tail_len = WORKFLOW_SSE_NODE_OUTPUT_MAX_CHARS.saturating_sub(head_len);
    let head = text.chars().take(head_len).collect::<String>();
    let tail =
        text.chars().rev().take(tail_len).collect::<String>().chars().rev().collect::<String>();
    let omitted = char_count.saturating_sub(head_len + tail_len);
    (format!("{head}\n\n... 已截断 {omitted} 字符，仅保留节点输出预览 ...\n\n{tail}"), true)
}

fn text_chunk_data(event: &WorkflowNodeDeltaEvent) -> VibeWindowTextChunkData {
    VibeWindowTextChunkData {
        text: event.text.clone(),
        from_variable_selector: vec![event.node_id.clone(), "text".to_string()],
    }
}

fn workflow_delta_should_stream_text(event: &WorkflowNodeDeltaEvent, streamed_text: bool) -> bool {
    match event.node_type.as_str() {
        "llm" => event.node_id == "final_response" && !event.replace,
        "answer" | "output" | "end" => !streamed_text,
        _ => false,
    }
}

fn workflow_finished_data(
    response: &WorkflowRunResponse,
    created_at: u64,
) -> VibeWindowWorkflowFinishedData {
    VibeWindowWorkflowFinishedData {
        id: response.run_id.clone(),
        workflow_id: "vibewindow-workflow".to_string(),
        status: workflow_status(&response.status).to_string(),
        outputs: Some(compact_sse_node_outputs(&response.outputs)),
        error: response.error.clone(),
        elapsed_time: None,
        total_tokens: None,
        total_steps: u32::try_from(response.nodes.len()).unwrap_or(u32::MAX),
        created_at,
        finished_at: now_unix_u64(),
        pause: response.pause.clone(),
    }
}

fn workflow_answer(response: &WorkflowRunResponse) -> String {
    response
        .answer
        .clone()
        .or_else(|| string_output(&response.outputs, "answer"))
        .or_else(|| string_output(&response.outputs, "text"))
        .or_else(|| string_output(&response.outputs, "result"))
        .unwrap_or_default()
}

fn string_output(outputs: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    outputs.get(key).and_then(Value::as_str).map(str::to_string)
}

fn empty_metadata() -> VibeWindowResponseMetadata {
    VibeWindowResponseMetadata {
        usage: VibeWindowUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
        retriever_resources: None,
    }
}

fn workflow_status(status: &WorkflowRunStatus) -> &'static str {
    match status {
        WorkflowRunStatus::Running => "running",
        WorkflowRunStatus::Paused => "running",
        WorkflowRunStatus::Succeeded => "succeeded",
        WorkflowRunStatus::Failed => "failed",
    }
}

fn node_status(status: &WorkflowNodeRunStatus) -> &'static str {
    match status {
        WorkflowNodeRunStatus::Paused => "running",
        WorkflowNodeRunStatus::Succeeded => "succeeded",
        WorkflowNodeRunStatus::Failed => "failed",
    }
}

fn now_unix_u64() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|duration| duration.as_secs()).unwrap_or(0)
}

fn now_unix_i64() -> i64 {
    i64::try_from(now_unix_u64()).unwrap_or(i64::MAX)
}

#[cfg(test)]
#[path = "workflow_tests.rs"]
mod workflow_tests;
