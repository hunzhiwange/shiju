//! 通过网关聊天流执行本地代理任务，并桥接工作树池、流式事件与执行结果。

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::thread;

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Map, Value, json};
use vw_api_types::chat::GatewayChatStreamRequest;
use vw_api_types::common::TimestampMs;
use vw_api_types::task::{
    TaskPoolWorktreeAcquireResponse, TaskPoolWorktreeRecycleResponse,
    TaskPoolWorktreeReleaseResponse,
};

use crate::agent_scheduler::domain::{RunId, TaskId};
use crate::agent_scheduler::runner::local::{
    LocalAgentExecutionOutput, LocalAgentExecutionRequest, LocalAgentExecutor, LocalAgentWorkspace,
    LocalAgentWorkspaceRecycleRequest, LocalAgentWorkspaceReleaseRequest,
    LocalAgentWorkspaceRequest,
};
use crate::agent_scheduler::store::AgentSchedulerStore;
use crate::agent_scheduler::store::logs::AppendAgentTaskLogRequest;
use crate::app::agent::gateway::api::handlers::session::stream::GatewayChatStreamCollected;
use crate::app::agent::project;

const GATEWAY_STREAM_DELTA_LOG_LIMIT: usize = 280;
const GATEWAY_STREAM_FIELD_LOG_LIMIT: usize = 160;

type GatewayFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;
type GatewayPayloadObserver<'a> = Box<dyn FnMut(&Value) + Send + 'a>;

trait GatewayWorktreePoolClient: Send + Sync {
    fn acquire_and_prepare<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_types::id::TaskId,
    ) -> GatewayFuture<'a, TaskPoolWorktreeAcquireResponse>;

    fn release_task_slot<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_types::id::TaskId,
    ) -> GatewayFuture<'a, TaskPoolWorktreeReleaseResponse>;

    fn recycle_task_slot<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_types::id::TaskId,
        taint_reason: Option<String>,
    ) -> GatewayFuture<'a, TaskPoolWorktreeRecycleResponse>;
}

struct RealGatewayWorktreePoolClient;

#[cfg(not(test))]
impl GatewayWorktreePoolClient for RealGatewayWorktreePoolClient {
    fn acquire_and_prepare<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_types::id::TaskId,
    ) -> GatewayFuture<'a, TaskPoolWorktreeAcquireResponse> {
        Box::pin(async move {
            crate::app::agent::gateway::api::handlers::task_pool::task_worktree_pool_service()
                .acquire_and_prepare(project, task_id)
                .await
        })
    }

    fn release_task_slot<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_types::id::TaskId,
    ) -> GatewayFuture<'a, TaskPoolWorktreeReleaseResponse> {
        Box::pin(async move {
            crate::app::agent::gateway::api::handlers::task_pool::task_worktree_pool_service()
                .release_task_slot(project, task_id)
                .await
        })
    }

    fn recycle_task_slot<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_types::id::TaskId,
        taint_reason: Option<String>,
    ) -> GatewayFuture<'a, TaskPoolWorktreeRecycleResponse> {
        Box::pin(async move {
            crate::app::agent::gateway::api::handlers::task_pool::task_worktree_pool_service()
                .recycle_task_slot(project, task_id, taint_reason)
                .await
        })
    }
}

#[cfg(test)]
impl GatewayWorktreePoolClient for RealGatewayWorktreePoolClient {
    fn acquire_and_prepare<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_types::id::TaskId,
    ) -> GatewayFuture<'a, TaskPoolWorktreeAcquireResponse> {
        Box::pin(async move {
            crate::app::agent::gateway::api::handlers::task_pool::task_worktree_pool_service()
                .acquire_and_prepare(project, task_id)
                .await
        })
    }

    fn release_task_slot<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_types::id::TaskId,
    ) -> GatewayFuture<'a, TaskPoolWorktreeReleaseResponse> {
        Box::pin(async move {
            crate::app::agent::gateway::api::handlers::task_pool::task_worktree_pool_service()
                .release_task_slot(project, task_id)
                .await
        })
    }

    fn recycle_task_slot<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_types::id::TaskId,
        taint_reason: Option<String>,
    ) -> GatewayFuture<'a, TaskPoolWorktreeRecycleResponse> {
        Box::pin(async move {
            crate::app::agent::gateway::api::handlers::task_pool::task_worktree_pool_service()
                .recycle_task_slot(project, task_id, taint_reason)
                .await
        })
    }
}

trait GatewayChatStreamClient: std::fmt::Debug + Send + Sync {
    fn collect_with_observer<'a>(
        &'a self,
        workspace_path: String,
        body: GatewayChatStreamRequest,
        observe_payload: GatewayPayloadObserver<'a>,
    ) -> GatewayFuture<'a, GatewayChatStreamCollected>;
}

