use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, RwLock};
#[cfg(any(test, feature = "test-support"))]
use std::sync::{Mutex, MutexGuard};

use anyhow::{Result, anyhow, bail};
mod definitions;
pub use definitions::{
    OwnedStatusDefinition, OwnedStatusTransition, OwnedTaskWorkflow, StatusOutcome, TaskWorkflow,
};
use vw_api_task_lifecycle::{
    TaskLifecycleStatusDto, TaskLifecycleStatusRoleDto, TaskLifecycleStatusTransitionDto,
    TaskLifecycleWorkflowDto,
};

use super::{StatusRole, TaskStatusDefinition, TaskWorkflowDefinition, builtin_task_workflows};
use crate::models::TaskStatus;

fn task_workflow_from_lifecycle_dto(workflow: &TaskLifecycleWorkflowDto) -> Result<TaskWorkflow> {
    workflow.validate().map_err(|err| anyhow!("生命周期工作流 {} 无效: {err}", workflow.id))?;
    let statuses = workflow
        .statuses
        .iter()
        .map(|status| owned_status_from_lifecycle_dto(&workflow.id, status))
        .collect::<Result<Vec<_>>>()?;
    Ok(TaskWorkflow { id: workflow.id.clone(), label: workflow.label.clone(), statuses })
}

