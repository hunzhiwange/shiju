//! 后台任务与任务池契约的兼容重导出。

pub use vw_api_task::{
    CancelTaskResponse, CommandTaskSpecDto, CreateTaskRequest, CreateTaskResponse, GetTaskResponse,
    ListTasksRequest, ListTasksResponse, TaskCompletedEvent, TaskDto, TaskErrorEvent, TaskEvent,
    TaskKind, TaskPlanRunRequest, TaskPlanRunStatus, TaskPlanRunStatusResponse,
    TaskPlanRunStreamEventDto, TaskPoolStatus, TaskPoolWorkflowSnapshotDto,
    TaskPoolWorkflowStatusCategoryDto, TaskPoolWorkflowStatusDto, TaskPoolWorkflowStepDto,
    TaskPoolWorkflowStepHandoffDto, TaskPoolWorktreeAcquireRequest,
    TaskPoolWorktreeAcquireResponse, TaskPoolWorktreeMaintenanceRequest,
    TaskPoolWorktreeMaintenanceResponse, TaskPoolWorktreeMergeLockDto,
    TaskPoolWorktreeMergeLockRequest, TaskPoolWorktreeMergeLockResponse,
    TaskPoolWorktreeMergeUnlockRequest, TaskPoolWorktreeMergeUnlockResponse,
    TaskPoolWorktreeRecycleRequest, TaskPoolWorktreeRecycleResponse,
    TaskPoolWorktreeReleaseRequest, TaskPoolWorktreeReleaseResponse, TaskPoolWorktreeSlotDto,
    TaskPoolWorktreeSlotRecycleRequest, TaskPoolWorktreeSlotRecycleResponse,
    TaskPoolWorktreeSlotStateDto, TaskPoolWorktreeSnapshotRequest,
    TaskPoolWorktreeSnapshotResponse, TaskStartedEvent, TaskStatus, TaskStderrEvent,
    TaskStdoutEvent,
};
