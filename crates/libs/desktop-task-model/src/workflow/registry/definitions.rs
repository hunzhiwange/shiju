//! 工作流快照、结果分支与可持久化状态定义。
use std::collections::HashSet;
use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use super::{StatusRole, TaskStatusDefinition, TaskWorkflowDefinition};
use crate::models::TaskStatus;

/// 状态完成后的分支结果 key。
///
/// 内置结果 key 为 `succeed` 与 `failed`，其余业务结果（如 `manual_review`、
/// `platform_rejected`）可由用户自定义。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StatusOutcome {
    key: String,
}

impl StatusOutcome {
    pub fn new(key: impl AsRef<str>) -> Self {
        let key = Self::canonical_key(key.as_ref().trim());
        Self { key: key.to_string() }
    }

    pub fn succeed() -> Self {
        Self::new("succeed")
    }

    pub fn failed() -> Self {
        Self::new("failed")
    }

    pub fn manual() -> Self {
        Self::new("manual")
    }

    pub fn retry() -> Self {
        Self::new("retry")
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn label(&self) -> String {
        match self.key.as_str() {
            "succeed" => "成功后".to_string(),
            "failed" => "失败后".to_string(),
            "manual" => "人工/例外后".to_string(),
            "retry" => "重试后".to_string(),
            key => format!("{key} 后"),
        }
    }

    pub fn is_succeed(&self) -> bool {
        self.key == "succeed"
    }

    pub fn is_failed(&self) -> bool {
        self.key == "failed"
    }

    pub fn is_retry(&self) -> bool {
        self.key == "retry"
    }

    fn canonical_key(key: &str) -> &str {
        match key {
            "success" => "succeed",
            "failure" => "failed",
            "other" => "manual",
            key => key,
        }
    }
}

impl Serialize for StatusOutcome {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.key())
    }
}

impl<'de> Deserialize<'de> for StatusOutcome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let key = String::deserialize(deserializer)?;
        Ok(Self::new(key))
    }
}

/// 单个状态在某个结果下的目标状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedStatusTransition {
    pub outcome: StatusOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub target: TaskStatus,
}

impl OwnedStatusTransition {
    pub fn display_label(&self) -> String {
        self.label
            .as_deref()
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| self.outcome.label())
    }
}

/// Owned 状态定义，用于用户自定义工作流的运行时注册与 JSON 持久化。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedStatusDefinition {
    pub status: TaskStatus,
    pub key: String,
    pub label: String,
    pub role: StatusRole,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
    #[serde(default)]
    pub is_terminal: bool,
    #[serde(default)]
    pub auto_execution_candidate: bool,
    #[serde(default)]
    pub occupies_execution_slot: bool,
    #[serde(default)]
    pub is_scheduling_backlog: bool,
    #[serde(default)]
    pub next: Option<TaskStatus>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transitions: Vec<OwnedStatusTransition>,
}

impl OwnedStatusDefinition {
    pub fn role(&self) -> StatusRole {
        self.role
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn transition_target(&self, outcome: StatusOutcome) -> Option<TaskStatus> {
        self.transitions
            .iter()
            .find(|transition| transition.outcome == outcome)
            .map(|transition| transition.target)
            .or_else(|| outcome.is_succeed().then_some(self.next).flatten())
    }

    pub fn outgoing_transitions(&self) -> Vec<OwnedStatusTransition> {
        let mut transitions = self.transitions.clone();
        if let Some(next) = self.next
            && !transitions.iter().any(|transition| transition.outcome.is_succeed())
        {
            transitions.insert(
                0,
                OwnedStatusTransition {
                    outcome: StatusOutcome::succeed(),
                    label: None,
                    target: next,
                },
            );
        }
        transitions
    }

    pub fn has_outgoing_transitions(&self) -> bool {
        self.next.is_some() || !self.transitions.is_empty()
    }
}

impl From<&TaskStatusDefinition> for OwnedStatusDefinition {
    fn from(definition: &TaskStatusDefinition) -> Self {
        let transitions = match (definition.role, definition.next) {
            (StatusRole::Failed, Some(target)) => vec![OwnedStatusTransition {
                outcome: StatusOutcome::retry(),
                label: Some("重试".to_string()),
                target,
            }],
            _ => Vec::new(),
        };
        Self {
            status: definition.status,
            key: definition.key.to_string(),
            label: definition.label.to_string(),
            role: definition.role,
            enabled: true,
            is_terminal: definition.is_terminal,
            auto_execution_candidate: definition.auto_execution_candidate,
            occupies_execution_slot: definition.occupies_execution_slot,
            is_scheduling_backlog: definition.is_scheduling_backlog,
            next: definition.next,
            transitions,
        }
    }
}

/// Owned 工作流定义，用于用户预设。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedTaskWorkflow {
    pub id: String,
    pub label: String,
    pub statuses: Vec<OwnedStatusDefinition>,
}

