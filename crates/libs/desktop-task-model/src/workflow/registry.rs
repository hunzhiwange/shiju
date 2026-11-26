use std::collections::HashSet;
use std::sync::{LazyLock, RwLock};
#[cfg(any(test, feature = "test-support"))]
use std::sync::{Mutex, MutexGuard};

use anyhow::{Result, anyhow, bail};
#[path = "registry/definitions.rs"]
mod definitions;
pub use definitions::{
    OwnedStatusDefinition, OwnedStatusTransition, OwnedTaskWorkflow, StatusOutcome, TaskWorkflow,
};
use vw_api_task_lifecycle::{
    TaskLifecycleStatusCategoryDto as Category, TaskLifecycleStatusDto, TaskLifecycleWorkflowDto,
};

use super::{StatusRole, builtin_task_workflows};
use crate::models::TaskStatus;

pub(crate) fn task_workflow_from_lifecycle_dto(
    workflow: &TaskLifecycleWorkflowDto,
) -> Result<TaskWorkflow> {
    workflow.validate().map_err(|err| anyhow!("生命周期工作流 {} 无效: {err}", workflow.id))?;
    // 展示模型保留分类，运行状态和结果分支不从步骤配置推导。
    let statuses = workflow
        .statuses
        .iter()
        .map(|status| {
            Ok(OwnedStatusDefinition {
                status: TaskStatus::parse_key(&status.key)
                    .ok_or_else(|| anyhow!("empty workflow status key"))?,
                key: status.key.clone(),
                label: status.label.clone(),
                role: match status.category {
                    Category::Unstarted => StatusRole::Backlog,
                    Category::Started => StatusRole::Custom,
                    Category::Done => StatusRole::Done,
                    Category::Closed => StatusRole::Archived,
                },
                enabled: status.enabled,
                is_terminal: status.category.is_terminal(),
                auto_execution_candidate: false,
                occupies_execution_slot: false,
                is_scheduling_backlog: false,
                next: None,
                transitions: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(TaskWorkflow {
        id: workflow.id.clone(),
        label: workflow.label.clone(),
        statuses,
        initial_status_key: workflow.initial_status_key.clone(),
        steps: workflow.steps.clone(),
    })
}

pub fn lifecycle_dto_from_task_workflow(
    workflow: TaskWorkflow,
    built_in: bool,
) -> TaskLifecycleWorkflowDto {
    TaskLifecycleWorkflowDto {
        id: workflow.id,
        label: workflow.label,
        built_in,
        initial_status_key: workflow.initial_status_key,
        steps: workflow.steps,
        statuses: workflow
            .statuses
            .iter()
            .map(|status| TaskLifecycleStatusDto {
                key: status.key.clone(),
                label: status.label.clone(),
                category: status_category(status.role),
                enabled: status.enabled,
            })
            .collect(),
    }
}

pub(crate) fn status_category(role: StatusRole) -> Category {
    match role {
        StatusRole::Backlog | StatusRole::Queued => Category::Unstarted,
        StatusRole::Done => Category::Done,
        StatusRole::Archived => Category::Closed,
        _ => Category::Started,
    }
}

#[cfg(any(test, feature = "test-support"))]
fn lifecycle_dto_from_owned_workflow(workflow: OwnedTaskWorkflow) -> TaskLifecycleWorkflowDto {
    lifecycle_dto_from_task_workflow(TaskWorkflow::from(workflow), false)
}

fn built_in_task_lifecycle_workflows() -> Vec<TaskLifecycleWorkflowDto> {
    builtin_task_workflows()
        .iter()
        .map(|workflow| lifecycle_dto_from_task_workflow(TaskWorkflow::from(*workflow), true))
        .collect()
}

/// 运行时工作流注册表。默认总是包含内置工作流，自定义预设来自网关生命周期 cache。
#[derive(Debug, Clone, Default)]
pub struct TaskWorkflowRegistry {
    custom_workflows: Vec<OwnedTaskWorkflow>,
}

impl TaskWorkflowRegistry {
    pub fn new() -> Self {
        Self { custom_workflows: Vec::new() }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn register(&mut self, workflow: OwnedTaskWorkflow) -> Result<()> {
        validate_owned_workflow(&workflow)?;
        if self.custom_workflows.iter().any(|existing| existing.id == workflow.id) {
            bail!("自定义工作流 id 已存在: {}", workflow.id);
        }
        self.custom_workflows.push(workflow);
        Ok(())
    }

    pub fn replace_custom_workflows(&mut self, workflows: Vec<OwnedTaskWorkflow>) -> Result<()> {
        let mut seen = HashSet::new();
        for workflow in &workflows {
            validate_owned_workflow(workflow)?;
            if !seen.insert(workflow.id.as_str()) {
                bail!("自定义工作流 id 重复: {}", workflow.id);
            }
        }
        self.custom_workflows = workflows;
        Ok(())
    }

    pub fn workflow_by_id(&self, id: &str) -> Option<TaskWorkflow> {
        self.custom_workflows
            .iter()
            .find(|workflow| workflow.id == id)
            .map(TaskWorkflow::from)
            .or_else(|| {
                builtin_task_workflows()
                    .iter()
                    .find(|workflow| workflow.id == id)
                    .map(|workflow| TaskWorkflow::from(*workflow))
            })
    }

    pub fn all_workflows(&self) -> Vec<TaskWorkflow> {
        let mut workflows: Vec<TaskWorkflow> = builtin_task_workflows()
            .iter()
            .map(|built_in| {
                self.custom_workflows
                    .iter()
                    .find(|workflow| workflow.id == built_in.id)
                    .map(TaskWorkflow::from)
                    .unwrap_or_else(|| TaskWorkflow::from(*built_in))
            })
            .collect();
        workflows.extend(
            self.custom_workflows
                .iter()
                .filter(|workflow| {
                    !builtin_task_workflows().iter().any(|built_in| built_in.id == workflow.id)
                })
                .map(TaskWorkflow::from),
        );
        workflows
    }
}

pub fn task_workflow_by_id(id: &str) -> Option<TaskWorkflow> {
    let id = id.trim();
    runtime_task_lifecycle_cache_snapshot()
        .into_iter()
        .find(|workflow| workflow.id == id)
        .and_then(|workflow| task_workflow_from_lifecycle_dto(&workflow).ok())
}

pub fn all_task_workflows() -> Vec<TaskWorkflow> {
    runtime_task_lifecycle_cache_snapshot()
        .into_iter()
        .filter_map(|workflow| match task_workflow_from_lifecycle_dto(&workflow) {
            Ok(workflow) => Some(workflow),
            Err(err) => {
                tracing::warn!(
                    target: "vw_desktop",
                    error = %err,
                    "rejected invalid cached task lifecycle workflow"
                );
                None
            }
        })
        .collect()
}

pub fn is_builtin_task_workflow_id(id: &str) -> bool {
    let id = id.trim();
    runtime_task_lifecycle_cache_snapshot()
        .into_iter()
        .find(|workflow| workflow.id == id)
        .map(|workflow| workflow.built_in)
        .unwrap_or_else(|| builtin_task_workflows().iter().any(|workflow| workflow.id == id))
}

pub fn update_runtime_task_lifecycle_cache(workflows: Vec<TaskLifecycleWorkflowDto>) -> Result<()> {
    validate_lifecycle_cache(&workflows)?;
    let mut cache =
        RUNTIME_TASK_LIFECYCLE_CACHE.write().map_err(|_| anyhow!("任务生命周期 cache 锁已损坏"))?;
    *cache = workflows;
    Ok(())
}

#[cfg(any(test, feature = "test-support"))]
pub fn reset_runtime_task_lifecycle_cache() -> Result<()> {
    update_runtime_task_lifecycle_cache(built_in_task_lifecycle_workflows())
}

#[cfg(any(test, feature = "test-support"))]
pub fn register_runtime_task_workflow(workflow: OwnedTaskWorkflow) -> Result<()> {
    let mut workflows = runtime_task_lifecycle_cache_snapshot();
    workflows.push(lifecycle_dto_from_owned_workflow(workflow));
    update_runtime_task_lifecycle_cache(workflows)
}

#[cfg(any(test, feature = "test-support"))]
pub fn replace_runtime_task_lifecycle_workflows_for_tests(
    workflows: Vec<OwnedTaskWorkflow>,
) -> Result<()> {
    let mut lifecycle_workflows = built_in_task_lifecycle_workflows();
    lifecycle_workflows.extend(workflows.into_iter().map(lifecycle_dto_from_owned_workflow));
    update_runtime_task_lifecycle_cache(lifecycle_workflows)
}

fn runtime_task_lifecycle_cache_snapshot() -> Vec<TaskLifecycleWorkflowDto> {
    RUNTIME_TASK_LIFECYCLE_CACHE
        .read()
        .map(|cache| cache.clone())
        .unwrap_or_else(|_| built_in_task_lifecycle_workflows())
}

fn validate_lifecycle_cache(workflows: &[TaskLifecycleWorkflowDto]) -> Result<()> {
    if workflows.is_empty() {
        bail!("任务生命周期 cache 不能为空");
    }

    let mut ids = HashSet::new();
    for workflow in workflows {
        workflow.validate().map_err(|err| anyhow!("生命周期工作流 {} 无效: {err}", workflow.id))?;
        let id = workflow.id.trim();
        if id != workflow.id {
            bail!("生命周期工作流 id 不能包含首尾空白: {}", workflow.id);
        }
        if !ids.insert(id) {
            bail!("生命周期工作流 id 重复: {id}");
        }
        task_workflow_from_lifecycle_dto(workflow)?;
    }
    Ok(())
}

fn validate_owned_workflow(workflow: &OwnedTaskWorkflow) -> Result<()> {
    lifecycle_dto_from_task_workflow(TaskWorkflow::from(workflow), false)
        .validate()
        .map_err(anyhow::Error::msg)
}

static RUNTIME_TASK_LIFECYCLE_CACHE: LazyLock<RwLock<Vec<TaskLifecycleWorkflowDto>>> =
    LazyLock::new(|| RwLock::new(built_in_task_lifecycle_workflows()));

#[cfg(any(test, feature = "test-support"))]
static RUNTIME_TASK_LIFECYCLE_CACHE_TEST_LOCK: LazyLock<Mutex<()>> =
    LazyLock::new(|| Mutex::new(()));

#[cfg(any(test, feature = "test-support"))]
pub fn runtime_task_lifecycle_cache_test_lock() -> MutexGuard<'static, ()> {
    RUNTIME_TASK_LIFECYCLE_CACHE_TEST_LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
}
