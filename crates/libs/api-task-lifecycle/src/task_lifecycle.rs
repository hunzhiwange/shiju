//! 任务生命周期状态目录和有序步骤的跨网关协议。
use serde::{Deserialize, Serialize};

use crate::step_workflow::StepWorkflow;
pub use crate::step_workflow::{
    WorkflowStatus as TaskLifecycleStatusDto,
    WorkflowStatusCategory as TaskLifecycleStatusCategoryDto, WorkflowStep as TaskLifecycleStepDto,
    WorkflowStepHandoff as TaskLifecycleStepHandoffDto,
};

/// 状态目录与步骤分开保存；步骤只通过稳定 key 引用目录条目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskLifecycleWorkflowDto {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub built_in: bool,
    pub statuses: Vec<TaskLifecycleStatusDto>,
    pub initial_status_key: String,
    pub steps: Vec<TaskLifecycleStepDto>,
}

impl TaskLifecycleWorkflowDto {
    pub fn step_workflow(&self) -> StepWorkflow {
        StepWorkflow {
            id: self.id.clone(),
            label: self.label.clone(),
            initial_status_key: self.initial_status_key.clone(),
            steps: self.steps.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.step_workflow().validate(&self.statuses)
    }

    /// 缺省完成目标取有序步骤的下一项，不跳过停用状态。
    pub fn completion_status_key(&self, status_key: &str) -> Option<&str> {
        let index = self.steps.iter().position(|step| step.status_key == status_key)?;
        let TaskLifecycleStepHandoffDto::Agent { completion_status_key, .. } =
            &self.steps[index].handoff
        else {
            return None;
        };
        completion_status_key
            .as_deref()
            .or_else(|| self.steps.get(index + 1).map(|step| step.status_key.as_str()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskLifecycleWorkflowUpsertBody {
    pub workflow: TaskLifecycleWorkflowDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskLifecycleWorkflowDeleteResponse {
    pub id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub deleted: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}
