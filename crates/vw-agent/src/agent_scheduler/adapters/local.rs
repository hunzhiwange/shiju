//! 描述向本地执行器派发任务时所需的稳定标识与请求时间。

use vw_api_types::common::TimestampMs;

use crate::agent_scheduler::domain::{RunId, TaskId, WorkerName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDispatchRequest {
    pub task_id: TaskId,
    pub run_id: RunId,
    pub worker_id: WorkerName,
    pub requested_at_ms: TimestampMs,
}