#[derive(Debug)]
struct RealGatewayChatStreamClient;

#[cfg(not(test))]
impl GatewayChatStreamClient for RealGatewayChatStreamClient {
    fn collect_with_observer<'a>(
        &'a self,
        workspace_path: String,
        body: GatewayChatStreamRequest,
        observe_payload: GatewayPayloadObserver<'a>,
    ) -> GatewayFuture<'a, GatewayChatStreamCollected> {
        Box::pin(async move {
            let mut observe_payload = observe_payload;
            crate::app::agent::gateway::api::handlers::session::stream::collect_gateway_chat_stream_with_observer(
                workspace_path,
                body,
                move |payload| observe_payload(payload),
            )
            .await
            .map_err(|error| anyhow!(error.to_string()))
        })
    }
}

#[cfg(test)]
impl GatewayChatStreamClient for RealGatewayChatStreamClient {
    fn collect_with_observer<'a>(
        &'a self,
        _workspace_path: String,
        _body: GatewayChatStreamRequest,
        _observe_payload: GatewayPayloadObserver<'a>,
    ) -> GatewayFuture<'a, GatewayChatStreamCollected> {
        Box::pin(async { bail!("real gateway chat stream client is disabled in tests") })
    }
}

#[derive(Debug, Clone)]
/// 使用网关聊天流和工作树池实现本地执行器契约。
pub(crate) struct GatewayLocalAgentExecutor {
    store: AgentSchedulerStore,
    chat_stream: Arc<dyn GatewayChatStreamClient>,
}

impl GatewayLocalAgentExecutor {
    pub(crate) fn new(store: AgentSchedulerStore) -> Self {
        Self { store, chat_stream: Arc::new(RealGatewayChatStreamClient) }
    }

    #[cfg(test)]
    fn new_with_chat_stream(
        store: AgentSchedulerStore,
        chat_stream: Arc<dyn GatewayChatStreamClient>,
    ) -> Self {
        Self { store, chat_stream }
    }
}

impl Default for GatewayLocalAgentExecutor {
    fn default() -> Self {
        Self::new(AgentSchedulerStore::new())
    }
}

impl LocalAgentExecutor for GatewayLocalAgentExecutor {
    fn lease_workspace(&self, request: LocalAgentWorkspaceRequest) -> Result<LocalAgentWorkspace> {
        lease_gateway_local_agent_workspace(request)
    }

