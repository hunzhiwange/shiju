//! 工作流步骤只决定进入规则；运行结果不隐式移动步骤。
use crate::state_machine::AgentTaskStatus;
use anyhow::{Result, bail};
use serde_json::{Map, Value, json};
use vw_api_task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusCategoryDto, TaskPoolWorkflowStatusDto,
    TaskPoolWorkflowStepDto, TaskPoolWorkflowStepHandoffDto,
};

pub fn builtin_task_workflow_snapshot() -> TaskPoolWorkflowSnapshotDto {
    task_workflow_snapshot(
        task_lifecycle::built_in_workflow_by_id(task_lifecycle::DEFAULT_TASK_LIFECYCLE_WORKFLOW_ID)
            .expect("default lifecycle is registered"),
    )
}
pub(crate) fn task_workflow_snapshot(
    workflow: vw_api_task_lifecycle::TaskLifecycleWorkflowDto,
) -> TaskPoolWorkflowSnapshotDto {
    workflow.into()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleStatusProjection {
    pub status_key: String,
    pub status_category: TaskPoolWorkflowStatusCategoryDto,
    pub runtime_status: AgentTaskStatus,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleDispatchEligibility {
    pub status: LifecycleStatusProjection,
    pub eligible: bool,
}
pub struct LifecycleBridge<'a> {
    workflow: &'a TaskPoolWorkflowSnapshotDto,
}
impl<'a> LifecycleBridge<'a> {
    pub fn new(workflow: &'a TaskPoolWorkflowSnapshotDto) -> Self {
        Self { workflow }
    }
    pub fn status(&self, key: &str) -> Result<&'a TaskPoolWorkflowStatusDto> {
        self.step(key)?;
        self.workflow
            .statuses
            .iter()
            .find(|s| s.key == key && s.enabled)
            .ok_or_else(|| anyhow::anyhow!("enabled workflow status `{key}` not found"))
    }
    pub fn step(&self, key: &str) -> Result<&'a TaskPoolWorkflowStepDto> {
        self.workflow
            .steps
            .iter()
            .find(|s| s.status_key == key)
            .ok_or_else(|| anyhow::anyhow!("workflow step `{key}` not found"))
    }
    pub fn resolve_status(&self, key: &str) -> Result<LifecycleStatusProjection> {
        let status = self.status(key)?;
        let runtime_status = match status.category {
            TaskPoolWorkflowStatusCategoryDto::Done => AgentTaskStatus::Completed,
            TaskPoolWorkflowStatusCategoryDto::Closed => AgentTaskStatus::Cancelled,
            _ => match &self.step(key)?.handoff {
                TaskPoolWorkflowStepHandoffDto::Agent { .. } => AgentTaskStatus::Queued,
                _ => AgentTaskStatus::Draft,
            },
        };
        Ok(LifecycleStatusProjection {
            status_key: key.to_owned(),
            status_category: status.category,
            runtime_status,
        })
    }
    // Policy retries update the runtime in the same step. Completion/return require an explicit move.
    pub fn resolve_outcome(&self, key: &str, outcome: &str) -> Result<LifecycleStatusProjection> {
        let status = self.resolve_status(key)?;
        if !matches!(outcome, "retry" | "failure")
            || status.runtime_status != AgentTaskStatus::Queued
        {
            bail!("workflow steps require an explicit move for outcome `{outcome}`");
        }
        if outcome == "failure" {
            return Ok(LifecycleStatusProjection {
                runtime_status: AgentTaskStatus::Failed,
                ..status
            });
        }
        Ok(status)
    }
    pub fn execution_status_key(&self, key: &str) -> Result<&'a str> {
        self.status(key)?;
        let step = self.step(key)?;
        if !matches!(step.handoff, TaskPoolWorkflowStepHandoffDto::Agent { .. }) {
            bail!("workflow step `{key}` does not hand off to an agent");
        }
        Ok(&step.status_key)
    }
    pub fn dispatch_eligibility(&self, key: &str) -> Result<LifecycleDispatchEligibility> {
        let status = self.resolve_status(key)?;
        let eligible = !status.status_category.is_terminal()
            && matches!(self.step(key)?.handoff, TaskPoolWorkflowStepHandoffDto::Agent { .. });
        Ok(LifecycleDispatchEligibility { status, eligible })
    }
    pub fn projection_metadata(&self, key: &str) -> Result<Map<String, Value>> {
        let status = self.status(key)?;
        Ok(Map::from_iter([
            ("task_board.status_key".into(), json!(key)),
            ("task_board.status_category".into(), json!(status.category)),
            ("task_board.workflow_id".into(), json!(self.workflow.id)),
        ]))
    }
}
pub fn status_category_key(category: TaskPoolWorkflowStatusCategoryDto) -> &'static str {
    match category {
        TaskPoolWorkflowStatusCategoryDto::Unstarted => "unstarted",
        TaskPoolWorkflowStatusCategoryDto::Started => "started",
        TaskPoolWorkflowStatusCategoryDto::Done => "done",
        TaskPoolWorkflowStatusCategoryDto::Closed => "closed",
    }
}
pub(crate) fn normalize_workflow_transitions(
    workflow: TaskPoolWorkflowSnapshotDto,
) -> TaskPoolWorkflowSnapshotDto {
    workflow
}
