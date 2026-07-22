//! 任务看板员工模式的纯路由与投影逻辑。

use std::collections::{HashMap, HashSet};

use crate::app::task::models::{Task, TaskExecutorBackend, TaskStatus};
use crate::app::task::workflow::{
    DEFAULT_TASK_WORKFLOW_ID, StatusRole, TaskWorkflow, task_workflow_by_id,
};

use super::models::{
    TaskBoardStaffConfig, TaskBoardStaffMember, TaskBoardStaffRole, TaskBoardStaffStatus,
    TaskBoardStageAssignment,
};

/// 员工模式投影，供任务看板按阶段、员工负载和未分配任务渲染。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskBoardStaffProjection {
    pub lanes: Vec<TaskBoardStaffLane>,
    pub member_loads: Vec<TaskBoardStaffMemberLoad>,
    pub unassigned: Vec<TaskBoardStaffAssignment>,
    pub total_task_count: usize,
    pub blocked_task_count: usize,
}

/// 单个任务状态阶段的员工模式 lane。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardStaffLane {
    pub status: TaskStatus,
    pub role: Option<TaskBoardStaffRole>,
    pub display_only: bool,
    pub assignments: Vec<TaskBoardStaffAssignment>,
    pub task_count: usize,
    pub blocked_count: usize,
}

/// 单个任务在员工模式里的路由结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardStaffAssignment {
    pub task_id: String,
    pub task_status: TaskStatus,
    pub task_description: String,
    pub priority: u32,
    pub order: u32,
    pub updated_at_ms: u64,
    pub role: Option<TaskBoardStaffRole>,
    pub member_id: Option<String>,
    pub member_name: Option<String>,
    pub member_role: Option<TaskBoardStaffRole>,
    pub display_only: bool,
    pub blocked: bool,
    pub block_reason: Option<String>,
    pub actions: TaskBoardStaffActionAvailability,
}

/// 员工模式中一个成员的当前负载。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardStaffMemberLoad {
    pub member_id: String,
    pub member_name: String,
    pub role: TaskBoardStaffRole,
    pub status: TaskBoardStaffStatus,
    pub enabled: bool,
    pub capacity: u32,
    pub executor_backend: Option<TaskExecutorBackend>,
    pub delegate_agent: Option<String>,
    pub acp_agent: Option<String>,
    pub task_ids: Vec<String>,
    pub running_task_ids: Vec<String>,
    pub blocked_task_ids: Vec<String>,
    pub load_count: usize,
    pub over_capacity: bool,
}

/// 员工模式里针对某个任务可展示的动作开关。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TaskBoardStaffActionAvailability {
    pub can_assign: bool,
    pub can_start: bool,
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_open_logs: bool,
    pub can_filter_member: bool,
}

#[derive(Clone)]
struct AssignmentMemberInfo {
    id: String,
    name: String,
    role: TaskBoardStaffRole,
}

struct MemberRoutingState<'a> {
    member: &'a TaskBoardStaffMember,
    task_ids: Vec<String>,
    running_task_ids: Vec<String>,
    blocked_task_ids: Vec<String>,
}

enum StageAssignmentChoice {
    Selected(usize),
    Unavailable(String),
}

impl TaskBoardStaffLane {
    fn new(status: TaskStatus, role: Option<TaskBoardStaffRole>, display_only: bool) -> Self {
        Self {
            status,
            role,
            display_only,
            assignments: Vec::new(),
            task_count: 0,
            blocked_count: 0,
        }
    }
}

impl TaskBoardStaffActionAvailability {
    fn for_assignment(
        status: TaskStatus,
        workflow: &TaskWorkflow,
        display_only: bool,
        blocked: bool,
        has_member: bool,
    ) -> Self {
        let status_role = workflow.status_role(status);
        Self {
            can_assign: !display_only && !blocked,
            can_start: has_member
                && !display_only
                && is_startable_status_role(status_role, blocked),
            can_pause: has_member
                && !blocked
                && matches!(status_role, Some(StatusRole::Executing))
                && status_with_role(workflow, StatusRole::Paused).is_some(),
            can_resume: has_member
                && matches!(status_role, Some(StatusRole::Paused))
                && resume_target_status(workflow).is_some(),
            can_open_logs: true,
            can_filter_member: has_member,
        }
    }
}

