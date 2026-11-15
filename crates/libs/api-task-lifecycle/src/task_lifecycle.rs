//! 任务生命周期 workflow 跨网关协议。
//!
//! 本模块只定义生命周期配置 CRUD 所需的共享 DTO。状态目标统一使用稳定 key
//! 字符串，不依赖桌面端任务状态枚举。

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 任务生命周期状态的引擎语义角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskLifecycleStatusRoleDto {
    Backlog,
    Queued,
    Planning,
    Executing,
    Failed,
    Paused,
    ReviewReady,
    Review,
    Merging,
    Done,
    Archived,
    Custom,
}

/// 任务生命周期状态转移分支。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskLifecycleStatusTransitionDto {
    pub outcome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub target: String,
}

/// 任务生命周期状态定义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskLifecycleStatusDto {
    pub key: String,
    pub label: String,
    pub role: TaskLifecycleStatusRoleDto,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub terminal: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto_execution_candidate: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub occupies_execution_slot: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_scheduling_backlog: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transitions: Vec<TaskLifecycleStatusTransitionDto>,
}

/// 任务生命周期 workflow 定义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskLifecycleWorkflowDto {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub built_in: bool,
    #[serde(default)]
    pub statuses: Vec<TaskLifecycleStatusDto>,
}

impl TaskLifecycleWorkflowDto {
    /// 校验跨网关保存和读取所需的最小 workflow 结构约束。
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("workflow id must not be empty".to_string());
        }

        let mut status_keys = HashSet::new();
        for status in &self.statuses {
            if status.key.trim().is_empty() {
                return Err("workflow status key must not be empty".to_string());
            }
            if !status_keys.insert(status.key.as_str()) {
                return Err(format!("workflow status key must be unique: {}", status.key));
            }
        }

        for status in &self.statuses {
            if let Some(next) = status.next.as_deref() {
                validate_workflow_target(&status_keys, "next", next)?;
            }
            for transition in &status.transitions {
                if transition.outcome.trim().is_empty() {
                    return Err(format!(
                        "workflow transition outcome must not be empty: {}",
                        status.key
                    ));
                }
                validate_workflow_target(&status_keys, "transition", transition.target.as_str())?;
            }
        }

        Ok(())
    }
}

/// 创建或更新任务生命周期 workflow 请求体。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskLifecycleWorkflowUpsertBody {
    pub workflow: TaskLifecycleWorkflowDto,
}

/// 删除任务生命周期 workflow 响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskLifecycleWorkflowDeleteResponse {
    pub id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub deleted: bool,
}

fn validate_workflow_target(
    status_keys: &HashSet<&str>,
    target_kind: &str,
    target: &str,
) -> Result<(), String> {
    if target.trim().is_empty() {
        return Err(format!("workflow {target_kind} target must not be empty"));
    }
    if !status_keys.contains(target) {
        return Err(format!("unknown workflow {target_kind} target: {target}"));
    }
    Ok(())
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_true(value: &bool) -> bool {
    *value
}

fn default_true() -> bool {
    true
}
