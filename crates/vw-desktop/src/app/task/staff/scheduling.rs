//! 员工阶段分配驱动的自主调度策略。

use std::collections::{HashMap, HashSet};

use crate::app::task::models::{Task, TaskStatus};
use crate::app::task::workflow::{
    DEFAULT_TASK_WORKFLOW_ID, StatusRole, TaskWorkflow, task_workflow_by_id,
};

use super::commands::TaskBoardStaffCommand;
use super::events::TaskBoardStaffOrchestrationContext;
use super::models::{TaskBoardStaffConfig, TaskBoardStaffMember, TaskBoardStageAssignment};
use super::routing::{TaskBoardStaffAssignment, TaskBoardStaffProjection};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskBoardStaffAutoSchedulePlan {
    pub commands: Vec<TaskBoardStaffCommand>,
    pub command_contexts: Vec<TaskBoardStaffAutoScheduleCommandContext>,
    pub skipped: Vec<TaskBoardStaffAutoScheduleSkip>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardStaffAutoScheduleCommandContext {
    pub command_id: String,
    pub orchestration: TaskBoardStaffOrchestrationContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardStaffAutoScheduleSkip {
    pub task_id: String,
    pub status: TaskStatus,
    pub reason: String,
    pub orchestration: Option<TaskBoardStaffOrchestrationContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutoScheduleAction {
    Assign,
    Start,
    Resume,
}

impl AutoScheduleAction {
    fn key(self) -> &'static str {
        match self {
            AutoScheduleAction::Assign => "assign",
            AutoScheduleAction::Start => "start",
            AutoScheduleAction::Resume => "resume",
        }
    }

    fn target_label(self) -> &'static str {
        match self {
            AutoScheduleAction::Assign => "分配",
            AutoScheduleAction::Start => "开始",
            AutoScheduleAction::Resume => "继续",
        }
    }
}

pub fn build_staff_auto_schedule_plan(
    tasks: &[Task],
    config: &TaskBoardStaffConfig,
    projection: &TaskBoardStaffProjection,
    running_task_ids: &[String],
    workflow_id: &str,
    now_ms: u64,
) -> Result<TaskBoardStaffAutoSchedulePlan, String> {
    let workflow = staff_workflow_for_id(workflow_id)?;
    let running_task_ids = running_task_ids.iter().map(String::as_str).collect::<HashSet<_>>();
    let mut member_loads = config
        .members
        .iter()
        .map(|member| (member.id.as_str(), member.current_task_ids.len()))
        .collect::<HashMap<_, _>>();
    let mut scheduled = HashSet::new();
    let mut plan = TaskBoardStaffAutoSchedulePlan::default();

    for task in tasks {
        let Some(stage_assignment) = config
            .stage_assignments
            .iter()
            .find(|assignment| assignment.status == task.status && assignment.autonomous)
        else {
            continue;
        };

        if task_is_ended(task, &workflow) {
            push_skip(&mut plan, task, "task ended");
            continue;
        }

        if running_task_ids.contains(task.id.as_str()) {
            push_skip(&mut plan, task, "task is already in the running list");
            continue;
        }

        let projection_assignment = find_projection_assignment(projection, &task.id);
        if let Some(member) = current_member_for_task(config, &task.id) {
            if member_skip_reason(config, task, member).is_none() {
                schedule_assigned_task(
                    &mut plan,
                    &mut scheduled,
                    config,
                    stage_assignment,
                    projection_assignment,
                    task,
                    member,
                    now_ms,
                );
                continue;
            }
        }

        schedule_stage_assignment_member(
            &mut plan,
            &mut scheduled,
            config,
            stage_assignment,
            task,
            &mut member_loads,
            now_ms,
        );
    }

    Ok(plan)
}

fn schedule_stage_assignment_member(
    plan: &mut TaskBoardStaffAutoSchedulePlan,
    scheduled: &mut HashSet<String>,
    config: &TaskBoardStaffConfig,
    stage_assignment: &TaskBoardStageAssignment,
    task: &Task,
    member_loads: &mut HashMap<&str, usize>,
    now_ms: u64,
) {
    match select_stage_assignment_member(config, stage_assignment, task, member_loads) {
        Ok(member_id) => {
            let command = TaskBoardStaffCommand::AssignTask {
                command_id: auto_command_id(
                    AutoScheduleAction::Assign,
                    &task.id,
                    &member_id,
                    now_ms,
                ),
                task_id: task.id.clone(),
                member_id: member_id.clone(),
            };
            let orchestration =
                config.members.iter().find(|member| member.id == member_id).map(|member| {
                    orchestration_context(
                        config,
                        stage_assignment,
                        task,
                        member,
                        AutoScheduleAction::Assign,
                    )
                });
            if push_command(plan, scheduled, command, task.status, orchestration)
                && let Some(load) = member_loads.get_mut(member_id.as_str())
            {
                *load = load.saturating_add(1);
            }
        }
        Err(reason) => push_skip(plan, task, reason),
    }
}

fn schedule_assigned_task(
    plan: &mut TaskBoardStaffAutoSchedulePlan,
    scheduled: &mut HashSet<String>,
    config: &TaskBoardStaffConfig,
    stage_assignment: &TaskBoardStageAssignment,
    projection_assignment: Option<&TaskBoardStaffAssignment>,
    task: &Task,
    member: &TaskBoardStaffMember,
    now_ms: u64,
) {
    if let Some(reason) = member_skip_reason(config, task, member) {
        push_skip(plan, task, reason);
        return;
    }

    if task.status == TaskStatus::Paused {
        if projection_assignment.is_some_and(|assignment| assignment.actions.can_resume) {
            push_command(
                plan,
                scheduled,
                TaskBoardStaffCommand::ResumeTask {
                    command_id: auto_command_id(
                        AutoScheduleAction::Resume,
                        &task.id,
                        &member.id,
                        now_ms,
                    ),
                    task_id: task.id.clone(),
                    member_id: member.id.clone(),
                },
                task.status,
                Some(orchestration_context(
                    config,
                    stage_assignment,
                    task,
                    member,
                    AutoScheduleAction::Resume,
                )),
            );
        } else {
            push_skip(plan, task, "task is paused but cannot be resumed by this stage");
        }
        return;
    }

    if projection_assignment.is_some_and(|assignment| assignment.actions.can_start) {
        push_command(
            plan,
            scheduled,
            TaskBoardStaffCommand::StartTask {
                command_id: auto_command_id(
                    AutoScheduleAction::Start,
                    &task.id,
                    &member.id,
                    now_ms,
                ),
                task_id: task.id.clone(),
                member_id: member.id.clone(),
            },
            task.status,
            Some(orchestration_context(
                config,
                stage_assignment,
                task,
                member,
                AutoScheduleAction::Start,
            )),
        );
    } else {
        push_skip(plan, task, "task status is not auto-startable");
    }
}

fn select_stage_assignment_member(
    config: &TaskBoardStaffConfig,
    assignment: &TaskBoardStageAssignment,
    task: &Task,
    member_loads: &HashMap<&str, usize>,
) -> Result<String, String> {
    let mut candidate_ids = Vec::new();
    let mut seen = HashSet::new();
    if let Some(member_id) = assignment.primary_member_id.as_deref().map(str::trim)
        && !member_id.is_empty()
        && seen.insert(member_id)
    {
        candidate_ids.push(member_id);
    }
    for member_id in &assignment.backup_member_ids {
        let member_id = member_id.trim();
        if !member_id.is_empty() && seen.insert(member_id) {
            candidate_ids.push(member_id);
        }
    }

    if candidate_ids.is_empty() {
        return Err(format!(
            "no primary or backup staff member configured for autonomous stage {}",
            assignment.status.to_string_key()
        ));
    }

    let mut reasons = Vec::new();
    for member_id in candidate_ids {
        let Some(member) = config.members.iter().find(|member| member.id == member_id) else {
            reasons.push(format!("configured member {member_id} does not exist"));
            continue;
        };
        if let Some(reason) = member_skip_reason(config, task, member) {
            reasons.push(reason);
            continue;
        }
        let load = member_loads.get(member.id.as_str()).copied().unwrap_or_default();
        if load >= member.capacity as usize {
            reasons.push(format!("member {} capacity is full", member.id));
            continue;
        }
        return Ok(member.id.clone());
    }

    Err(reasons.join("; "))
}

fn member_skip_reason(
    config: &TaskBoardStaffConfig,
    task: &Task,
    member: &TaskBoardStaffMember,
) -> Option<String> {
    if !member.enabled {
        return Some(format!("member {} is disabled", member.id));
    }
    let Some(required_role) = config.role_for_status(task.status) else {
        return Some(format!("role is not configured for status {}", task.status.to_string_key()));
    };
    if member.role != required_role {
        return Some(format!(
            "role mismatch for task {}: status {} requires {}, member {} is {}",
            task.id,
            task.status.to_string_key(),
            required_role.key(),
            member.id,
            member.role.key()
        ));
    }
    None
}

fn current_member_for_task<'a>(
    config: &'a TaskBoardStaffConfig,
    task_id: &str,
) -> Option<&'a TaskBoardStaffMember> {
    config
        .members
        .iter()
        .find(|member| member.current_task_ids.iter().any(|current_id| current_id == task_id))
}

