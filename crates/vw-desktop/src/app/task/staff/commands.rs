//! 员工模式结构化命令。

use crate::app::task::models::{Task, TaskStatus};
use crate::app::task::workflow::{
    DEFAULT_TASK_WORKFLOW_ID, StatusRole, TaskWorkflow, task_workflow_by_id,
};

use super::models::{TaskBoardStaffConfig, TaskBoardStaffMember, TaskBoardStageAction};
use super::stage_capabilities::{
    ADVANCE_STATUS_CAPABILITY_ID, CODE_REVIEW_CAPABILITY_ID, EXECUTE_TASK_CAPABILITY_ID,
    GIT_MERGE_CAPABILITY_ID, MANUAL_CAPABILITY_ID, SAVED_WORKFLOW_CAPABILITY_ID,
};

/// 任务看板员工模式命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskBoardStaffCommand {
    AssignTask { command_id: String, task_id: String, member_id: String },
    TakeOverTask { command_id: String, task_id: String, member_id: String },
    PauseTask { command_id: String, task_id: String, member_id: String },
    ResumeTask { command_id: String, task_id: String, member_id: String },
    StartTask { command_id: String, task_id: String, member_id: String },
    OpenTaskLogs { command_id: String, task_id: String },
    FilterByMember { command_id: String, member_id: String },
}

impl TaskBoardStaffCommand {
    pub fn command_id(&self) -> &str {
        match self {
            TaskBoardStaffCommand::AssignTask { command_id, .. }
            | TaskBoardStaffCommand::TakeOverTask { command_id, .. }
            | TaskBoardStaffCommand::PauseTask { command_id, .. }
            | TaskBoardStaffCommand::ResumeTask { command_id, .. }
            | TaskBoardStaffCommand::StartTask { command_id, .. }
            | TaskBoardStaffCommand::OpenTaskLogs { command_id, .. }
            | TaskBoardStaffCommand::FilterByMember { command_id, .. } => command_id,
        }
    }

    pub fn task_id(&self) -> Option<&str> {
        match self {
            TaskBoardStaffCommand::AssignTask { task_id, .. }
            | TaskBoardStaffCommand::TakeOverTask { task_id, .. }
            | TaskBoardStaffCommand::PauseTask { task_id, .. }
            | TaskBoardStaffCommand::ResumeTask { task_id, .. }
            | TaskBoardStaffCommand::StartTask { task_id, .. }
            | TaskBoardStaffCommand::OpenTaskLogs { task_id, .. } => Some(task_id),
            TaskBoardStaffCommand::FilterByMember { .. } => None,
        }
    }

    pub fn member_id(&self) -> Option<&str> {
        match self {
            TaskBoardStaffCommand::AssignTask { member_id, .. }
            | TaskBoardStaffCommand::TakeOverTask { member_id, .. }
            | TaskBoardStaffCommand::PauseTask { member_id, .. }
            | TaskBoardStaffCommand::ResumeTask { member_id, .. }
            | TaskBoardStaffCommand::StartTask { member_id, .. }
            | TaskBoardStaffCommand::FilterByMember { member_id, .. } => Some(member_id),
            TaskBoardStaffCommand::OpenTaskLogs { .. } => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            TaskBoardStaffCommand::AssignTask { .. } => "派任务",
            TaskBoardStaffCommand::TakeOverTask { .. } => "接管任务",
            TaskBoardStaffCommand::PauseTask { .. } => "暂停任务",
            TaskBoardStaffCommand::ResumeTask { .. } => "继续任务",
            TaskBoardStaffCommand::StartTask { .. } => "开始任务",
            TaskBoardStaffCommand::OpenTaskLogs { .. } => "打开日志",
            TaskBoardStaffCommand::FilterByMember { .. } => "筛选成员",
        }
    }
}

/// 员工命令通过策略校验后允许执行的任务看板动作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskBoardStaffCommandAction {
    AssignTask {
        task_id: String,
        member_id: String,
    },
    ChangeTaskStatus {
        task_id: String,
        new_status: TaskStatus,
    },
    ExecuteTask {
        task_id: String,
    },
    RunCodeReview {
        task_id: String,
    },
    RunMerge {
        task_id: String,
    },
    RunStageWorkflow {
        task_id: String,
        status: TaskStatus,
        workflow_uuid: String,
        member_id: String,
    },
    ViewTaskLogs {
        task_id: String,
    },
    FilterByMember {
        member_id: String,
    },
}

