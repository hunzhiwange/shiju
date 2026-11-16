//! 真实网关I/O与工具上下文组合，执行编排归task-scheduler。
use std::sync::Arc;
#[cfg(not(test))]
use anyhow::anyhow;
#[cfg(test)]
use anyhow::bail;
use vw_api_chat::chat::GatewayChatStreamRequest;
use session_port::chat_stream::GatewayChatStreamCollected;
use task_scheduler::runner::gateway_executor::{GatewayChatStreamClient, GatewayFuture, GatewayPayloadObserver};
#[cfg(test)]
use task_scheduler::runner::gateway_executor::run_gateway_future;
#[cfg(test)]
use task_scheduler::runner::gateway_request::*;
#[cfg(test)]
use crate::agent_scheduler::store::AgentSchedulerStore;
#[cfg(test)]
use crate::agent_scheduler::runner::local::*;
#[derive(Debug, Clone)]
pub(crate) struct SchedulerGatewayHost;
impl task_scheduler::runner::gateway_executor::GatewayExecutorHost for SchedulerGatewayHost {
    type ContextHost = crate::tools::context::AgentContextHost;
    fn worktree_pool() -> &'static task_scheduler::runner::gateway_executor::TaskWorktreePoolService {
        crate::gateway::api::handlers::task_pool::task_worktree_pool_service()
    }
    fn chat_stream() -> Arc<dyn GatewayChatStreamClient> { Arc::new(RealGatewayChatStreamClient) }
    fn tool_context(session_id: String, workspace_path: String, owner: task_scheduler::runner::worktree_owner::SchedulerWorktreeOwner) -> Arc<crate::tools::ToolUseContext> {
        Arc::new(crate::tools::ToolUseContext::new(session_id, Some(workspace_path))
            .with_run_worktree_owner(crate::tools::RunWorktreeOwner::new(owner)))
    }
}
pub(crate) type GatewayLocalAgentExecutor = task_scheduler::runner::gateway_executor::GatewayLocalAgentExecutor<SchedulerGatewayHost>;
#[derive(Debug)]
struct RealGatewayChatStreamClient;

#[cfg(not(test))]
impl GatewayChatStreamClient for RealGatewayChatStreamClient {
    fn collect_with_observer<'a>(
        &'a self,
        workspace_path: String,
        body: GatewayChatStreamRequest,
        cancellation: tokio_util::sync::CancellationToken,
        observe_payload: GatewayPayloadObserver<'a>,
    ) -> GatewayFuture<'a, GatewayChatStreamCollected> {
        Box::pin(async move {
            let mut observe_payload = observe_payload;
            crate::app::agent::gateway::api::handlers::session::stream::collect_scheduler_gateway_chat_stream(
                workspace_path,
                body,
                cancellation,
                move |payload| observe_payload(payload),
            )
            .await
            .map_err(|error| {
                let message = error.to_string();
                if message.contains("execution shutdown unconfirmed") {
                    anyhow!(crate::agent_scheduler::runner::local::ExecutionShutdownUnconfirmed(message))
                } else { anyhow!(message) }
            })
        })
    }
}

#[cfg(test)]
impl GatewayChatStreamClient for RealGatewayChatStreamClient {
    fn collect_with_observer<'a>(
        &'a self,
        _workspace_path: String,
        _body: GatewayChatStreamRequest,
        _cancellation: tokio_util::sync::CancellationToken,
        _observe_payload: GatewayPayloadObserver<'a>,
    ) -> GatewayFuture<'a, GatewayChatStreamCollected> {
        Box::pin(async { bail!("real gateway chat stream client is disabled in tests") })
    }
}

#[cfg(test)]
#[path = "gateway_executor_tests.rs"]
mod gateway_executor_tests;
