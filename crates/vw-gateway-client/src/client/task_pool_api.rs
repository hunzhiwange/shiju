use serde::Deserialize;
use vw_api_types::task::{
    TaskPlanRunRequest, TaskPlanRunStatusResponse, TaskPlanRunStreamEventDto,
    TaskPoolWorktreeAcquireRequest, TaskPoolWorktreeAcquireResponse,
    TaskPoolWorktreeMaintenanceRequest, TaskPoolWorktreeMaintenanceResponse,
    TaskPoolWorktreeMergeLockRequest, TaskPoolWorktreeMergeLockResponse,
    TaskPoolWorktreeMergeUnlockRequest, TaskPoolWorktreeMergeUnlockResponse,
    TaskPoolWorktreeRecycleRequest, TaskPoolWorktreeRecycleResponse,
    TaskPoolWorktreeReleaseRequest, TaskPoolWorktreeReleaseResponse,
    TaskPoolWorktreeSlotRecycleRequest, TaskPoolWorktreeSlotRecycleResponse,
    TaskPoolWorktreeSnapshotRequest, TaskPoolWorktreeSnapshotResponse,
};

use super::GatewayClient;
const TASK_PLAN_RUN_STREAM_PATH: &str = "/v1/task-plan-run/stream";
const TASK_PLAN_RUN_PATH: &str = "/v1/task-plan-run";
const TASK_POOL_WORKTREE_SNAPSHOT_PATH: &str = "/v1/task-pool/worktrees/snapshot";
const TASK_POOL_WORKTREE_MAINTAIN_PATH: &str = "/v1/task-pool/worktrees/maintain";
const TASK_POOL_WORKTREE_ACQUIRE_PATH: &str = "/v1/task-pool/worktrees/acquire";
const TASK_POOL_WORKTREE_RELEASE_PATH: &str = "/v1/task-pool/worktrees/release";
const TASK_POOL_WORKTREE_RECYCLE_PATH: &str = "/v1/task-pool/worktrees/recycle";
const TASK_POOL_WORKTREE_SLOT_RECYCLE_PATH: &str = "/v1/task-pool/worktrees/slots/recycle";
const TASK_POOL_WORKTREE_MERGE_LOCK_PATH: &str = "/v1/task-pool/worktrees/merge-lock";
const TASK_POOL_WORKTREE_MERGE_UNLOCK_PATH: &str = "/v1/task-pool/worktrees/merge-unlock";

#[derive(Debug, Deserialize)]
struct StreamStopResponse {
    cancelled: bool,
}

impl GatewayClient {
    /// 按可恢复流请求标识幂等停止 plan-run 运行。
    pub async fn task_plan_run_stream_stop(&self, request_id: &str) -> Result<bool, String> {
        let path = format!(
            "{TASK_PLAN_RUN_STREAM_PATH}/{}/stop",
            encode_task_pool_path_segment(required_stream_request_id(request_id)?)
        );
        let response: StreamStopResponse =
            self.post_json(&path, &[], &serde_json::json!({})).await?;
        Ok(response.cancelled)
    }

    /// 订阅聊天 workflow 模式的计划执行 SSE。
    pub async fn stream_task_plan_run(
        endpoint: &crate::endpoint::GatewayEndpoint,
        body: &TaskPlanRunRequest,
        mut on_event: impl FnMut(TaskPlanRunStreamEventDto) -> bool,
    ) -> Result<(), String> {
        Self::stream_task_plan_run_attempt(endpoint, body, None, |event| on_event(event.event))
            .await
            .map(|_| ())
    }

    /// 执行一次新 plan-run 流并返回明确终止原因。
    pub async fn stream_task_plan_run_with_result(
        endpoint: &crate::endpoint::GatewayEndpoint,
        body: &TaskPlanRunRequest,
        on_event: impl FnMut(crate::stream::GatewayStreamEvent<TaskPlanRunStreamEventDto>) -> bool,
    ) -> Result<crate::stream::GatewayStreamAttemptResult, String> {
        Self::stream_task_plan_run_attempt(endpoint, body, None, on_event).await
    }