/// 从任务池、员工配置和运行中任务 id 生成员工模式投影。
pub fn build_staff_mode_projection(
    tasks: &[Task],
    config: &TaskBoardStaffConfig,
    running_task_ids: &[String],
    workflow_id: &str,
) -> Result<TaskBoardStaffProjection, String> {
    let workflow = staff_workflow_for_id(workflow_id)?;
    if config.workflow.is_empty() {
        return Err(format!(
            "task lifecycle workflow {} has no staff stages loaded",
            workflow.id()
        ));
    }
    let running_task_set = running_task_ids.iter().map(String::as_str).collect::<HashSet<_>>();
    let task_id_set = tasks.iter().map(|task| task.id.as_str()).collect::<HashSet<_>>();
    let mut member_states = build_member_states(config, &task_id_set, &running_task_set);
    let mut lanes = build_lanes(config);
    let mut lane_index_by_status = lanes
        .iter()
        .enumerate()
        .map(|(index, lane)| (lane.status, index))
        .collect::<HashMap<_, _>>();
    let mut unassigned = Vec::new();

    for task in tasks {
        let stage = stage_for_status(config, task.status, workflow.id())?;
        let assignment = route_task(task, stage, &mut member_states, &running_task_set, &workflow);
        if assignment.member_id.is_none() && !assignment.display_only {
            unassigned.push(assignment.clone());
        }
        push_assignment(
            &mut lanes,
            &mut lane_index_by_status,
            task.status,
            stage.role,
            stage.display_only,
            assignment,
        );
    }

    let blocked_task_count = lanes.iter().map(|lane| lane.blocked_count).sum();
    Ok(TaskBoardStaffProjection {
        lanes,
        member_loads: member_states.into_iter().map(MemberRoutingState::into_load).collect(),
        unassigned,
        total_task_count: tasks.len(),
        blocked_task_count,
    })
}