fn find_projection_assignment<'a>(
    projection: &'a TaskBoardStaffProjection,
    task_id: &str,
) -> Option<&'a TaskBoardStaffAssignment> {
    projection
        .lanes
        .iter()
        .flat_map(|lane| lane.assignments.iter())
        .find(|assignment| assignment.task_id == task_id)
}

pub(super) fn push_command(
    plan: &mut TaskBoardStaffAutoSchedulePlan,
    scheduled: &mut HashSet<String>,
    command: TaskBoardStaffCommand,
    status: TaskStatus,
    orchestration: Option<TaskBoardStaffOrchestrationContext>,
) -> bool {
    let task_id = command.task_id().unwrap_or_default().to_string();
    let member_id = command.member_id().unwrap_or_default().to_string();
    let key = format!("{}:{task_id}:{member_id}", command.label());
    if scheduled.insert(key) {
        if let Some(orchestration) = orchestration {
            plan.command_contexts.push(TaskBoardStaffAutoScheduleCommandContext {
                command_id: command.command_id().to_string(),
                orchestration,
            });
        }
        plan.commands.push(command);
        return true;
    }

    plan.skipped.push(TaskBoardStaffAutoScheduleSkip {
        task_id,
        status,
        reason: "duplicate auto schedule command".to_string(),
        orchestration,
    });
    false
}

