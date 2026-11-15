use vw_api_task::{
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

impl GatewayClient {
    /// 按可恢复流请求标识幂等停止 plan-run 运行。
    pub async fn task_plan_run_stream_stop(&self, request_id: &str) -> Result<bool, String> {
        gateway_task_pool::GatewayTaskPoolClient::new(self.transport().clone())
            .task_plan_run_stream_stop(request_id)
            .await
    }

    /// 订阅聊天 workflow 模式的计划执行 SSE。
    pub async fn stream_task_plan_run(
        endpoint: &crate::endpoint::GatewayEndpoint,
        body: &TaskPlanRunRequest,
        on_event: impl FnMut(TaskPlanRunStreamEventDto) -> bool,
    ) -> Result<(), String> {
        gateway_task_pool::GatewayTaskPoolClient::stream_task_plan_run(endpoint, body, on_event).await
    }

    /// 执行一次新 plan-run 流并返回明确终止原因。
    pub async fn stream_task_plan_run_with_result(
        endpoint: &crate::endpoint::GatewayEndpoint,
        body: &TaskPlanRunRequest,
        on_event: impl FnMut(crate::stream::GatewayStreamEvent<TaskPlanRunStreamEventDto>) -> bool,
    ) -> Result<crate::stream::GatewayStreamAttemptResult, String> {
        gateway_task_pool::GatewayTaskPoolClient::stream_task_plan_run_with_result(
            endpoint, body, on_event,
        )
        .await
    }

    /// 执行一次支持请求复用与游标恢复的计划运行流订阅。
    pub async fn stream_task_plan_run_resumable(
        endpoint: &crate::endpoint::GatewayEndpoint,
        body: &TaskPlanRunRequest,
        resume: &crate::stream::GatewayStreamResume,
        on_event: impl FnMut(crate::stream::GatewayStreamEvent<TaskPlanRunStreamEventDto>) -> bool,
    ) -> Result<crate::stream::GatewayStreamAttemptResult, String> {
        gateway_task_pool::GatewayTaskPoolClient::stream_task_plan_run_resumable(
            endpoint, body, resume, on_event,
        )
        .await
    }

    /// 查询当前租户下的 plan-run 状态。
    pub async fn task_plan_run_status(
        &self,
        task_id: &str,
    ) -> Result<TaskPlanRunStatusResponse, String> {
        gateway_task_pool::GatewayTaskPoolClient::new(self.transport().clone())
            .task_plan_run_status(task_id)
            .await
    }

    /// 幂等停止当前租户下的 plan-run。
    pub async fn task_plan_run_stop(
        &self,
        task_id: &str,
    ) -> Result<TaskPlanRunStatusResponse, String> {
        gateway_task_pool::GatewayTaskPoolClient::new(self.transport().clone())
            .task_plan_run_stop(task_id)
            .await
    }

    /// 读取任务池 worktree 池快照。
    pub async fn task_pool_worktree_snapshot(
        &self,
        request: &TaskPoolWorktreeSnapshotRequest,
    ) -> Result<TaskPoolWorktreeSnapshotResponse, String> {
        gateway_task_pool::GatewayTaskPoolClient::new(self.transport().clone())
            .task_pool_worktree_snapshot(request)
            .await
    }

    /// 执行任务池 worktree 池维护。
    pub async fn task_pool_worktree_maintain(
        &self,
        request: &TaskPoolWorktreeMaintenanceRequest,
    ) -> Result<TaskPoolWorktreeMaintenanceResponse, String> {
        gateway_task_pool::GatewayTaskPoolClient::new(self.transport().clone())
            .task_pool_worktree_maintain(request)
            .await
    }

    /// 获取任务池 worktree 槽位租约。
    pub async fn task_pool_worktree_acquire(
        &self,
        request: &TaskPoolWorktreeAcquireRequest,
    ) -> Result<TaskPoolWorktreeAcquireResponse, String> {
        gateway_task_pool::GatewayTaskPoolClient::new(self.transport().clone())
            .task_pool_worktree_acquire(request)
            .await
    }

    /// 释放任务池 worktree 槽位租约。
    pub async fn task_pool_worktree_release(
        &self,
        request: &TaskPoolWorktreeReleaseRequest,
    ) -> Result<TaskPoolWorktreeReleaseResponse, String> {
        gateway_task_pool::GatewayTaskPoolClient::new(self.transport().clone())
            .task_pool_worktree_release(request)
            .await
    }

    /// 回收指定任务占用的任务池 worktree 槽位。
    pub async fn task_pool_worktree_recycle(
        &self,
        request: &TaskPoolWorktreeRecycleRequest,
    ) -> Result<TaskPoolWorktreeRecycleResponse, String> {
        gateway_task_pool::GatewayTaskPoolClient::new(self.transport().clone())
            .task_pool_worktree_recycle(request)
            .await
    }

    /// 回收指定任务池 worktree 槽位。
    pub async fn task_pool_worktree_slot_recycle(
        &self,
        request: &TaskPoolWorktreeSlotRecycleRequest,
    ) -> Result<TaskPoolWorktreeSlotRecycleResponse, String> {
        gateway_task_pool::GatewayTaskPoolClient::new(self.transport().clone())
            .task_pool_worktree_slot_recycle(request)
            .await
    }

    /// 获取任务池 worktree 合并目标锁。
    pub async fn task_pool_worktree_merge_lock(
        &self,
        request: &TaskPoolWorktreeMergeLockRequest,
    ) -> Result<TaskPoolWorktreeMergeLockResponse, String> {
        gateway_task_pool::GatewayTaskPoolClient::new(self.transport().clone())
            .task_pool_worktree_merge_lock(request)
            .await
    }

    /// 释放任务池 worktree 合并目标锁。
    pub async fn task_pool_worktree_merge_unlock(
        &self,
        request: &TaskPoolWorktreeMergeUnlockRequest,
    ) -> Result<TaskPoolWorktreeMergeUnlockResponse, String> {
        gateway_task_pool::GatewayTaskPoolClient::new(self.transport().clone())
            .task_pool_worktree_merge_unlock(request)
            .await
    }
}

#[cfg(test)]
#[path = "task_pool_api_tests.rs"]
mod task_pool_api_tests;
