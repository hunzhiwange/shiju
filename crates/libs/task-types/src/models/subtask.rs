use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use super::now_ms;

static SUBTASK_ID_SEQUENCE: AtomicU32 = AtomicU32::new(0);

/// SubTask 数据结构承载该模块对外传递的 SubTask 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub boundary: String,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub target_files: Vec<String>,
    pub created_at_ms: u64,
    pub order: u32,
    pub completed: bool,
    #[serde(default = "default_subtask_status")]
    pub status: SubTaskStatus,
    #[serde(default)]
    pub execution_started_at_ms: Option<u64>,
    #[serde(default)]
    pub last_execution_duration_ms: Option<u64>,
}

/// 子任务执行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubTaskStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
}

fn default_subtask_status() -> SubTaskStatus {
    SubTaskStatus::Pending
}

impl SubTask {
    /// 提供 new 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn new(content: String) -> Self {
        let ms = now_ms();
        let seq = SUBTASK_ID_SEQUENCE.fetch_add(1, Ordering::Relaxed) % 100000;
        Self {
            id: format!("SUB-{ms}.{seq:05}"),
            content,
            boundary: String::new(),
            acceptance_criteria: Vec::new(),
            target_files: Vec::new(),
            created_at_ms: ms,
            order: 0,
            completed: false,
            status: SubTaskStatus::Pending,
            execution_started_at_ms: None,
            last_execution_duration_ms: None,
        }
    }

    pub fn start_execution(&mut self) {
        self.status = SubTaskStatus::Running;
        self.completed = false;
        self.execution_started_at_ms = Some(now_ms());
        self.last_execution_duration_ms = None;
    }

    pub fn mark_completed(&mut self) {
        if let Some(started_at) = self.execution_started_at_ms {
            self.last_execution_duration_ms = Some(now_ms().saturating_sub(started_at));
        }
        self.execution_started_at_ms = None;
        self.status = SubTaskStatus::Completed;
        self.completed = true;
    }

    pub fn mark_failed(&mut self) {
        if let Some(started_at) = self.execution_started_at_ms {
            self.last_execution_duration_ms = Some(now_ms().saturating_sub(started_at));
        }
        self.execution_started_at_ms = None;
        self.status = SubTaskStatus::Failed;
        self.completed = false;
    }

    pub fn display_execution_duration_ms(&self, now_ms: u64) -> Option<u64> {
        if self.status == SubTaskStatus::Running {
            let started_at = self.execution_started_at_ms.unwrap_or(self.created_at_ms);
            return Some(now_ms.saturating_sub(started_at));
        }
        self.last_execution_duration_ms
    }
}