    fn execute(&self, request: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput> {
        let raw_output = collect_gateway_local_agent_output(
            &self.store,
            request,
            Arc::clone(&self.chat_stream),
        )?;
        Ok(LocalAgentExecutionOutput { raw_output })
    }

    fn release_workspace(&self, request: LocalAgentWorkspaceReleaseRequest) -> Result<()> {
        release_gateway_local_agent_workspace(request)
    }

    fn recycle_workspace(&self, request: LocalAgentWorkspaceRecycleRequest) -> Result<()> {
        recycle_gateway_local_agent_workspace(request)
    }
}

fn lease_gateway_local_agent_workspace(
    request: LocalAgentWorkspaceRequest,
) -> Result<LocalAgentWorkspace> {
    run_gateway_future("create agent scheduler gateway worktree lease runtime", async move {
        let project = project_for_local_workspace(&request.project_path).await?;
        lease_gateway_local_agent_workspace_for_project(
            request,
            &project,
            &RealGatewayWorktreePoolClient,
        )
        .await
    })
}

async fn lease_gateway_local_agent_workspace_for_project(
    request: LocalAgentWorkspaceRequest,
    project: &project::Info,
    pool: &impl GatewayWorktreePoolClient,
) -> Result<LocalAgentWorkspace> {
    let task_id = vw_api_types::id::TaskId(request.task_id.as_str().to_string());
    let response = pool
        .acquire_and_prepare(project, &task_id)
        .await
        .context("acquire task worktree for local agent execution")?;
    if !response.leased {
        bail!("task worktree pool did not lease a workspace for task {}", request.task_id.as_str());
    }
    let slot = response.slot.context("task worktree pool leased without slot metadata")?;
    Ok(LocalAgentWorkspace { slot_id: slot.id, path: slot.path, branch: slot.branch })
}

fn release_gateway_local_agent_workspace(request: LocalAgentWorkspaceReleaseRequest) -> Result<()> {
    run_gateway_future("create agent scheduler gateway worktree release runtime", async move {
        let project = project_for_local_workspace(&request.project_path).await?;
        release_gateway_local_agent_workspace_for_project(
            request,
            &project,
            &RealGatewayWorktreePoolClient,
        )
        .await
    })
}

async fn release_gateway_local_agent_workspace_for_project(
    request: LocalAgentWorkspaceReleaseRequest,
    project: &project::Info,
    pool: &impl GatewayWorktreePoolClient,
) -> Result<()> {
    let task_id = vw_api_types::id::TaskId(request.task_id.as_str().to_string());
    let response = pool
        .release_task_slot(project, &task_id)
        .await
        .context("release successful local agent worktree")?;
    if !response.released {
        bail!("task worktree pool did not release workspace for task {}", task_id.as_ref());
    }
    Ok(())
}

fn recycle_gateway_local_agent_workspace(request: LocalAgentWorkspaceRecycleRequest) -> Result<()> {
    run_gateway_future("create agent scheduler gateway worktree recycle runtime", async move {
        let project = project_for_local_workspace(&request.project_path).await?;
        recycle_gateway_local_agent_workspace_for_project(
            request,
            &project,
            &RealGatewayWorktreePoolClient,
        )
        .await
    })
}

async fn recycle_gateway_local_agent_workspace_for_project(
    request: LocalAgentWorkspaceRecycleRequest,
    project: &project::Info,
    pool: &impl GatewayWorktreePoolClient,
) -> Result<()> {
    let task_id = vw_api_types::id::TaskId(request.task_id.as_str().to_string());
    let response = pool
        .recycle_task_slot(project, &task_id, Some(request.taint_reason))
        .await
        .context("recycle failed local agent worktree")?;
    if !response.recycled {
        bail!("task worktree pool did not recycle workspace for task {}", task_id.as_ref());
    }
    Ok(())
}

async fn project_for_local_workspace(project_path: &Path) -> Result<project::Info> {
    let target = normalize_project_path(project_path);
    let tenant = project::default_tenant_scope();
    let projects = project::list(&tenant).await.context("list registered projects")?;
    projects
        .into_iter()
        .find(|info| {
            normalize_project_path(Path::new(&info.worktree)) == target
                || info
                    .sandboxes
                    .iter()
                    .any(|sandbox| normalize_project_path(Path::new(sandbox)) == target)
        })
        .with_context(|| {
            format!(
                "registered git project not found for local agent workspace: {}",
                project_path.display()
            )
        })
}

fn normalize_project_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    let normalized = value.replace('\\', "/");
    normalized
        .trim_end_matches('/')
        .trim_start_matches('/')
        .strip_prefix("./")
        .unwrap_or_else(|| normalized.trim_end_matches('/').trim_start_matches('/'))
        .to_string()
}

fn collect_gateway_local_agent_output(
    store: &AgentSchedulerStore,
    request: LocalAgentExecutionRequest,
    chat_stream: Arc<dyn GatewayChatStreamClient>,
) -> Result<String> {
    let workspace_path = request.workspace.path.clone();
    let project_path = request.project_path.clone();
    let task_id = request.task_id.clone();
    let run_id = request.run_id.clone();
    let body = gateway_local_agent_stream_request(&request);
    let stream_log_store = store.clone();
    let collected =
        run_gateway_future("create agent scheduler gateway local executor runtime", async move {
            chat_stream
                .collect_with_observer(
                    workspace_path,
                    body,
                    Box::new(move |payload| {
                        if let Err(error) = append_gateway_stream_payload_task_log(
                            &stream_log_store,
                            &project_path,
                            &task_id,
                            &run_id,
                            current_timestamp_ms(),
                            payload,
                        ) {
                            tracing::warn!(
                                error = %error,
                                task_id = task_id.as_str(),
                                run_id = run_id.as_str(),
                                "failed to append agent scheduler gateway stream log"
                            );
                        }
                    }),
                )
                .await
        })?;
    Ok(collected.output)
}

/// 将网关流式载荷裁剪为安全长度后追加到任务日志。
pub(crate) fn append_gateway_stream_payload_task_log(
    store: &AgentSchedulerStore,
    project_path: &Path,
    task_id: &TaskId,
    run_id: &RunId,
    timestamp_ms: TimestampMs,
    payload: &Value,
) -> Result<()> {
    for message in gateway_stream_payload_log_messages(payload) {
        store.append_task_log(
            project_path,
            AppendAgentTaskLogRequest {
                task_id: task_id.clone(),
                run_id: Some(run_id.clone()),
                timestamp_ms,
                from_status: None,
                to_status: None,
                from_status_key: None,
                to_status_key: None,
                message,
            },
        )?;
    }
    Ok(())
}

