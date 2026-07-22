use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};
use vw_api_types::task_lifecycle::{TaskLifecycleStatusDto, TaskLifecycleWorkflowDto};

use super::builtin::{built_in_workflow_by_id, built_in_workflows, is_built_in_workflow_id};

#[derive(Debug, Clone, Default)]
pub struct TaskLifecycleRegistry {
    custom_workflows: Vec<TaskLifecycleWorkflowDto>,
}

impl TaskLifecycleRegistry {
    pub fn new() -> Self {
        Self { custom_workflows: Vec::new() }
    }

    pub fn from_custom_workflows(workflows: Vec<TaskLifecycleWorkflowDto>) -> Result<Self> {
        let mut registry = Self::new();
        registry.replace_custom_workflows(workflows)?;
        Ok(registry)
    }

    pub fn register(&mut self, mut workflow: TaskLifecycleWorkflowDto) -> Result<()> {
        validate_custom_workflow(&workflow)?;
        if self.custom_workflows.iter().any(|existing| existing.id == workflow.id) {
            bail!("lifecycle workflow id already exists: {}", workflow.id);
        }
        workflow.built_in = is_built_in_workflow_id(workflow.id.as_str());
        self.custom_workflows.push(workflow);
        Ok(())
    }

    pub fn replace_custom_workflows(
        &mut self,
        workflows: Vec<TaskLifecycleWorkflowDto>,
    ) -> Result<()> {
        let mut seen = HashSet::new();
        for workflow in &workflows {
            validate_custom_workflow(workflow)?;
            if !seen.insert(workflow.id.as_str()) {
                bail!("lifecycle workflow id duplicated: {}", workflow.id);
            }
        }

        self.custom_workflows = workflows
            .into_iter()
            .map(|mut workflow| {
                workflow.built_in = is_built_in_workflow_id(workflow.id.as_str());
                workflow
            })
            .collect();
        Ok(())
    }

    pub fn workflow_by_id(&self, id: &str) -> Option<TaskLifecycleWorkflowDto> {
        self.custom_workflows
            .iter()
            .find(|workflow| workflow.id == id)
            .cloned()
            .or_else(|| built_in_workflow_by_id(id))
    }

    pub fn all_workflows(&self) -> Vec<TaskLifecycleWorkflowDto> {
        let mut workflows = built_in_workflows()
            .into_iter()
            .map(|built_in| {
                self.custom_workflows
                    .iter()
                    .find(|workflow| workflow.id == built_in.id)
                    .cloned()
                    .unwrap_or(built_in)
            })
            .collect::<Vec<_>>();
        workflows.extend(
            self.custom_workflows
                .iter()
                .filter(|workflow| !is_built_in_workflow_id(workflow.id.as_str()))
                .cloned(),
        );
        workflows
    }

    pub fn custom_workflows(&self) -> &[TaskLifecycleWorkflowDto] {
        &self.custom_workflows
    }
}

fn validate_custom_workflow(workflow: &TaskLifecycleWorkflowDto) -> Result<()> {
    validate_workflow_id(workflow.id.as_str())?;
    if workflow.label.trim().is_empty() {
        bail!("custom lifecycle workflow label must not be empty: {}", workflow.id);
    }
    if workflow.statuses.is_empty() {
        bail!("custom lifecycle workflow must contain at least one status: {}", workflow.id);
    }

    workflow.validate().map_err(anyhow::Error::msg)?;

    let status_by_key: HashMap<&str, &TaskLifecycleStatusDto> =
        workflow.statuses.iter().map(|status| (status.key.as_str(), status)).collect();
    for status in &workflow.statuses {
        validate_status(workflow.id.as_str(), status)?;
    }
    for status in &workflow.statuses {
        reject_next_cycle(status.key.as_str(), &status_by_key)?;
    }

    Ok(())
}

fn validate_workflow_id(id: &str) -> Result<()> {
    if id.trim().is_empty() {
        bail!("custom lifecycle workflow id must not be empty");
    }
    if id.trim() != id {
        bail!("custom lifecycle workflow id must not contain surrounding whitespace: {id}");
    }
    if !is_valid_key(id) {
        bail!("custom lifecycle workflow id invalid: {id}");
    }
    Ok(())
}

fn validate_status(workflow_id: &str, status: &TaskLifecycleStatusDto) -> Result<()> {
    if !is_valid_key(status.key.as_str()) {
        bail!("lifecycle workflow status key invalid: {}", status.key);
    }
    if status.label.trim().is_empty() {
        bail!("lifecycle workflow status label must not be empty: {}:{}", workflow_id, status.key);
    }

    let has_outgoing = status.next.is_some() || !status.transitions.is_empty();
    if status.terminal == has_outgoing {
        bail!(
            "lifecycle workflow status {} terminal flag must match outgoing transitions",
            status.key
        );
    }

    let mut outcomes = HashSet::new();
    for transition in &status.transitions {
        if !is_valid_key(transition.outcome.as_str()) {
            bail!(
                "lifecycle workflow transition outcome invalid: {}:{}",
                status.key,
                transition.outcome
            );
        }
        if !outcomes.insert(transition.outcome.as_str()) {
            bail!(
                "lifecycle workflow status {} transition outcome duplicated: {}",
                status.key,
                transition.outcome
            );
        }
    }

    if let Some(success_target) = status
        .transitions
        .iter()
        .find(|transition| transition.outcome == "succeed")
        .map(|transition| transition.target.as_str())
        && status.next.as_deref().is_some_and(|next| next != success_target)
    {
        bail!("lifecycle workflow status {} next must match succeed transition", status.key);
    }

    Ok(())
}

fn reject_next_cycle(
    start: &str,
    status_by_key: &HashMap<&str, &TaskLifecycleStatusDto>,
) -> Result<()> {
    let mut visited = HashSet::new();
    let mut current = start;
    while let Some(status) = status_by_key.get(current) {
        let Some(next) = status.next.as_deref() else {
            return Ok(());
        };
        if !visited.insert(current) {
            bail!("lifecycle workflow cycle next chain: {start}");
        }
        current = next;
    }
    Ok(())
}

fn is_valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.trim() == key
        && key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}