    /// 执行一次支持请求复用与游标恢复的计划运行流订阅。
    pub async fn stream_task_plan_run_resumable(
        endpoint: &crate::endpoint::GatewayEndpoint,
        body: &TaskPlanRunRequest,
        resume: &crate::stream::GatewayStreamResume,
        on_event: impl FnMut(crate::stream::GatewayStreamEvent<TaskPlanRunStreamEventDto>) -> bool,
    ) -> Result<crate::stream::GatewayStreamAttemptResult, String> {
        Self::stream_task_plan_run_attempt(endpoint, body, Some(resume), on_event).await
    }

    async fn stream_task_plan_run_attempt(
        endpoint: &crate::endpoint::GatewayEndpoint,
        body: &TaskPlanRunRequest,
        resume: Option<&crate::stream::GatewayStreamResume>,
        mut on_event: impl FnMut(crate::stream::GatewayStreamEvent<TaskPlanRunStreamEventDto>) -> bool,
    ) -> Result<crate::stream::GatewayStreamAttemptResult, String> {
        use futures_util::StreamExt;

        use crate::http::{
            apply_request_context, log_request, log_request_succeeded, response_error,
            transport_error,
        };

        log_request("POST", endpoint, TASK_PLAN_RUN_STREAM_PATH, &[], Some(body));

        #[cfg(not(target_arch = "wasm32"))]
        let builder = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(
                crate::stream::STREAM_CONNECT_TIMEOUT_SECS,
            ))
            .timeout(std::time::Duration::from_secs(crate::stream::STREAM_REQUEST_TIMEOUT_SECS));
        #[cfg(target_arch = "wasm32")]
        let builder = reqwest::Client::builder();

        let client = builder
            .build()
            .map_err(|err| transport_error("POST", endpoint, TASK_PLAN_RUN_STREAM_PATH, err))?;
        let mut request =
            client.post(format!("{}{}", endpoint.base_url(), TASK_PLAN_RUN_STREAM_PATH)).json(body);
        if let Some(resume) = resume {
            request = crate::stream::apply_stream_resume(request, resume);
        }
        let request = apply_request_context(request, endpoint);
        let response = request
            .send()
            .await
            .map_err(|err| transport_error("POST", endpoint, TASK_PLAN_RUN_STREAM_PATH, err))?;
        if !response.status().is_success() {
            return Err(response_error("POST", endpoint, TASK_PLAN_RUN_STREAM_PATH, response).await);
        }

        log_request_succeeded("POST", endpoint, TASK_PLAN_RUN_STREAM_PATH);
        let mut bytes_stream = response.bytes_stream();
        let mut sse_buffer = String::new();
        while let Some(chunk) = bytes_stream.next().await {
            let chunk = chunk.map_err(|err| err.to_string())?;
            sse_buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(frame) = crate::stream::take_next_sse_event(&mut sse_buffer) {
                let Some(frame) = crate::stream::parse_sse_frame(&frame)? else {
                    continue;
                };
                let id = match (frame.id, resume.is_some()) {
                    (Some(id), _) => id,
                    (None, true) => {
                        return Err("gateway resumable stream event is missing SSE id".to_string());
                    }
                    (None, false) => 0,
                };

                let event = serde_json::from_str::<TaskPlanRunStreamEventDto>(&frame.data)
                    .map_err(|err| err.to_string())?;
                // Blocked 与 Finished/Error 同为终态：服务端在 Blocked 后会结束 SSE。
                let terminal = matches!(
                    event,
                    TaskPlanRunStreamEventDto::Finished { .. }
                        | TaskPlanRunStreamEventDto::Error { .. }
                        | TaskPlanRunStreamEventDto::Blocked { .. }
                        | TaskPlanRunStreamEventDto::Cancelled { .. }
                );
                if !on_event(crate::stream::GatewayStreamEvent { id, event }) {
                    return Ok(crate::stream::GatewayStreamAttemptResult::CallerStopped);
                }
                if terminal {
                    return Ok(crate::stream::GatewayStreamAttemptResult::Terminated);
                }
            }
        }

