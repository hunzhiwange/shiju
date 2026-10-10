//! 在可配置工作流状态与调度器运行时状态之间投影生命周期和派发资格。

use std::collections::HashSet;

use anyhow::{Result, bail};
use serde_json::{Map, Value, json};
use vw_api_types::task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto, TaskPoolWorkflowStatusRoleDto,
    TaskPoolWorkflowTransitionDto,
};

use crate::agent_scheduler::state_machine::AgentTaskStatus;

/// 不依赖员工配置的内建任务生命周期，用于尚未保存任务板工作流的项目。
pub fn builtin_task_workflow_snapshot() -> TaskPoolWorkflowSnapshotDto {
    let definitions = [
        ("pool", "Pool", TaskPoolWorkflowStatusRoleDto::Backlog, false),
        ("pending", "Pending", TaskPoolWorkflowStatusRoleDto::Queued, false),
        ("planning", "Planning", TaskPoolWorkflowStatusRoleDto::Planning, false),
        ("running", "Running", TaskPoolWorkflowStatusRoleDto::Executing, false),
        ("failed", "Failed", TaskPoolWorkflowStatusRoleDto::Failed, false),
        ("paused", "Paused", TaskPoolWorkflowStatusRoleDto::Paused, false),
        ("code_complete", "Code Complete", TaskPoolWorkflowStatusRoleDto::ReviewReady, false),
        ("code_review", "Code Review", TaskPoolWorkflowStatusRoleDto::Review, false),
        ("pr_submitted", "PR Submitted", TaskPoolWorkflowStatusRoleDto::Merging, false),
        ("completed", "Completed", TaskPoolWorkflowStatusRoleDto::Done, true),
        ("archived", "Archived", TaskPoolWorkflowStatusRoleDto::Archived, true),
    ];
    let statuses = definitions
        .iter()
        .enumerate()
        .map(|(index, (key, label, role, terminal))| {
            let next = definitions.get(index + 1).map(|(key, ..)| (*key).to_string());
            let transitions = if *role == TaskPoolWorkflowStatusRoleDto::Failed {
                next.as_ref()
                    .map(|target| {
                        vec![TaskPoolWorkflowTransitionDto {
                            outcome: "retry".to_string(),
                            label: Some("重试".to_string()),
                            target: target.clone(),
                        }]
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            TaskPoolWorkflowStatusDto {
                key: (*key).to_string(),
                label: (*label).to_string(),
                role: *role,
                enabled: true,
                terminal: *terminal,
                next,
                transitions,
            }
        })
        .collect();
    TaskPoolWorkflowSnapshotDto {
        id: "dev_lifecycle".to_string(),
        label: "Default Task Workflow".to_string(),
        statuses,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 工作流状态投影到运行时状态后的结果。
pub struct LifecycleStatusProjection {
    pub status_key: String,
    pub status_role: TaskPoolWorkflowStatusRoleDto,
    pub runtime_status: AgentTaskStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 状态是否允许派发及其规范化投影。
pub struct LifecycleDispatchEligibility {
    pub status: LifecycleStatusProjection,
    pub eligible: bool,
}

/// 针对一份不可变工作流快照解析状态和结果转换。
pub struct LifecycleBridge<'a> {
    workflow: &'a TaskPoolWorkflowSnapshotDto,
}

impl<'a> LifecycleBridge<'a> {
    pub fn new(workflow: &'a TaskPoolWorkflowSnapshotDto) -> Self {
        Self { workflow }
    }

    /// 按键获取工作流状态；空键或未知键返回错误。
    pub fn status(&self, status_key: &str) -> Result<&'a TaskPoolWorkflowStatusDto> {
        let status_key = required_text(status_key, "workflow status key")?;
        match self.find_status(status_key) {
            Some(status) if status.enabled => Ok(status),
            Some(_) => bail!("workflow status `{status_key}` is disabled"),
            None => {
                bail!("workflow status `{status_key}` not found in workflow `{}`", self.workflow.id)
            }
        }
    }

    pub fn resolve_status(&self, status_key: &str) -> Result<LifecycleStatusProjection> {
        let status = self.status(status_key)?;
        Ok(LifecycleStatusProjection {
            status_key: status.key.clone(),
            status_role: status.role,
            runtime_status: runtime_status_for_role(status.role)?,
        })
    }

    pub fn resolve_outcome(
        &self,
        from_status_key: &str,
        outcome: &str,
    ) -> Result<LifecycleStatusProjection> {
        let target_status_key = self.resolve_outcome_target_status_key(from_status_key, outcome)?;
        self.resolve_status(&target_status_key)
    }

    /// 根据当前状态和动作结果解析唯一目标状态键。
    pub fn resolve_outcome_target_status_key(
        &self,
        from_status_key: &str,
        outcome: &str,
    ) -> Result<String> {
        let status = self.status(from_status_key)?;
        let outcome = required_text(outcome, "workflow transition outcome")?;
        let target = status
            .transitions
            .iter()
            .find(|transition| workflow_transition_matches_outcome(&transition.outcome, outcome))
            .map(|transition| transition.target.clone())
            .or_else(|| outcome_is_success(outcome).then(|| status.next.clone()).flatten());

        let Some(target) = target else {
            bail!("workflow status `{}` has no transition for outcome `{outcome}`", status.key);
        };
        self.next_enabled_status(target)
    }

    /// 判断状态角色是否属于可派发阶段，并保留投影供审计。
    pub fn dispatch_eligibility(&self, status_key: &str) -> Result<LifecycleDispatchEligibility> {
        let status = self.resolve_status(status_key)?;
        let eligible = matches!(
            status.status_role,
            TaskPoolWorkflowStatusRoleDto::Queued | TaskPoolWorkflowStatusRoleDto::Planning
        ) && status.runtime_status == AgentTaskStatus::Queued;
        Ok(LifecycleDispatchEligibility { status, eligible })
    }

    pub fn projection_metadata(&self, status_key: &str) -> Result<Map<String, Value>> {
        let status = self.resolve_status(status_key)?;
        let mut metadata = Map::new();
        metadata.insert("task_board.status_key".to_string(), json!(status.status_key));
        metadata.insert(
            "task_board.status_role".to_string(),
            json!(status_role_key(status.status_role)),
        );
        Ok(metadata)
    }

    fn find_status(&self, status_key: &str) -> Option<&'a TaskPoolWorkflowStatusDto> {
        self.workflow.statuses.iter().find(|status| status.key == status_key)
    }

    fn next_enabled_status(&self, status_key: String) -> Result<String> {
        let mut current = status_key;
        let mut visited = HashSet::new();
        loop {
            let Some(status) = self.find_status(&current) else {
                bail!(
                    "workflow status `{current}` not found while resolving enabled outcome target"
                );
            };
            if status.enabled {
                return Ok(current);
            }
            if !visited.insert(current.clone()) {
                bail!("workflow disabled status chain contains a cycle at `{current}`");
            }

            let Some(next) = success_target_or_next(status) else {
                bail!("workflow status `{}` is disabled and has no enabled successor", status.key);
            };
            current = next.to_string();
        }
    }
}

pub fn runtime_status_for_role(role: TaskPoolWorkflowStatusRoleDto) -> Result<AgentTaskStatus> {
    match role {
        TaskPoolWorkflowStatusRoleDto::Backlog => Ok(AgentTaskStatus::Draft),
        TaskPoolWorkflowStatusRoleDto::Queued | TaskPoolWorkflowStatusRoleDto::Planning => {
            Ok(AgentTaskStatus::Queued)
        }
        TaskPoolWorkflowStatusRoleDto::Executing => Ok(AgentTaskStatus::Running),
        TaskPoolWorkflowStatusRoleDto::Failed => Ok(AgentTaskStatus::Failed),
        TaskPoolWorkflowStatusRoleDto::Paused => Ok(AgentTaskStatus::Paused),
        TaskPoolWorkflowStatusRoleDto::ReviewReady
        | TaskPoolWorkflowStatusRoleDto::Review
        | TaskPoolWorkflowStatusRoleDto::Merging => Ok(AgentTaskStatus::Submitted),
        TaskPoolWorkflowStatusRoleDto::Done => Ok(AgentTaskStatus::Completed),
        TaskPoolWorkflowStatusRoleDto::Archived => Ok(AgentTaskStatus::Archived),
        TaskPoolWorkflowStatusRoleDto::Custom => {
            bail!("workflow status role custom cannot map to runtime agent status")
        }
    }
}

pub fn status_role_key(role: TaskPoolWorkflowStatusRoleDto) -> &'static str {
    match role {
        TaskPoolWorkflowStatusRoleDto::Backlog => "backlog",
        TaskPoolWorkflowStatusRoleDto::Queued => "queued",
        TaskPoolWorkflowStatusRoleDto::Planning => "planning",
        TaskPoolWorkflowStatusRoleDto::Executing => "executing",
        TaskPoolWorkflowStatusRoleDto::Failed => "failed",
        TaskPoolWorkflowStatusRoleDto::Paused => "paused",
        TaskPoolWorkflowStatusRoleDto::ReviewReady => "review_ready",
        TaskPoolWorkflowStatusRoleDto::Review => "review",
        TaskPoolWorkflowStatusRoleDto::Merging => "merging",
        TaskPoolWorkflowStatusRoleDto::Done => "done",
        TaskPoolWorkflowStatusRoleDto::Archived => "archived",
        TaskPoolWorkflowStatusRoleDto::Custom => "custom",
    }
}

fn success_target_or_next(status: &TaskPoolWorkflowStatusDto) -> Option<&str> {
    status
        .transitions
        .iter()
        .find(|transition| outcome_is_success(&transition.outcome))
        .map(|transition| transition.target.as_str())
        .or(status.next.as_deref())
}

fn workflow_transition_matches_outcome(transition_outcome: &str, outcome: &str) -> bool {
    let transition_outcome = normalized_outcome(transition_outcome);
    let outcome = normalized_outcome(outcome);
    match outcome.as_str() {
        "success" | "succeed" => matches!(transition_outcome.as_str(), "success" | "succeed"),
        "failure" | "fail" | "failed" => {
            matches!(transition_outcome.as_str(), "failure" | "fail" | "failed")
        }
        "blocked" | "block" => matches!(transition_outcome.as_str(), "blocked" | "block"),
        _ => transition_outcome == outcome,
    }
}

fn outcome_is_success(outcome: &str) -> bool {
    matches!(normalized_outcome(outcome).as_str(), "success" | "succeed")
}

fn normalized_outcome(outcome: &str) -> String {
    outcome.trim().to_ascii_lowercase()
}

fn required_text<'a>(value: &'a str, field: &str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        bail!("{field} must not be empty");
    }
    Ok(value)
}