/// 工作流查询快照。内置与用户预设都会转换为该结构，供状态机和 UI 一致读取。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskWorkflow {
    pub id: String,
    pub label: String,
    pub statuses: Vec<OwnedStatusDefinition>,
}

impl TaskWorkflow {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn statuses(&self) -> &[OwnedStatusDefinition] {
        &self.statuses
    }

    pub fn status_definition(&self, status: TaskStatus) -> Option<&OwnedStatusDefinition> {
        self.statuses.iter().find(|definition| definition.status == status)
    }

    pub fn next_status(&self, status: TaskStatus) -> Option<TaskStatus> {
        self.next_status_for_outcome(status, StatusOutcome::succeed())
    }

    pub fn next_status_for_outcome(
        &self,
        status: TaskStatus,
        outcome: StatusOutcome,
    ) -> Option<TaskStatus> {
        let target = self.status_definition(status)?.transition_target(outcome)?;
        self.next_enabled_status(target)
    }

    pub fn status_transitions(&self, status: TaskStatus) -> Vec<OwnedStatusTransition> {
        self.status_definition(status)
            .map(OwnedStatusDefinition::outgoing_transitions)
            .unwrap_or_default()
    }

    pub fn status_role(&self, status: TaskStatus) -> Option<StatusRole> {
        self.status_definition(status).map(|definition| definition.role)
    }

    pub fn status_enabled(&self, status: TaskStatus) -> bool {
        self.status_definition(status).is_some_and(|definition| definition.enabled)
    }

    pub fn occupies_execution_slot(&self, status: TaskStatus) -> bool {
        self.status_definition(status)
            .is_some_and(|definition| definition.enabled && definition.occupies_execution_slot)
    }

    pub fn is_scheduling_backlog(&self, status: TaskStatus) -> bool {
        self.status_definition(status)
            .is_some_and(|definition| definition.enabled && definition.is_scheduling_backlog)
    }

    pub fn role_occupies_execution_slot(&self, status: TaskStatus) -> bool {
        self.status_enabled(status)
            && matches!(
                self.status_role(status),
                Some(StatusRole::Executing | StatusRole::Planning)
            )
    }

    pub fn role_is_scheduling_backlog(&self, status: TaskStatus) -> bool {
        self.status_enabled(status)
            && matches!(
                self.status_role(status),
                Some(StatusRole::Backlog | StatusRole::Queued | StatusRole::Planning)
            )
    }

    pub fn role_is_retry_candidate(&self, status: TaskStatus) -> bool {
        self.status_enabled(status) && matches!(self.status_role(status), Some(StatusRole::Failed))
    }

    pub fn role_is_timeout_monitored(&self, status: TaskStatus) -> bool {
        self.status_enabled(status)
            && matches!(self.status_role(status), Some(StatusRole::Executing))
    }

    pub(crate) fn next_enabled_status(&self, status: TaskStatus) -> Option<TaskStatus> {
        let mut current = status;
        let mut visited = HashSet::new();
        loop {
            let definition = self.status_definition(current)?;
            if definition.enabled {
                return Some(current);
            }
            if !visited.insert(current) {
                return None;
            }
            current = definition.transition_target(StatusOutcome::succeed())?;
        }
    }
}

impl From<&TaskWorkflowDefinition> for TaskWorkflow {
    fn from(workflow: &TaskWorkflowDefinition) -> Self {
        Self {
            id: workflow.id.to_string(),
            label: workflow.label.to_string(),
            statuses: workflow.statuses.iter().map(OwnedStatusDefinition::from).collect(),
        }
    }
}

impl From<OwnedTaskWorkflow> for TaskWorkflow {
    fn from(workflow: OwnedTaskWorkflow) -> Self {
        Self { id: workflow.id, label: workflow.label, statuses: workflow.statuses }
    }
}

impl From<&OwnedTaskWorkflow> for TaskWorkflow {
    fn from(workflow: &OwnedTaskWorkflow) -> Self {
        Self {
            id: workflow.id.clone(),
            label: workflow.label.clone(),
            statuses: workflow.statuses.clone(),
        }
    }
}

fn is_true(value: &bool) -> bool {
    *value
}

fn default_true() -> bool {
    true
}
