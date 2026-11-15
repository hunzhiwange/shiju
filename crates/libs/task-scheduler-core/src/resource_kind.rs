//! 调度器内建资源种类，与 domain 的开放字符串种类保持独立。

use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 调度器内建的资源互斥维度。
pub enum ResourceKind {
    WorkerSlot,
    Worktree,
    TeamCapacity,
    MergeTarget,
}

impl ResourceKind {
    /// 返回资源表和事件载荷使用的稳定种类键。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WorkerSlot => "worker_slot",
            Self::Worktree => "worktree",
            Self::TeamCapacity => "team_capacity",
            Self::MergeTarget => "merge_target",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "worker_slot" => Ok(Self::WorkerSlot),
            "worktree" => Ok(Self::Worktree),
            "team_capacity" => Ok(Self::TeamCapacity),
            "merge_target" => Ok(Self::MergeTarget),
            other => bail!("unsupported resource kind: {other}"),
        }
    }
}