fn build_member_states<'a>(
    config: &'a TaskBoardStaffConfig,
    task_id_set: &HashSet<&str>,
    running_task_set: &HashSet<&str>,
) -> Vec<MemberRoutingState<'a>> {
    config
        .members
        .iter()
        .map(|member| {
            let task_ids = member
                .current_task_ids
                .iter()
                .filter(|task_id| !task_id_set.contains(task_id.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            let running_task_ids = task_ids
                .iter()
                .filter(|task_id| running_task_set.contains(task_id.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            MemberRoutingState { member, task_ids, running_task_ids, blocked_task_ids: Vec::new() }
        })
        .collect()
}

fn build_lanes(config: &TaskBoardStaffConfig) -> Vec<TaskBoardStaffLane> {
    let mut seen = HashSet::new();
    let mut lanes = Vec::new();
    for stage in &config.workflow {
        if seen.insert(stage.status) {
            lanes.push(TaskBoardStaffLane::new(stage.status, stage.role, stage.display_only));
        }
    }

    lanes
}

fn route_task(
    task: &Task,
    stage: RoutingStage<'_>,
    member_states: &mut [MemberRoutingState<'_>],
    running_task_set: &HashSet<&str>,
    workflow: &TaskWorkflow,
) -> TaskBoardStaffAssignment {
    if matches!(task.status, TaskStatus::Completed | TaskStatus::Archived) || stage.display_only {
        return build_assignment(task, stage.role, None, true, false, None, workflow);
    }

    let mut stage_assignment_unavailable = None;
    if let Some(assignment) = stage.assignment {
        match choose_member_for_stage_assignment(member_states, assignment) {
            StageAssignmentChoice::Selected(member_index) => {
                let blocked = matches!(task.status, TaskStatus::Failed | TaskStatus::Paused);
                let block_reason = blocked.then(|| blocked_task_reason(task));
                let member_info = member_info(member_states[member_index].member);
                let role = stage.role.or(Some(member_info.role));
                record_member_task(
                    &mut member_states[member_index],
                    task,
                    running_task_set,
                    blocked,
                );
                return build_assignment(
                    task,
                    role,
                    Some(member_info),
                    false,
                    blocked,
                    block_reason,
                    workflow,
                );
            }
            StageAssignmentChoice::Unavailable(reason) => {
                stage_assignment_unavailable = Some(reason);
            }
        }
    }

    if matches!(task.status, TaskStatus::Failed | TaskStatus::Paused) {
        return route_blocked_task(task, stage, member_states, running_task_set, workflow);
    }

    let Some(role) = stage.role else {
        let reason = match stage_assignment_unavailable {
            Some(stage_reason) => format!(
                "{stage_reason}; no staff role configured for status {}",
                task.status.to_string_key()
            ),
            None => format!("no staff role configured for status {}", task.status.to_string_key()),
        };
        return build_assignment(task, None, None, false, true, Some(reason), workflow);
    };

    let preferred_index = find_original_member_index(task, member_states);
    let Some(member_index) = choose_member_for_role(member_states, role, preferred_index) else {
        let role_reason = unavailable_role_reason(role, member_states);
        let reason = match stage_assignment_unavailable {
            Some(stage_reason) => format!("{stage_reason}; {role_reason}"),
            None => role_reason,
        };
        return build_assignment(task, Some(role), None, false, true, Some(reason), workflow);
    };

    let member_info = member_info(member_states[member_index].member);
    record_member_task(&mut member_states[member_index], task, running_task_set, false);
    build_assignment(task, Some(role), Some(member_info), false, false, None, workflow)
}

fn choose_member_for_stage_assignment(
    member_states: &[MemberRoutingState<'_>],
    assignment: &TaskBoardStageAssignment,
) -> StageAssignmentChoice {
    let mut reasons = Vec::new();
    let mut has_candidate = false;

    if let Some(member_id) = assignment.primary_member_id.as_deref().map(str::trim) {
        if !member_id.is_empty() {
            has_candidate = true;
            if let Some(index) =
                choose_stage_assignment_candidate(member_states, member_id, &mut reasons)
            {
                return StageAssignmentChoice::Selected(index);
            }
        }
    }

    for member_id in &assignment.backup_member_ids {
        let member_id = member_id.trim();
        if member_id.is_empty() {
            continue;
        }
        has_candidate = true;
        if let Some(index) =
            choose_stage_assignment_candidate(member_states, member_id, &mut reasons)
        {
            return StageAssignmentChoice::Selected(index);
        }
    }

    if !has_candidate {
        return StageAssignmentChoice::Unavailable(format!(
            "no primary or backup staff member configured for stage {}",
            assignment.status.to_string_key()
        ));
    }

    StageAssignmentChoice::Unavailable(format!(
        "stage assignment unavailable for {}: {}",
        assignment.status.to_string_key(),
        reasons.join("; ")
    ))
}

fn choose_stage_assignment_candidate(
    member_states: &[MemberRoutingState<'_>],
    member_id: &str,
    reasons: &mut Vec<String>,
) -> Option<usize> {
    let Some(index) = member_states.iter().position(|state| state.member.id == member_id) else {
        reasons.push(format!("configured member {member_id} does not exist"));
        return None;
    };
    if member_is_available(&member_states[index]) {
        return Some(index);
    }
    reasons.push(stage_assignment_member_unavailable_reason(&member_states[index]));
    None
}

fn member_is_available(state: &MemberRoutingState<'_>) -> bool {
    state.member.enabled && state.task_ids.len() < state.member.capacity as usize
}

fn stage_assignment_member_unavailable_reason(state: &MemberRoutingState<'_>) -> String {
    if !state.member.enabled {
        return format!("configured member {} is disabled", state.member.id);
    }
    if state.task_ids.len() >= state.member.capacity as usize {
        return format!("configured member {} has no available capacity", state.member.id);
    }
    format!("configured member {} is unavailable", state.member.id)
}

fn route_blocked_task(
    task: &Task,
    stage: RoutingStage<'_>,
    member_states: &mut [MemberRoutingState<'_>],
    running_task_set: &HashSet<&str>,
    workflow: &TaskWorkflow,
) -> TaskBoardStaffAssignment {
    let reason = blocked_task_reason(task);
    let Some(member_index) = find_original_member_index(task, member_states) else {
        return build_assignment(
            task,
            stage.role,
            None,
            false,
            true,
            Some(format!("{reason}; original staff member is not assigned")),
            workflow,
        );
    };

    let member = member_states[member_index].member;
    let member_info = member_info(member);
    record_member_task(&mut member_states[member_index], task, running_task_set, true);
    build_assignment(
        task,
        Some(member.role),
        Some(member_info),
        false,
        true,
        Some(reason),
        workflow,
    )
}

fn choose_member_for_role(
    member_states: &[MemberRoutingState<'_>],
    role: TaskBoardStaffRole,
    preferred_index: Option<usize>,
) -> Option<usize> {
    if let Some(index) = preferred_index {
        if member_is_available_for_role(&member_states[index], role) {
            return Some(index);
        }
    }

    member_states
        .iter()
        .enumerate()
        .find_map(|(index, state)| member_is_available_for_role(state, role).then_some(index))
}

fn member_is_available_for_role(state: &MemberRoutingState<'_>, role: TaskBoardStaffRole) -> bool {
    state.member.role == role && member_is_available(state)
}

fn find_original_member_index(
    task: &Task,
    member_states: &[MemberRoutingState<'_>],
) -> Option<usize> {
    member_states
        .iter()
        .position(|state| state.member.current_task_ids.iter().any(|task_id| task_id == &task.id))
        .or_else(|| find_member_by_task_hints(task, member_states))
}

fn find_member_by_task_hints(
    task: &Task,
    member_states: &[MemberRoutingState<'_>],
) -> Option<usize> {
    let assignee = task.assignee.trim();
    if !assignee.is_empty() {
        if let Some(index) = member_states
            .iter()
            .position(|state| state.member.id == assignee || state.member.name == assignee)
        {
            return Some(index);
        }
    }

    let task_acp_agent = task.acp_agent.as_deref().map(str::trim).filter(|value| !value.is_empty());
    task_acp_agent.and_then(|agent| {
        member_states.iter().position(|state| {
            state
                .member
                .acp_agent
                .as_deref()
                .map(str::trim)
                .is_some_and(|member_agent| member_agent == agent)
        })
    })
}

fn record_member_task(
    state: &mut MemberRoutingState<'_>,
    task: &Task,
    running_task_set: &HashSet<&str>,
    blocked: bool,
) {
    push_unique(&mut state.task_ids, task.id.clone());
    if running_task_set.contains(task.id.as_str()) {
        push_unique(&mut state.running_task_ids, task.id.clone());
    }
    if blocked {
        push_unique(&mut state.blocked_task_ids, task.id.clone());
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn unavailable_role_reason(
    role: TaskBoardStaffRole,
    member_states: &[MemberRoutingState<'_>],
) -> String {
    let mut has_same_role = false;
    let mut has_enabled_member = false;
    for state in member_states {
        if state.member.role != role {
            continue;
        }
        has_same_role = true;
        if state.member.enabled {
            has_enabled_member = true;
        }
    }

    if !has_same_role {
        return format!("no {} staff member is configured", role.key());
    }
    if !has_enabled_member {
        return format!("no enabled {} staff member is available", role.key());
    }
    format!("no enabled {} staff member has available capacity", role.key())
}

fn blocked_task_reason(task: &Task) -> String {
    match task.status {
        TaskStatus::Failed => task
            .last_error
            .as_deref()
            .map(str::trim)
            .filter(|reason| !reason.is_empty())
            .map(|reason| format!("task failed: {reason}"))
            .unwrap_or_else(|| "task failed".to_string()),
        TaskStatus::Paused => task
            .pause_reason
            .as_deref()
            .map(str::trim)
            .filter(|reason| !reason.is_empty())
            .map(|reason| format!("task paused: {reason}"))
            .unwrap_or_else(|| "task paused".to_string()),
        _ => "task is blocked".to_string(),
    }
}

fn build_assignment(
    task: &Task,
    role: Option<TaskBoardStaffRole>,
    member: Option<AssignmentMemberInfo>,
    display_only: bool,
    blocked: bool,
    block_reason: Option<String>,
    workflow: &TaskWorkflow,
) -> TaskBoardStaffAssignment {
    let has_member = member.is_some();
    TaskBoardStaffAssignment {
        task_id: task.id.clone(),
        task_status: task.status,
        task_description: task.description.clone(),
        priority: task.priority,
        order: task.order,
        updated_at_ms: task.updated_at_ms,
        role,
        member_id: member.as_ref().map(|member| member.id.clone()),
        member_name: member.as_ref().map(|member| member.name.clone()),
        member_role: member.as_ref().map(|member| member.role),
        display_only,
        blocked,
        block_reason,
        actions: TaskBoardStaffActionAvailability::for_assignment(
            task.status,
            workflow,
            display_only,
            blocked,
            has_member,
        ),
    }
}

fn is_startable_status_role(status_role: Option<StatusRole>, blocked: bool) -> bool {
    match status_role {
        Some(StatusRole::Backlog | StatusRole::Queued | StatusRole::Planning) => !blocked,
        Some(StatusRole::Failed) => true,
        Some(StatusRole::ReviewReady | StatusRole::Review | StatusRole::Merging) => !blocked,
        Some(
            StatusRole::Executing
            | StatusRole::Paused
            | StatusRole::Done
            | StatusRole::Archived
            | StatusRole::Custom,
        )
        | None => false,
    }
}

fn status_with_role(workflow: &TaskWorkflow, status_role: StatusRole) -> Option<TaskStatus> {
    workflow
        .statuses()
        .iter()
        .find(|definition| definition.role() == status_role)
        .map(|definition| definition.status)
}

fn resume_target_status(workflow: &TaskWorkflow) -> Option<TaskStatus> {
    status_with_role(workflow, StatusRole::Queued)
        .or_else(|| status_with_role(workflow, StatusRole::Planning))
}

fn staff_workflow_for_id(workflow_id: &str) -> Result<TaskWorkflow, String> {
    let workflow_id = workflow_id.trim();
    let workflow_id = if workflow_id.is_empty() { DEFAULT_TASK_WORKFLOW_ID } else { workflow_id };
    task_workflow_by_id(workflow_id)
        .ok_or_else(|| format!("unknown task board workflow id: {workflow_id}"))
}

fn push_assignment(
    lanes: &mut Vec<TaskBoardStaffLane>,
    lane_index_by_status: &mut HashMap<TaskStatus, usize>,
    status: TaskStatus,
    role: Option<TaskBoardStaffRole>,
    display_only: bool,
    assignment: TaskBoardStaffAssignment,
) {
    let index = *lane_index_by_status.entry(status).or_insert_with(|| {
        lanes.push(TaskBoardStaffLane::new(status, role, display_only));
        lanes.len() - 1
    });
    let lane = &mut lanes[index];
    if assignment.blocked {
        lane.blocked_count = lane.blocked_count.saturating_add(1);
    }
    lane.assignments.push(assignment);
    lane.task_count = lane.assignments.len();
}

fn member_info(member: &TaskBoardStaffMember) -> AssignmentMemberInfo {
    AssignmentMemberInfo { id: member.id.clone(), name: member.name.clone(), role: member.role }
}

impl MemberRoutingState<'_> {
    fn into_load(self) -> TaskBoardStaffMemberLoad {
        let load_count = self.task_ids.len();
        TaskBoardStaffMemberLoad {
            member_id: self.member.id.clone(),
            member_name: self.member.name.clone(),
            role: self.member.role,
            status: self.member.status,
            enabled: self.member.enabled,
            capacity: self.member.capacity,
            executor_backend: self.member.executor_backend,
            delegate_agent: self.member.delegate_agent.clone(),
            acp_agent: self.member.acp_agent.clone(),
            task_ids: self.task_ids,
            running_task_ids: self.running_task_ids,
            blocked_task_ids: self.blocked_task_ids,
            load_count,
            over_capacity: load_count > self.member.capacity as usize,
        }
    }
}

#[derive(Clone, Copy)]
struct RoutingStage<'a> {
    role: Option<TaskBoardStaffRole>,
    display_only: bool,
    assignment: Option<&'a TaskBoardStageAssignment>,
}

fn stage_for_status<'a>(
    config: &'a TaskBoardStaffConfig,
    status: TaskStatus,
    workflow_id: &str,
) -> Result<RoutingStage<'a>, String> {
    let assignment = config.stage_assignments.iter().find(|assignment| assignment.status == status);
    config
        .workflow
        .iter()
        .find(|stage| stage.status == status)
        .map(|stage| RoutingStage {
            role: stage.role,
            display_only: stage.display_only,
            assignment,
        })
        .ok_or_else(|| {
            format!(
                "task lifecycle workflow {} does not define status {}",
                workflow_id,
                status.to_string_key()
            )
        })
}
