use serde::{Deserialize, Serialize};

/// 文件差异在快照中的变更状态。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiffStatus {
    Added,
    Deleted,
    Modified,
}

/// 单个文件的差异摘要。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub file: String,
    pub before: String,
    pub after: String,
    pub additions: i64,
    pub deletions: i64,
    pub status: Option<DiffStatus>,
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod snapshot_tests;
