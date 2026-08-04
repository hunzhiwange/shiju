//! `tasks.json` 文档模型与加载。

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// `tasks.json` 顶层文档。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanTasksDocument {
    /// 计划目录（相对仓库路径或绝对路径）。
    pub plan_dir: String,
    /// 任务条目列表。
    pub tasks: Vec<PlanTaskEntry>,
}

/// 单个计划任务条目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanTaskEntry {
    /// 任务 ID。
    #[serde(deserialize_with = "deserialize_task_id")]
    pub id: String,
    /// 任务标题。
    pub title: String,
    /// 任务文件名（相对 `plan_dir`）。
    pub file: String,
    /// 所属批次。
    pub batch: u32,
    /// 依赖的更早批次列表。
    #[serde(default)]
    pub depends_on_batches: Vec<u32>,
    /// 任务状态文本（如「未开始」）。
    pub status: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TaskId {
    String(String),
    Signed(i64),
    Unsigned(u64),
}

fn deserialize_task_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    TaskId::deserialize(deserializer).map(|id| match id {
        TaskId::String(value) => value,
        TaskId::Signed(value) => value.to_string(),
        TaskId::Unsigned(value) => value.to_string(),
    })
}

/// 从 JSON 文本解析 `tasks.json`。
pub fn parse_tasks_json(content: &str) -> Result<PlanTasksDocument, String> {
    serde_json::from_str(content).map_err(|error| format!("解析 tasks.json 失败: {error}"))
}

/// 从路径加载并解析 `tasks.json`。
pub fn load_tasks_json(path: &Path) -> Result<PlanTasksDocument, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("读取 tasks.json 失败 ({}): {error}", path.display()))?;
    parse_tasks_json(&content)
}
