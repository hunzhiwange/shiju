//! 通过网关聊天流执行本地代理任务，并桥接工作树池、流式事件与执行结果。

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::thread;

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;
use vw_api_chat::chat::GatewayChatStreamRequest;
use vw_api_transport::common::TimestampMs;
use vw_api_task::{TaskPoolWorktreeRecycleResponse, TaskPoolWorktreeReleaseResponse};

use crate::runner::local::{
    LocalAgentExecutionOutput, LocalAgentExecutionRequest, LocalAgentExecutor, LocalAgentWorkspace,
    LocalAgentWorkspaceRecycleRequest, LocalAgentWorkspaceReleaseRequest,
    LocalAgentWorkspaceRequest,
};
use crate::store::AgentSchedulerStore;
use session_port::chat_stream::GatewayChatStreamCollected;
use agent_project as project;
pub use task_worktree_pool::TaskWorktreePoolService;

pub type GatewayFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;
pub type GatewayPayloadObserver<'a> = Box<dyn FnMut(&Value) + Send + 'a>;

pub trait GatewayWorktreePoolClient: Send + Sync {
    fn release_task_slot<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_transport::TaskId,
    ) -> GatewayFuture<'a, TaskPoolWorktreeReleaseResponse>;

    fn recycle_task_slot<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_transport::TaskId,
        taint_reason: Option<String>,
    ) -> GatewayFuture<'a, TaskPoolWorktreeRecycleResponse>;
}

struct RealGatewayWorktreePoolClient<H>(std::marker::PhantomData<H>);
impl<H: GatewayExecutorHost> GatewayWorktreePoolClient for RealGatewayWorktreePoolClient<H> {
    fn release_task_slot<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_transport::TaskId,
    ) -> GatewayFuture<'a, TaskPoolWorktreeReleaseResponse> {
        Box::pin(async move {
            H::worktree_pool()
                .release_task_slot(project, task_id)
                .await
        })
    }

    fn recycle_task_slot<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_transport::TaskId,
        taint_reason: Option<String>,
    ) -> GatewayFuture<'a, TaskPoolWorktreeRecycleResponse> {
        Box::pin(async move {
            H::worktree_pool()
                .recycle_task_slot(project, task_id, taint_reason)
                .await
        })
    }
}

pub trait GatewayChatStreamClient: std::fmt::Debug + Send + Sync {
    fn collect_with_observer<'a>(
        &'a self,
        workspace_path: String,
        body: GatewayChatStreamRequest,
        cancellation: tokio_util::sync::CancellationToken,
        observe_payload: GatewayPayloadObserver<'a>,
    ) -> GatewayFuture<'a, GatewayChatStreamCollected>;
}

#[derive(Debug, Clone)]
/// 使用网关聊天流和工作树池实现本地执行器契约。
pub struct GatewayLocalAgentExecutor<H: GatewayExecutorHost> {
    host: std::marker::PhantomData<H>,
    store: AgentSchedulerStore,
    chat_stream: Arc<dyn GatewayChatStreamClient>,
}

impl<H: GatewayExecutorHost> GatewayLocalAgentExecutor<H> {
    pub fn new(store: AgentSchedulerStore) -> Self {
        Self { store, chat_stream: H::chat_stream(), host: std::marker::PhantomData }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn new_with_chat_stream(
        store: AgentSchedulerStore,
        chat_stream: Arc<dyn GatewayChatStreamClient>,
    ) -> Self {
        Self { store, chat_stream, host: std::marker::PhantomData }
    }
}

impl<H: GatewayExecutorHost> Default for GatewayLocalAgentExecutor<H> {
    fn default() -> Self {
        Self::new(AgentSchedulerStore::new())
    }
}

impl<H: GatewayExecutorHost> LocalAgentExecutor for GatewayLocalAgentExecutor<H> {
    fn lease_workspace(&self, request: LocalAgentWorkspaceRequest) -> Result<LocalAgentWorkspace> {
        lease_gateway_local_agent_workspace::<H>(request)
    }