fn push_skip(plan: &mut TaskBoardStaffAutoSchedulePlan, task: &Task, reason: impl Into<String>) {
    plan.skipped.push(TaskBoardStaffAutoScheduleSkip {
        task_id: task.id.clone(),
        status: task.status,
        reason: reason.into(),
        orchestration: None,
    });
}

fn auto_command_id(
    action: AutoScheduleAction,
    task_id: &str,
    member_id: &str,
    now_ms: u64,
) -> String {
    format!("staff-auto-{}-{task_id}-{member_id}-{now_ms}", action.key())
}

fn orchestration_context(
    config: &TaskBoardStaffConfig,
    assignment: &TaskBoardStageAssignment,
    task: &Task,
    member: &TaskBoardStaffMember,
    action: AutoScheduleAction,
) -> TaskBoardStaffOrchestrationContext {
    TaskBoardStaffOrchestrationContext::new(
        task.status,
        member,
        "自主调度",
        format!("{} {} -> {}", action.target_label(), task.id, member.name),
        manager_member_names(config, assignment),
    )
}

fn manager_member_names(
    config: &TaskBoardStaffConfig,
    assignment: &TaskBoardStageAssignment,
) -> Vec<String> {
    assignment
        .manager_member_ids
        .iter()
        .filter_map(|member_id| {
            config
                .members
                .iter()
                .find(|member| member.id == *member_id)
                .map(|member| member.name.clone())
        })
        .collect()
}

fn task_is_ended(task: &Task, workflow: &TaskWorkflow) -> bool {
    matches!(task.status, TaskStatus::Completed | TaskStatus::Archived)
        || matches!(
            workflow.status_role(task.status),
            Some(StatusRole::Done | StatusRole::Archived)
        )
}

fn staff_workflow_for_id(workflow_id: &str) -> Result<TaskWorkflow, String> {
    let workflow_id = workflow_id.trim();
    let workflow_id = if workflow_id.is_empty() { DEFAULT_TASK_WORKFLOW_ID } else { workflow_id };
    task_workflow_by_id(workflow_id)
        .ok_or_else(|| format!("unknown task board workflow id: {workflow_id}"))
}