        Ok(crate::stream::GatewayStreamAttemptResult::Incomplete)
    }

    /// 查询当前租户下的 plan-run 状态。
    pub async fn task_plan_run_status(
        &self,
        task_id: &str,
    ) -> Result<TaskPlanRunStatusResponse, String> {
        let path = format!("{TASK_PLAN_RUN_PATH}/{task_id}/status");
        self.get_json(&path, &[]).await
    }

    /// 幂等停止当前租户下的 plan-run。
    pub async fn task_plan_run_stop(
        &self,
        task_id: &str,
    ) -> Result<TaskPlanRunStatusResponse, String> {
        let path = format!("{TASK_PLAN_RUN_PATH}/{task_id}/stop");
        self.post_json(&path, &[], &serde_json::json!({})).await
    }

    /// 读取任务池 worktree 池快照。
    pub async fn task_pool_worktree_snapshot(
        &self,
        request: &TaskPoolWorktreeSnapshotRequest,
    ) -> Result<TaskPoolWorktreeSnapshotResponse, String> {
        self.post_json(TASK_POOL_WORKTREE_SNAPSHOT_PATH, &[], request).await
    }

    /// 执行任务池 worktree 池维护。
    pub async fn task_pool_worktree_maintain(
        &self,
        request: &TaskPoolWorktreeMaintenanceRequest,
    ) -> Result<TaskPoolWorktreeMaintenanceResponse, String> {
        self.post_json(TASK_POOL_WORKTREE_MAINTAIN_PATH, &[], request).await
    }

    /// 获取任务池 worktree 槽位租约。
    pub async fn task_pool_worktree_acquire(
        &self,
        request: &TaskPoolWorktreeAcquireRequest,
    ) -> Result<TaskPoolWorktreeAcquireResponse, String> {
        self.post_json(TASK_POOL_WORKTREE_ACQUIRE_PATH, &[], request).await
    }

    /// 释放任务池 worktree 槽位租约。
    pub async fn task_pool_worktree_release(
        &self,
        request: &TaskPoolWorktreeReleaseRequest,
    ) -> Result<TaskPoolWorktreeReleaseResponse, String> {
        self.post_json(TASK_POOL_WORKTREE_RELEASE_PATH, &[], request).await
    }

    /// 回收指定任务占用的任务池 worktree 槽位。
    pub async fn task_pool_worktree_recycle(
        &self,
        request: &TaskPoolWorktreeRecycleRequest,
    ) -> Result<TaskPoolWorktreeRecycleResponse, String> {
        self.post_json(TASK_POOL_WORKTREE_RECYCLE_PATH, &[], request).await
    }

    /// 回收指定任务池 worktree 槽位。
    pub async fn task_pool_worktree_slot_recycle(
        &self,
        request: &TaskPoolWorktreeSlotRecycleRequest,
    ) -> Result<TaskPoolWorktreeSlotRecycleResponse, String> {
        self.post_json(TASK_POOL_WORKTREE_SLOT_RECYCLE_PATH, &[], request).await
    }

    /// 获取任务池 worktree 合并目标锁。
    pub async fn task_pool_worktree_merge_lock(
        &self,
        request: &TaskPoolWorktreeMergeLockRequest,
    ) -> Result<TaskPoolWorktreeMergeLockResponse, String> {
        self.post_json(TASK_POOL_WORKTREE_MERGE_LOCK_PATH, &[], request).await
    }

    /// 释放任务池 worktree 合并目标锁。
    pub async fn task_pool_worktree_merge_unlock(
        &self,
        request: &TaskPoolWorktreeMergeUnlockRequest,
    ) -> Result<TaskPoolWorktreeMergeUnlockResponse, String> {
        self.post_json(TASK_POOL_WORKTREE_MERGE_UNLOCK_PATH, &[], request).await
    }
}

fn required_stream_request_id(request_id: &str) -> Result<&str, String> {
    if request_id.trim().is_empty() {
        return Err("stream request id must not be empty".to_string());
    }
    Ok(request_id)
}

fn encode_task_pool_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
#[path = "task_pool_api_tests.rs"]
mod task_pool_api_tests;
