//! 面向任务看板的步骤及交接配置。

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
#[path = "step_workflow_tests.rs"]
mod tests;

/// 状态库中的进度分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
        if self.id.trim().is_empty() || self.label.trim().is_empty() {
            return Err("workflow id and label must not be empty".into());
        }
        let catalog: HashMap<_, _> =
            statuses.iter().map(|status| (status.key.as_str(), status.category)).collect();
        if catalog.len() != statuses.len() {
            return Err("status library keys must be unique".into());
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
        for step in &self.steps {
            let key = step.status_key.as_str();
            if catalog[key].is_terminal() && step.handoff != WorkflowStepHandoff::KeepAssignee {
                return Err(format!("terminal workflow status cannot hand off: {key}"));
            }
            match &step.handoff {
                WorkflowStepHandoff::KeepAssignee => {}
                WorkflowStepHandoff::Member { member_id } => {
                    if member_id.trim().is_empty() {
                        return Err(format!("workflow status {key} requires a member"));
                    }
                }
                WorkflowStepHandoff::Agent {
                    agent_id,
                    completion_status_key,
                    return_status_key,
                    ..
                } => {
                    if agent_id.trim().is_empty() {
                        return Err(format!("workflow status {key} requires an agent"));
                    }
                    for target in [completion_status_key, return_status_key].into_iter().flatten() {
                        if target == key || !step_keys.contains(target.as_str()) {
                            return Err(format!(
                                "workflow status {key} has invalid target: {target}"
                            ));
                        }
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