    fn execute(&self, request: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput> {
        let raw_output = collect_gateway_local_agent_output::<H>(
            &self.store,
            request,
            Arc::clone(&self.chat_stream),
        )?;
        Ok(LocalAgentExecutionOutput { raw_output })
    }

    fn release_workspace(&self, request: LocalAgentWorkspaceReleaseRequest) -> Result<()> {
        release_gateway_local_agent_workspace::<H>(request)
    }

    fn recycle_workspace(&self, request: LocalAgentWorkspaceRecycleRequest) -> Result<()> {
        recycle_gateway_local_agent_workspace::<H>(request)
    }
}

fn lease_gateway_local_agent_workspace<H: GatewayExecutorHost>(
    request: LocalAgentWorkspaceRequest,
) -> Result<LocalAgentWorkspace> {
    run_gateway_future("create agent scheduler gateway worktree lease runtime", async move {
        let project = project_for_local_workspace(&request.project_path).await?;
        let task_id = vw_api_transport::TaskId(request.task_id.as_str().to_string());
        let run_id = request.run_id.as_str().to_string();
        let agent = request.agent.clone();
        let workspace = lease_gateway_local_agent_workspace_for_project(
            request,
            &project,
            &RealGatewayWorktreePoolClient::<H>(std::marker::PhantomData),
        )
        .await?;
        H::worktree_pool()
            .bind_run(&project, &task_id, &run_id, agent)
            .await?;
        Ok(workspace)
    })
}

pub async fn lease_gateway_local_agent_workspace_for_project(
    request: LocalAgentWorkspaceRequest,
    project: &project::Info,
    _pool: &impl GatewayWorktreePoolClient,
) -> Result<LocalAgentWorkspace> {
    let path = request.project_path.canonicalize().context("resolve execution directory")?;
    if !path.is_dir() {
        bail!("execution directory is not a directory: {}", path.display());
    }
    let authorized =
        std::iter::once(&project.worktree).chain(project.sandboxes.iter()).any(|registered| {
            Path::new(registered).canonicalize().is_ok_and(|registered| registered == path)
        });
    if !authorized {
        bail!("execution directory is outside registered project: {}", path.display());
    }
    Ok(LocalAgentWorkspace {
        slot_id: None,
        path: path.to_string_lossy().into_owned(),
        branch: None,
    })
}

fn release_gateway_local_agent_workspace<H: GatewayExecutorHost>(request: LocalAgentWorkspaceReleaseRequest) -> Result<()> {
    run_gateway_future("create agent scheduler gateway worktree release runtime", async move {
        let project = project_for_local_workspace(&request.project_path).await?;
        H::worktree_pool()
            .finish_run(
                &project,
                &vw_api_transport::TaskId(request.task_id.as_str().to_string()),
                request.run_id.as_str(),
                None,
            )
            .await?;
        release_gateway_local_agent_workspace_for_project(
            request,
            &project,
            &RealGatewayWorktreePoolClient::<H>(std::marker::PhantomData),
        )
        .await
    })
}

pub async fn release_gateway_local_agent_workspace_for_project(
    request: LocalAgentWorkspaceReleaseRequest,
    project: &project::Info,
    pool: &impl GatewayWorktreePoolClient,
) -> Result<()> {
    if request.workspace.slot_id.is_none() {
        return Ok(());
    }
    let task_id = vw_api_transport::TaskId(request.task_id.as_str().to_string());
    let response = pool
        .release_task_slot(project, &task_id)
        .await
        .context("release successful local agent worktree")?;
    if !response.released {
        bail!("task worktree pool did not release workspace for task {}", task_id.as_ref());
    }
    Ok(())
}

fn recycle_gateway_local_agent_workspace<H: GatewayExecutorHost>(request: LocalAgentWorkspaceRecycleRequest) -> Result<()> {
    run_gateway_future("create agent scheduler gateway worktree recycle runtime", async move {
        let project = project_for_local_workspace(&request.project_path).await?;
        H::worktree_pool()
            .finish_run(
                &project,
                &vw_api_transport::TaskId(request.task_id.as_str().to_string()),
                request.run_id.as_str(),
                Some(request.taint_reason.clone()),
            )
            .await?;
        recycle_gateway_local_agent_workspace_for_project(
            request,
            &project,
            &RealGatewayWorktreePoolClient::<H>(std::marker::PhantomData),
        )
        .await
    })
}

pub async fn recycle_gateway_local_agent_workspace_for_project(
    request: LocalAgentWorkspaceRecycleRequest,
    project: &project::Info,
    pool: &impl GatewayWorktreePoolClient,
) -> Result<()> {
    if request.workspace.slot_id.is_none() {
        return Ok(());
    }
    let task_id = vw_api_transport::TaskId(request.task_id.as_str().to_string());
    let response = pool
        .recycle_task_slot(project, &task_id, Some(request.taint_reason))
        .await
        .context("recycle failed local agent worktree")?;
    if !response.recycled {
        bail!("task worktree pool did not recycle workspace for task {}", task_id.as_ref());
    }
    Ok(())
}

pub async fn project_for_local_workspace(project_path: &Path) -> Result<project::Info> {
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

pub fn normalize_project_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    let normalized = value.replace('\\', "/");
    normalized
        .trim_end_matches('/')
        .trim_start_matches('/')
        .strip_prefix("./")
        .unwrap_or_else(|| normalized.trim_end_matches('/').trim_start_matches('/'))
        .to_string()
}

fn collect_gateway_local_agent_output<H: GatewayExecutorHost>(
    store: &AgentSchedulerStore,
    request: LocalAgentExecutionRequest,
    chat_stream: Arc<dyn GatewayChatStreamClient>,
) -> Result<String> {
    let workspace_path = request.workspace.path.clone();
    let project_path = request.project_path.clone();
    let task_id = request.task_id.clone();
    let run_id = request.run_id.clone();
    let body = gateway_local_agent_stream_request(&request);
    let tool_context = H::tool_context(
        body.session_id.as_ref().expect("scheduler session").as_ref().to_string(),
        workspace_path.clone(),
        crate::runner::worktree_owner::SchedulerWorktreeOwner {
            claims: Default::default(), store: store.clone(), project_path: project_path.clone(),
            task_id: task_id.clone(), run_id: run_id.clone(),
        },
    );
    let worktree_owner = tool_context.run_worktree_owner().expect("run owner").clone();
    let stream_log_store = store.clone();
    let store_for_cancel = store.clone();
    let cancel_project = project_path.clone();
    let cancel_task = task_id.clone();
    let cancel_run = run_id.clone();
    let collected = run_gateway_future(
        "create agent scheduler gateway local executor runtime",
        async move {
            let cancellation = tokio_util::sync::CancellationToken::new();
            let mut collection = Box::pin(tool_context::context::scope_tool_use_context(
                tool_context,
                chat_stream.collect_with_observer(
                    workspace_path,
                    body,
                    cancellation.clone(),
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
                ),
            ));
            let mut poll = tokio::time::interval(std::time::Duration::from_millis(50));
            loop {
                tokio::select! {
                    result = &mut collection => break result,
                    _ = poll.tick() => {
                        // Read failure is fail-closed: stop execution but keep waiting for shutdown.
                        let running = store_for_cancel.load_task(&cancel_project, &cancel_task)
                            .ok().flatten().is_some_and(|task| task.status == crate::state_machine::AgentTaskStatus::Running)
                            && store_for_cancel.load_run(&cancel_project, &cancel_run)
                                .ok().flatten().is_some_and(|run| run.status == crate::domain::AgentRunStatus::Running);
                        if !running { cancellation.cancel(); }
                    }
                }
            }
        },
    );
    if !collected.as_ref().err().is_some_and(|error| {
        error
            .downcast_ref::<crate::runner::local::ExecutionShutdownUnconfirmed>()
            .is_some()
    }) {
        worktree_owner.release_directories()?;
    } else {
        worktree_owner.retain_unconfirmed()?;
    }
    Ok(collected?.output)
}

pub use super::gateway_request::*;

pub fn run_gateway_future<T, F>(context: &'static str, future: F) -> Result<T>
where
    T: Send + 'static,
    F: Future<Output = Result<T>> + Send + 'static,
{
    if tokio::runtime::Handle::try_current().is_ok() {
        return thread::spawn(move || block_on_gateway_future(context, future)).join().map_err(
            |_| {
                anyhow!(crate::runner::local::ExecutionShutdownUnconfirmed(
                    format!("{context} thread panicked")
                ))
            },
        )?;
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


/// 实际网关I/O与工具上下文组合；业务流程由本模块保有。
pub trait GatewayExecutorHost: Clone + std::fmt::Debug + Send + Sync + 'static {
    type ContextHost: tool_context::host::ContextHost;
    fn worktree_pool() -> &'static task_worktree_pool::TaskWorktreePoolService;
    fn chat_stream() -> Arc<dyn GatewayChatStreamClient>;
    fn tool_context(session_id: String, workspace_path: String,
        owner: super::worktree_owner::SchedulerWorktreeOwner)
        -> Arc<tool_context::context::ToolUseContext<Self::ContextHost>>;
}