fn gateway_stream_payload_log_messages(payload: &Value) -> Vec<String> {
    match payload.get("type").and_then(Value::as_str) {
        Some("chat.delta") => payload
            .get("delta")
            .and_then(Value::as_str)
            .map(gateway_delta_log_messages)
            .unwrap_or_default(),
        Some("chat.step_start") => vec![gateway_stream_step_message(
            payload,
            "started",
            payload.get("model").and_then(Value::as_str).and_then(non_empty_str).map(|model| {
                format!(" model={}", truncate_log_text(model, GATEWAY_STREAM_FIELD_LOG_LIMIT))
            }),
        )],
        Some("chat.step_finish") => vec![gateway_stream_step_message(
            payload,
            "finished",
            payload.get("finish_reason").and_then(Value::as_str).and_then(non_empty_str).map(
                |reason| {
                    format!(
                        " finish_reason={}",
                        truncate_log_text(reason, GATEWAY_STREAM_FIELD_LOG_LIMIT)
                    )
                },
            ),
        )],
        Some("chat.post_tool_round") => {
            vec![gateway_stream_step_message(payload, "completed tool round", None)]
        }
        Some("chat.error") => payload
            .get("error")
            .and_then(Value::as_str)
            .and_then(non_empty_str)
            .map(|error| {
                format!(
                    "gateway stream error: {}",
                    truncate_log_text(error, GATEWAY_STREAM_DELTA_LOG_LIMIT)
                )
            })
            .into_iter()
            .collect(),
        Some("chat.done") => vec!["gateway stream completed".to_string()],
        _ => Vec::new(),
    }
}

fn gateway_delta_log_messages(delta: &str) -> Vec<String> {
    delta
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(3)
        .map(|line| {
            format!(
                "gateway stream output: {}",
                truncate_log_text(line, GATEWAY_STREAM_DELTA_LOG_LIMIT)
            )
        })
        .collect()
}

fn gateway_stream_step_message(payload: &Value, action: &str, suffix: Option<String>) -> String {
    let step_index =
        i64_field(payload, "step_index").map_or_else(|| "?".to_string(), |value| value.to_string());
    format!("gateway stream step {step_index} {action}{}", suffix.unwrap_or_default())
}

fn truncate_log_text(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let mut truncated: String = trimmed.chars().take(keep).collect();
    truncated.push_str("...");
    truncated
}

/// 从执行请求构造网关流式聊天请求，并保留模型与代理路由选择。
pub(crate) fn gateway_local_agent_stream_request(
    request: &LocalAgentExecutionRequest,
) -> GatewayChatStreamRequest {
    let mut options = gateway_local_agent_options(request.workspace.path.clone());
    if let Some(agent) = request.agent.as_deref() {
        options.insert("agent".to_string(), json!(agent));
    }
    if let Some(acp_agent) = request.acp_agent.as_deref() {
        options.insert("acp_test".to_string(), json!(true));
        options.insert("acp_agent".to_string(), json!(acp_agent));
        options.insert("acp_force_new_session".to_string(), json!(true));
        options.insert("acp_close_client_after_request".to_string(), json!(true));
        options.insert("acp_history_strategy".to_string(), json!("discard"));
        options.insert("acp_history_recent_count".to_string(), json!(1));
        options.insert("acp_permission_mode".to_string(), json!("approve-all"));
    }

    GatewayChatStreamRequest {
        session_id: Some(
            format!("agent-scheduler-{}-{}", request.task_id.as_str(), request.run_id.as_str())
                .into(),
        ),
        messages: vec![json!({ "role": "user", "content": request.prompt.clone() })],
        system: None,
        model: request.model.clone(),
        reasoning_effort: request.reasoning_effort,
        agent: request.agent.clone(),
        allowed_tools: None,
        acp_agent: request.acp_agent.clone(),
        acp_allowed_tools: None,
        options: Some(Value::Object(options)),
    }
}

fn gateway_local_agent_options(cwd: String) -> Map<String, Value> {
    let mut options = Map::new();
    options.insert("cwd".to_string(), json!(cwd));
    options.insert("full_access".to_string(), json!(true));
    options
}

fn run_gateway_future<T, F>(context: &'static str, future: F) -> Result<T>
where
    T: Send + 'static,
    F: Future<Output = Result<T>> + Send + 'static,
{
    if tokio::runtime::Handle::try_current().is_ok() {
        return thread::spawn(move || block_on_gateway_future(context, future))
            .join()
            .map_err(|_| anyhow!("{context} thread panicked"))?;
    }
    block_on_gateway_future(context, future)
}

fn block_on_gateway_future<T, F>(context: &'static str, future: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    let runtime =
        tokio::runtime::Builder::new_current_thread().enable_all().build().context(context)?;
    runtime.block_on(future)
}

fn current_timestamp_ms() -> TimestampMs {
    TimestampMs(chrono::Utc::now().timestamp_millis().max(0))
}

fn i64_field(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn non_empty_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

#[cfg(test)]
#[path = "gateway_executor_tests.rs"]
mod gateway_executor_tests;