/// 员工命令策略错误，供事件流和 toast 复用同一条反馈。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardStaffCommandError {
    pub message: String,
}

impl TaskBoardStaffCommandError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

pub fn validate_staff_command(
    command: &TaskBoardStaffCommand,
    tasks: &[Task],
    config: &TaskBoardStaffConfig,
    _running_task_ids: &[String],
    workflow_id: &str,
) -> Result<TaskBoardStaffCommandAction, TaskBoardStaffCommandError> {
    let workflow = staff_workflow_for_id(workflow_id).map_err(TaskBoardStaffCommandError::new)?;
    match command {
        TaskBoardStaffCommand::AssignTask { task_id, member_id, .. }
        | TaskBoardStaffCommand::TakeOverTask { task_id, member_id, .. } => {
            let (task, member) = validate_task_member_command(command, tasks, config, &workflow)?;
            ensure_member_capacity(member, task_id)?;
            Ok(TaskBoardStaffCommandAction::AssignTask {
                task_id: task.id.clone(),
                member_id: member_id.clone(),
            })
        }
        TaskBoardStaffCommand::StartTask { task_id, .. } => {
            let (task, member) = validate_task_member_command(command, tasks, config, &workflow)?;
            ensure_member_capacity(member, task_id)?;
            start_action(task, member, config, &workflow)
        }
        TaskBoardStaffCommand::PauseTask { .. } => {
            let (task, _) = validate_task_member_command(command, tasks, config, &workflow)?;
            let Some(new_status) = pause_target_status(task, &workflow) else {
                return Err(cannot_pause_status(task));
            };
            Ok(TaskBoardStaffCommandAction::ChangeTaskStatus {
                task_id: task.id.clone(),
                new_status,
            })
        }
        TaskBoardStaffCommand::ResumeTask { .. } => {
            let (task, _) = validate_task_member_command(command, tasks, config, &workflow)?;
            let Some(new_status) = resume_target_status(task, &workflow) else {
                return Err(cannot_resume_status(task));
            };
            Ok(TaskBoardStaffCommandAction::ChangeTaskStatus {
                task_id: task.id.clone(),
                new_status,
            })
        }
        TaskBoardStaffCommand::OpenTaskLogs { task_id, .. } => {
            let task = find_task(tasks, task_id)?;
            Ok(TaskBoardStaffCommandAction::ViewTaskLogs { task_id: task.id.clone() })
        }
        TaskBoardStaffCommand::FilterByMember { member_id, .. } => {
            let member = find_member(config, member_id)?;
            ensure_member_enabled(member)?;
            Ok(TaskBoardStaffCommandAction::FilterByMember { member_id: member.id.clone() })
        }
    }
}

fn validate_task_member_command<'a>(
    command: &TaskBoardStaffCommand,
    tasks: &'a [Task],
    config: &'a TaskBoardStaffConfig,
    workflow: &TaskWorkflow,
) -> Result<(&'a Task, &'a TaskBoardStaffMember), TaskBoardStaffCommandError> {
    let task_id = command.task_id().unwrap_or_default();
    let member_id = command.member_id().unwrap_or_default();
    let task = find_task(tasks, task_id)?;
    ensure_task_is_active(task, command.label(), workflow)?;
    let member = find_member(config, member_id)?;
    ensure_member_enabled(member)?;
    ensure_member_role_matches_task(config, task, member)?;
    Ok((task, member))
}

fn find_task<'a>(tasks: &'a [Task], task_id: &str) -> Result<&'a Task, TaskBoardStaffCommandError> {
    tasks.iter().find(|task| task.id == task_id).ok_or_else(|| {
        TaskBoardStaffCommandError::new(format!("员工命令失败：任务 {task_id} 不存在"))
    })
}

fn find_member<'a>(
    config: &'a TaskBoardStaffConfig,
    member_id: &str,
) -> Result<&'a TaskBoardStaffMember, TaskBoardStaffCommandError> {
    config.members.iter().find(|member| member.id == member_id).ok_or_else(|| {
        TaskBoardStaffCommandError::new(format!("员工命令失败：员工 {member_id} 不存在"))
    })
}

