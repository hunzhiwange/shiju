//! 描述向本地执行器派发任务时所需的稳定标识与请求时间。

use vw_api_types::common::TimestampMs;

use crate::agent_scheduler::domain::{RunId, TaskId};

#[derive(Debug, Clone, PartialEq, Eq)]
/// 本地执行器启动一次已认领运行所需的最小不可变请求。
pub struct LocalDispatchRequest {
    pub task_id: TaskId,
    pub run_id: RunId,
    pub requested_at_ms: TimestampMs,
}
