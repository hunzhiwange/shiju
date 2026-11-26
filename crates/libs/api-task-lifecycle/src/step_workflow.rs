//! 面向任务看板的步骤及交接配置。

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
#[path = "step_workflow_tests.rs"]
mod tests;

/// 状态库中的进度分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatusCategory {
    Unstarted,
    Started,
    Done,
    Closed,
}

impl WorkflowStatusCategory {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Closed)
    }
}

/// 可供工作流引用的状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStatus {
    pub key: String,
    pub label: String,
    pub category: WorkflowStatusCategory,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

/// 任务实际进入步骤时的接手规则。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkflowStepHandoff {
    KeepAssignee,
    Member {
        member_id: String,
    },
    Agent {
        agent_id: String,
        instructions: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        completion_status_key: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        return_status_key: Option<String>,
    },
}

/// 工作流中的一个有序步骤。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStep {
    pub status_key: String,
    pub handoff: WorkflowStepHandoff,
}

/// 交接与看板步骤配置。运行态由调度器单独管理。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepWorkflow {
    pub id: String,
    pub label: String,
    pub initial_status_key: String,
    pub steps: Vec<WorkflowStep>,
}

impl StepWorkflow {
    /// 检查步骤引用及目标，负责人归属由网关检查。
    pub fn validate(&self, statuses: &[WorkflowStatus]) -> Result<(), String> {
        if !is_valid_key(&self.id) || self.label.trim().is_empty() {
            return Err(
                "workflow id must be a stable lowercase key and label must not be empty".into()
            );
        }
        let mut catalog = HashMap::new();
        for status in statuses {
            if !is_valid_key(&status.key) || status.label.trim().is_empty() {
                return Err(format!("invalid status library entry: {}", status.key));
            }
            if catalog.insert(status.key.as_str(), status).is_some() {
                return Err(format!("status library keys must be unique: {}", status.key));
            }
        }
        let mut step_keys = HashSet::new();
        for step in &self.steps {
            if !catalog.contains_key(step.status_key.as_str()) {
                return Err(format!("unknown workflow status: {}", step.status_key));
            }
            if !step_keys.insert(step.status_key.as_str()) {
                return Err(format!("duplicate workflow status: {}", step.status_key));
            }
        }
        if !step_keys.contains(self.initial_status_key.as_str()) {
            return Err(format!("unknown initial workflow status: {}", self.initial_status_key));
        }
        if !catalog[self.initial_status_key.as_str()].enabled {
            return Err(format!(
                "initial workflow status is disabled: {}",
                self.initial_status_key
            ));
        }
        for step in &self.steps {
            let key = step.status_key.as_str();
            if catalog[key].category.is_terminal()
                && step.handoff != WorkflowStepHandoff::KeepAssignee
            {
                return Err(format!("terminal workflow status cannot hand off: {key}"));
            }
            match &step.handoff {
                WorkflowStepHandoff::KeepAssignee => {}
                WorkflowStepHandoff::Member { member_id } => {
                    if !is_valid_assignee_id(member_id) {
                        return Err(format!("workflow status {key} requires a member"));
                    }
                }
                WorkflowStepHandoff::Agent {
                    agent_id,
                    completion_status_key: _,
                    return_status_key,
                    instructions,
                } => {
                    if !is_valid_assignee_id(agent_id) {
                        return Err(format!("workflow status {key} requires an agent"));
                    }
                    if instructions.trim().is_empty() {
                        return Err(format!("workflow status {key} requires instructions"));
                    }
                    let completion = self.completion_status_key(key).ok_or_else(|| {
                        format!("workflow status {key} requires a completion target")
                    })?;
                    for target in
                        [Some(completion), return_status_key.as_deref()].into_iter().flatten()
                    {
                        if target == key || !step_keys.contains(target) {
                            return Err(format!(
                                "workflow status {key} has invalid target: {target}"
                            ));
                        }
                        if !catalog[target].enabled {
                            return Err(format!(
                                "workflow status {key} target is disabled: {target}"
                            ));
                        }
                    }
                    if return_status_key
                        .as_deref()
                        .is_some_and(|target| catalog[target].category.is_terminal())
                    {
                        return Err(format!(
                            "workflow status {key} cannot return to a terminal status"
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// 返回智能体步骤的完成目标，省略时使用后续步骤。
    pub fn completion_status_key(&self, status_key: &str) -> Option<&str> {
        let index = self.steps.iter().position(|step| step.status_key == status_key)?;
        let WorkflowStepHandoff::Agent { completion_status_key, .. } = &self.steps[index].handoff
        else {
            return None;
        };
        completion_status_key
            .as_deref()
            .or_else(|| self.steps.get(index + 1).map(|step| step.status_key.as_str()))
    }
}

fn is_valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

// Identity existence and project membership are checked by the gateway. IDs remain opaque.
fn is_valid_assignee_id(id: &str) -> bool {
    !id.is_empty() && id.trim() == id && !id.chars().any(char::is_control)
}

fn default_true() -> bool {
    true
}
fn is_true(value: &bool) -> bool {
    *value
}