fn ensure_task_is_active(
    task: &Task,
    action_label: &str,
    workflow: &TaskWorkflow,
) -> Result<(), TaskBoardStaffCommandError> {
    if matches!(task.status, TaskStatus::Completed | TaskStatus::Archived)
        || matches!(
            workflow.status_role(task.status),
            Some(StatusRole::Done | StatusRole::Archived)
        )
    {
        return Err(TaskBoardStaffCommandError::new(format!(
            "员工命令失败：任务 {} 已结束，不能{}",
            task.id, action_label
        )));
    }
    Ok(())
}

fn ensure_member_enabled(member: &TaskBoardStaffMember) -> Result<(), TaskBoardStaffCommandError> {
    if !member.enabled {
        return Err(TaskBoardStaffCommandError::new(format!(
            "员工命令失败：员工 {} 已停用",
            member.id
        )));
    }
    Ok(())
}

fn ensure_member_role_matches_task(
    config: &TaskBoardStaffConfig,
    task: &Task,
    member: &TaskBoardStaffMember,
) -> Result<(), TaskBoardStaffCommandError> {
    let Some(required_role) = config.role_for_status(task.status) else {
        return Err(TaskBoardStaffCommandError::new(format!(
            "员工命令失败：任务 {} 的 {} 阶段没有可执行员工角色",
            task.id,
            task.status.label()
        )));
    };
    if member.role != required_role {
        return Err(TaskBoardStaffCommandError::new(format!(
            "员工命令失败：角色不匹配，任务 {} 的 {} 阶段需要 {}，但员工 {} 是 {}",
            task.id,
            task.status.label(),
            required_role.key(),
            member.id,
            member.role.key()
        )));
    }
    Ok(())
}

fn ensure_member_capacity(
    member: &TaskBoardStaffMember,
    task_id: &str,
) -> Result<(), TaskBoardStaffCommandError> {
    if member.current_task_ids.iter().any(|current_id| current_id == task_id) {
        return Ok(());
    }
    if member.current_task_ids.len() >= member.capacity as usize {
        return Err(TaskBoardStaffCommandError::new(format!(
            "员工命令失败：员工 {} 容量已满",
            member.id
        )));
    }
    Ok(())
}

fn start_action(
    task: &Task,
    member: &TaskBoardStaffMember,
    config: &TaskBoardStaffConfig,
    workflow: &TaskWorkflow,
) -> Result<TaskBoardStaffCommandAction, TaskBoardStaffCommandError> {
    let status_role = workflow.status_role(task.status);
    if let Some(action) = stage_action(config, task.status) {
        return stage_action_command(task, member, workflow, action);
    }

    match status_role {
        Some(StatusRole::Backlog | StatusRole::Failed) => advance_status_command(task, workflow),
        Some(StatusRole::Queued | StatusRole::Planning) => {
            Ok(TaskBoardStaffCommandAction::ExecuteTask { task_id: task.id.clone() })
        }
        Some(StatusRole::ReviewReady | StatusRole::Review | StatusRole::Merging) => {
            Err(missing_stage_action(task, workflow))
        }
        Some(StatusRole::Executing) => Err(TaskBoardStaffCommandError::new(format!(
            "员工命令失败：任务 {} 已在执行中",
            task.id
        ))),
        Some(StatusRole::Paused) => Err(TaskBoardStaffCommandError::new(format!(
            "员工命令失败：任务 {} 已暂停，请使用继续动作",
            task.id
        ))),
        Some(StatusRole::Done | StatusRole::Archived | StatusRole::Custom) | None => {
            Err(unsupported_start_status(task))
        }
    }
}

fn stage_action(
    config: &TaskBoardStaffConfig,
    status: TaskStatus,
) -> Option<&TaskBoardStageAction> {
    config
        .stage_actions
        .iter()
        .find(|action| action.status == status && !action.capability_id.trim().is_empty())
}

