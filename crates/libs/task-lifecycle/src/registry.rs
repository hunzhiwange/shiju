//! 任务生命周期工作流注册表及自定义工作流校验。
//!
//! 内置工作流始终保留，调用方只能注册标识、状态目录和交接步骤合法的自定义定义。

use std::collections::HashSet;

use anyhow::{Result, bail};
use vw_api_task_lifecycle::TaskLifecycleWorkflowDto;

use super::builtin::{built_in_workflow_by_id, built_in_workflows, is_built_in_workflow_id};

#[derive(Debug, Clone, Default)]
/// 合并内置与自定义任务生命周期工作流的只读注册表。
pub struct TaskLifecycleRegistry {
    custom_workflows: Vec<TaskLifecycleWorkflowDto>,
}

impl TaskLifecycleRegistry {
    /// 创建不含自定义工作流的注册表。
    pub fn new() -> Self {
        Self { custom_workflows: Vec::new() }
    }

    /// 校验给定自定义工作流集合后创建注册表。
    pub fn from_custom_workflows(workflows: Vec<TaskLifecycleWorkflowDto>) -> Result<Self> {
        let mut registry = Self::new();
        registry.replace_custom_workflows(workflows)?;
        Ok(registry)
    }

    /// 注册单个自定义工作流；重复标识或无效交接步骤会被拒绝。
    pub fn register(&mut self, mut workflow: TaskLifecycleWorkflowDto) -> Result<()> {
        validate_custom_workflow(&workflow)?;
        if self.custom_workflows.iter().any(|existing| existing.id == workflow.id) {
            bail!("lifecycle workflow id already exists: {}", workflow.id);
        }
        workflow.built_in = is_built_in_workflow_id(workflow.id.as_str());
        self.custom_workflows.push(workflow);
        Ok(())
    }

    /// 原子校验并替换全部自定义工作流。
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

    /// 按标识读取工作流，自定义定义优先于同标识内置投影。
    pub fn workflow_by_id(&self, id: &str) -> Option<TaskLifecycleWorkflowDto> {
        self.custom_workflows
            .iter()
            .find(|workflow| workflow.id == id)
            .cloned()
            .or_else(|| built_in_workflow_by_id(id))
    }

    /// 返回内置与自定义工作流的稳定合并列表。
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

    /// 借用当前已校验的自定义工作流集合。
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

fn is_valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.trim() == key
        && key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}
