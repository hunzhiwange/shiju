//! Workflow 运行相关共享 DTO。
//!
//! 本模块定义跨网关边界传输的数据结构与共享的工作流标识。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const VIDEO_EDIT_TEMPLATE_UUID: &str = "16cbb033-b312-48b6-9bfc-91c87391b167";

/// 按程序保留的 UUID 识别内置工作流。
pub fn is_builtin_workflow_uuid(uuid: &str) -> bool {
    matches!(uuid, VIDEO_EDIT_TEMPLATE_UUID)
}

fn default_workflow_max_steps() -> u32 {
    200
}

/// Workflow 执行请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowRunRequest {
    /// 本地持久化应用的 UUID。提供该字段时网关会从本地库读取 YAML。
    #[serde(default, rename = "application_uuid", skip_serializing_if = "Option::is_none")]
    pub workflow_uuid: Option<String>,
    /// Dify YAML 文本。
    #[serde(default, rename = "application_workflow", skip_serializing_if = "Option::is_none")]
    pub workflow_yaml: Option<String>,
    /// 用户查询文本，会写入 `sys.query`，并在存在同名开始变量时复用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// 开始节点输入变量。
    #[serde(default)]
    pub inputs: BTreeMap<String, Value>,
    /// 最大执行节点数，防止循环或异常图结构无限运行。
    #[serde(default = "default_workflow_max_steps")]
    pub max_steps: u32,
}

impl Default for WorkflowRunRequest {
    fn default() -> Self {
        Self {
            workflow_uuid: None,
            workflow_yaml: None,
            query: None,
            inputs: BTreeMap::new(),
            max_steps: default_workflow_max_steps(),
        }
    }
}

/// Workflow 执行状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRunStatus {
    Running,
    Paused,
    Succeeded,
    Failed,
}

/// 单个节点执行状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNodeRunStatus {
    Paused,
    Succeeded,
    Failed,
}

/// Human input 暂停信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowPauseDto {
    pub node_id: String,
    pub title: String,
    pub form_token: String,
    pub form: Value,
    #[serde(default)]
    pub actions: Vec<WorkflowHumanActionDto>,
}

/// Human input 动作分支。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowHumanActionDto {
    pub id: String,
    pub label: String,
}

/// 单个节点执行结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowNodeRunDto {
    pub node_id: String,
    pub node_type: String,
    pub title: String,
    pub status: WorkflowNodeRunStatus,
    #[serde(default)]
    pub inputs: BTreeMap<String, Value>,
    #[serde(default)]
    pub outputs: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default)]
    pub elapsed_ms: u64,
}

/// Workflow 执行响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowRunResponse {
    pub run_id: String,
    pub status: WorkflowRunStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(default)]
    pub outputs: BTreeMap<String, Value>,
    #[serde(default)]
    pub nodes: Vec<WorkflowNodeRunDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pause: Option<WorkflowPauseDto>,
}

/// Workflow Human Input 恢复请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowResumeRequest {
    pub run_id: String,
    pub form_token: String,
    #[serde(default)]
    pub form_values: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

/// 本地 Workflow 列表项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRecordSummary {
    pub uuid: String,
    pub name: String,
    pub description: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

/// 本地 Workflow 完整记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRecord {
    pub uuid: String,
    pub name: String,
    pub description: String,
    pub workflow_yaml: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

/// 创建或更新本地 Workflow 的请求体。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRecordUpsertBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub workflow_yaml: String,
}

/// 删除本地 Workflow 的响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRecordDeleteResponse {
    pub uuid: String,
    pub deleted: bool,
}

/// User-editable options stored at `workflow.video_edit` in the template.
/// Missing max_parallel still means a shared budget of five operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VideoEditTemplateOptions {
    #[serde(deserialize_with = "deserialize_video_edit_max_parallel")]
    max_parallel: u8,
}

impl Default for VideoEditTemplateOptions {
    fn default() -> Self {
        Self { max_parallel: 5 }
    }
}

impl VideoEditTemplateOptions {
    pub fn new(max_parallel: u8) -> Result<Self, String> {
        if !(1..=5).contains(&max_parallel) {
            return Err("video_edit.max_parallel must be between 1 and 5".into());
        }
        Ok(Self { max_parallel })
    }

    pub fn max_parallel(&self) -> u8 {
        self.max_parallel
    }
}

fn deserialize_video_edit_max_parallel<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = u8::deserialize(deserializer)?;
    VideoEditTemplateOptions::new(value).map_err(serde::de::Error::custom)?;
    Ok(value)
}

#[cfg(test)]
#[path = "workflow_video_tests.rs"]
mod workflow_video_tests;

/// Sanitized native video progress; never contains tool input, credentials or signed URLs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoEditProgress {
    pub run_id: String,
    pub sequence: u64,
    pub stage: String,
    pub total: usize,
    pub queued: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub peak: usize,
    pub limit: usize,
    pub template_sha256: String,
    pub jobs_queued: usize,
    pub jobs_running: usize,
    pub jobs_completed: usize,
    pub jobs_failed: usize,
}