fn owned_status_from_lifecycle_dto(
    workflow_id: &str,
    status: &TaskLifecycleStatusDto,
) -> Result<OwnedStatusDefinition> {
    let task_status = task_status_from_lifecycle_key(workflow_id, "状态", &status.key)?;
    let next = status
        .next
        .as_deref()
        .map(|key| task_status_from_lifecycle_key(workflow_id, "next", key))
        .transpose()?;
    let transitions = status
        .transitions
        .iter()
        .map(|transition| {
            Ok(OwnedStatusTransition {
                outcome: StatusOutcome::new(&transition.outcome),
                label: transition.label.clone(),
                target: task_status_from_lifecycle_key(
                    workflow_id,
                    "transition",
                    &transition.target,
                )?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(OwnedStatusDefinition {
        status: task_status,
        key: status.key.clone(),
        label: status.label.clone(),
        role: status_role_from_lifecycle_dto(status.role),
        enabled: status.enabled,
        is_terminal: status.terminal,
        auto_execution_candidate: status.auto_execution_candidate,
        occupies_execution_slot: status.occupies_execution_slot,
        is_scheduling_backlog: status.is_scheduling_backlog,
        next,
        transitions,
    })
}

fn task_status_from_lifecycle_key(workflow_id: &str, field: &str, key: &str) -> Result<TaskStatus> {
    TaskStatus::parse_key(key)
        .ok_or_else(|| anyhow!("生命周期工作流 {workflow_id} {field} key 不能为空"))
}

fn lifecycle_dto_from_task_workflow(
    workflow: TaskWorkflow,
    built_in: bool,
) -> TaskLifecycleWorkflowDto {
    let statuses = workflow.statuses;
    TaskLifecycleWorkflowDto {
        id: workflow.id,
        label: workflow.label,
        built_in,
        statuses: statuses
            .iter()
            .map(|status| TaskLifecycleStatusDto {
                key: status.key.clone(),
                label: status.label.clone(),
                role: status_role_to_lifecycle_dto(status.role),
                enabled: status.enabled,
                terminal: status.is_terminal,
                auto_execution_candidate: status.auto_execution_candidate,
                occupies_execution_slot: status.occupies_execution_slot,
                is_scheduling_backlog: status.is_scheduling_backlog,
                next: status.next.map(|target| lifecycle_status_key_for_target(&statuses, target)),
                transitions: status
                    .transitions
                    .iter()
                    .map(|transition| TaskLifecycleStatusTransitionDto {
                        outcome: transition.outcome.key().to_string(),
                        label: transition.label.clone(),
                        target: lifecycle_status_key_for_target(&statuses, transition.target),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn lifecycle_status_key_for_target(
    statuses: &[OwnedStatusDefinition],
    target: TaskStatus,
) -> String {
    statuses
        .iter()
        .find(|definition| definition.status == target)
        .map(|definition| definition.key.clone())
        .unwrap_or_else(|| target.to_string_key().to_string())
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

fn status_role_from_lifecycle_dto(role: TaskLifecycleStatusRoleDto) -> StatusRole {
    match role {
        TaskLifecycleStatusRoleDto::Backlog => StatusRole::Backlog,
        TaskLifecycleStatusRoleDto::Queued => StatusRole::Queued,
        TaskLifecycleStatusRoleDto::Planning => StatusRole::Planning,
        TaskLifecycleStatusRoleDto::Executing => StatusRole::Executing,
        TaskLifecycleStatusRoleDto::Failed => StatusRole::Failed,
        TaskLifecycleStatusRoleDto::Paused => StatusRole::Paused,
        TaskLifecycleStatusRoleDto::ReviewReady => StatusRole::ReviewReady,
        TaskLifecycleStatusRoleDto::Review => StatusRole::Review,
        TaskLifecycleStatusRoleDto::Merging => StatusRole::Merging,
        TaskLifecycleStatusRoleDto::Done => StatusRole::Done,
        TaskLifecycleStatusRoleDto::Archived => StatusRole::Archived,
        TaskLifecycleStatusRoleDto::Custom => StatusRole::Custom,
    }
}

fn status_role_to_lifecycle_dto(role: StatusRole) -> TaskLifecycleStatusRoleDto {
    match role {
        StatusRole::Backlog => TaskLifecycleStatusRoleDto::Backlog,
        StatusRole::Queued => TaskLifecycleStatusRoleDto::Queued,
        StatusRole::Planning => TaskLifecycleStatusRoleDto::Planning,
        StatusRole::Executing => TaskLifecycleStatusRoleDto::Executing,
        StatusRole::Failed => TaskLifecycleStatusRoleDto::Failed,
        StatusRole::Paused => TaskLifecycleStatusRoleDto::Paused,
        StatusRole::ReviewReady => TaskLifecycleStatusRoleDto::ReviewReady,
        StatusRole::Review => TaskLifecycleStatusRoleDto::Review,
        StatusRole::Merging => TaskLifecycleStatusRoleDto::Merging,
        StatusRole::Done => TaskLifecycleStatusRoleDto::Done,
        StatusRole::Archived => TaskLifecycleStatusRoleDto::Archived,
        StatusRole::Custom => TaskLifecycleStatusRoleDto::Custom,
    }
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
    let id = workflow.id.trim();
    if id.is_empty() {
        bail!("自定义工作流 id 不能为空");
    }
    if id != workflow.id {
        bail!("自定义工作流 id 不能包含首尾空白: {}", workflow.id);
    }
    if workflow.label.trim().is_empty() {
        bail!("自定义工作流 label 不能为空: {id}");
    }
    if workflow.statuses.is_empty() {
        bail!("自定义工作流至少需要一个状态: {id}");
    }

    let mut keys = HashSet::new();
    let mut statuses = HashSet::new();
    let mut status_by_key = HashMap::new();

    for definition in &workflow.statuses {
        validate_status_definition(id, definition)?;
        if !keys.insert(definition.key.as_str()) {
            bail!("工作流 {id} 状态 key 重复: {}", definition.key);
        }
        if !statuses.insert(definition.status) {
            bail!("工作流 {id} 状态重复: {:?}", definition.status);
        }
        status_by_key.insert(definition.status, definition.key.as_str());
    }

    for definition in &workflow.statuses {
        if definition.is_terminal == definition.has_outgoing_transitions() {
            bail!("工作流 {id} 状态 {} 的终态标记必须与后续分支一致", definition.key);
        }
        if let Some(next) = definition.next
            && !statuses.contains(&next)
        {
            bail!("工作流 {id} 状态 {} 指向未知 next: {:?}", definition.key, next);
        }
        let mut outcomes = HashSet::new();
        for transition in &definition.transitions {
            if !is_valid_outcome_key(transition.outcome.key()) {
                bail!(
                    "工作流 {id} 状态 {} 的结果 key 非法: {}",
                    definition.key,
                    transition.outcome.key()
                );
            }
            if !outcomes.insert(transition.outcome.key()) {
                bail!(
                    "工作流 {id} 状态 {} 的 {} 分支重复",
                    definition.key,
                    transition.outcome.label()
                );
            }
            if !statuses.contains(&transition.target) {
                bail!(
                    "工作流 {id} 状态 {} 的 {} 分支指向未知状态: {:?}",
                    definition.key,
                    transition.outcome.label(),
                    transition.target
                );
            }
        }
        if let Some(success) = definition
            .transitions
            .iter()
            .find(|transition| transition.outcome.is_succeed())
            .map(|transition| transition.target)
            && definition.next.is_some_and(|next| next != success)
        {
            bail!("工作流 {id} 状态 {} 的 next 必须与成功分支一致", definition.key);
        }
    }

    for definition in &workflow.statuses {
        reject_next_cycle(id, definition.status, &workflow.statuses, &status_by_key)?;
    }

    Ok(())
}

fn validate_status_definition(workflow_id: &str, definition: &OwnedStatusDefinition) -> Result<()> {
    let key = definition.key.as_str();
    if !is_valid_status_key(key) {
        bail!("工作流 {workflow_id} 状态 key 非法: {key}");
    }
    if definition.label.trim().is_empty() {
        bail!("工作流 {workflow_id} 状态 label 不能为空: {key}");
    }
    if TaskStatus::parse_key(key) != Some(definition.status) {
        bail!("工作流 {workflow_id} 状态 key 与 status 不匹配: {key}");
    }
    if let TaskStatus::Custom(custom_key) = definition.status
        && custom_key != key
    {
        bail!("工作流 {workflow_id} 自定义状态 key 不一致: {key}");
    }
    Ok(())
}

fn reject_next_cycle(
    workflow_id: &str,
    start: TaskStatus,
    definitions: &[OwnedStatusDefinition],
    status_by_key: &HashMap<TaskStatus, &str>,
) -> Result<()> {
    let mut visited = HashSet::new();
    let mut current = start;
    while let Some(definition) = definitions.iter().find(|definition| definition.status == current)
    {
        let Some(next) = definition.next else {
            return Ok(());
        };
        if !visited.insert(current) {
            let key = status_by_key.get(&start).copied().unwrap_or("<unknown>");
            bail!("工作流 {workflow_id} 状态 next 链成环: {key}");
        }
        current = next;
    }
    Ok(())
}

fn is_valid_status_key(key: &str) -> bool {
    !key.is_empty()
        && key.trim() == key
        && key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

fn is_valid_outcome_key(key: &str) -> bool {
    is_valid_status_key(key)
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