fn stage_action_command(
    task: &Task,
    member: &TaskBoardStaffMember,
    workflow: &TaskWorkflow,
    action: &TaskBoardStageAction,
) -> Result<TaskBoardStaffCommandAction, TaskBoardStaffCommandError> {
    match action.capability_id.trim() {
        ADVANCE_STATUS_CAPABILITY_ID => advance_status_command(task, workflow),
        EXECUTE_TASK_CAPABILITY_ID => {
            Ok(TaskBoardStaffCommandAction::ExecuteTask { task_id: task.id.clone() })
        }
        CODE_REVIEW_CAPABILITY_ID => {
            Ok(TaskBoardStaffCommandAction::RunCodeReview { task_id: task.id.clone() })
        }
        GIT_MERGE_CAPABILITY_ID => {
            Ok(TaskBoardStaffCommandAction::RunMerge { task_id: task.id.clone() })
        }
        SAVED_WORKFLOW_CAPABILITY_ID => {
            let Some(workflow_uuid) =
                action.workflow_uuid.as_deref().map(str::trim).filter(|value| !value.is_empty())
            else {
                return Err(TaskBoardStaffCommandError::new(format!(
                    "员工命令失败：任务 {} 的已保存 Workflow 阶段能力缺少 workflow_uuid",
                    task.id
                )));
            };
            Ok(TaskBoardStaffCommandAction::RunStageWorkflow {
                task_id: task.id.clone(),
                status: task.status,
                workflow_uuid: workflow_uuid.to_string(),
                member_id: member.id.clone(),
            })
        }
        MANUAL_CAPABILITY_ID => Err(TaskBoardStaffCommandError::new(format!(
            "员工命令失败：任务 {} 当前阶段配置为人工处理，不能通过开始命令自动执行",
            task.id
        ))),
        capability_id => Err(TaskBoardStaffCommandError::new(format!(
            "员工命令失败：任务 {} 配置了不支持的阶段能力 {}",
            task.id, capability_id
        ))),
    }
}

fn advance_status_command(
    task: &Task,
    workflow: &TaskWorkflow,
) -> Result<TaskBoardStaffCommandAction, TaskBoardStaffCommandError> {
    let Some(new_status) = workflow.next_status(task.status) else {
        return Err(TaskBoardStaffCommandError::new(format!(
            "员工命令失败：任务 {} 当前是 {}，没有可推进的下一阶段",
            task.id,
            task.status.label_for_workflow_id(workflow.id())
        )));
    };
    Ok(TaskBoardStaffCommandAction::ChangeTaskStatus { task_id: task.id.clone(), new_status })
}

fn missing_stage_action(task: &Task, workflow: &TaskWorkflow) -> TaskBoardStaffCommandError {
    TaskBoardStaffCommandError::new(format!(
        "员工命令失败：任务 {} 当前是 {}，需要配置阶段能力后才能开始",
        task.id,
        task.status.label_for_workflow_id(workflow.id())
    ))
}

fn unsupported_start_status(task: &Task) -> TaskBoardStaffCommandError {
    TaskBoardStaffCommandError::new(format!(
        "员工命令失败：任务 {} 当前是 {}，不能开始",
        task.id,
        task.status.label()
    ))
}

fn pause_target_status(task: &Task, workflow: &TaskWorkflow) -> Option<TaskStatus> {
    if !matches!(workflow.status_role(task.status), Some(StatusRole::Executing)) {
        return None;
    }
    status_with_role(workflow, StatusRole::Paused)
}

fn resume_target_status(task: &Task, workflow: &TaskWorkflow) -> Option<TaskStatus> {
    if !matches!(workflow.status_role(task.status), Some(StatusRole::Paused)) {
        return None;
    }
    workflow
        .next_status(task.status)
        .or_else(|| status_with_role(workflow, StatusRole::Queued))
        .or_else(|| status_with_role(workflow, StatusRole::Planning))
}

fn status_with_role(workflow: &TaskWorkflow, status_role: StatusRole) -> Option<TaskStatus> {
    workflow
        .statuses()
        .iter()
        .find(|definition| definition.role() == status_role)
        .map(|definition| definition.status)
}

fn cannot_pause_status(task: &Task) -> TaskBoardStaffCommandError {
    TaskBoardStaffCommandError::new(format!(
        "员工命令失败：任务 {} 当前是 {}，不能暂停",
        task.id,
        task.status.label()
    ))
}

fn cannot_resume_status(task: &Task) -> TaskBoardStaffCommandError {
    TaskBoardStaffCommandError::new(format!(
        "员工命令失败：任务 {} 当前是 {}，不能继续",
        task.id,
        task.status.label()
    ))
}

fn staff_workflow_for_id(workflow_id: &str) -> Result<TaskWorkflow, String> {
    let workflow_id = workflow_id.trim();
    let workflow_id = if workflow_id.is_empty() { DEFAULT_TASK_WORKFLOW_ID } else { workflow_id };
    task_workflow_by_id(workflow_id)
        .ok_or_else(|| format!("unknown task board workflow id: {workflow_id}"))
}
