//! 员工模式命令队列和事件流。

use std::collections::{HashMap, HashSet, VecDeque};

use crate::app::task::models::TaskStatus;

use super::commands::TaskBoardStaffCommand;
use super::models::{TaskBoardStaffLevel, TaskBoardStaffMember};

/// 员工模式命令状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskBoardStaffCommandStatus {
    Pending,
    Inflight,
    Applied,
    Failed,
    Skipped,
}

impl TaskBoardStaffCommandStatus {
    pub fn label(self) -> &'static str {
        match self {
            TaskBoardStaffCommandStatus::Pending => "待处理",
            TaskBoardStaffCommandStatus::Inflight => "处理中",
            TaskBoardStaffCommandStatus::Applied => "已应用",
            TaskBoardStaffCommandStatus::Failed => "失败",
            TaskBoardStaffCommandStatus::Skipped => "已跳过",
        }
    }
}

/// 员工模式事件类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskBoardStaffEventKind {
    Command,
    Message,
}

/// 员工模式事件状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskBoardStaffEventStatus {
    Command(TaskBoardStaffCommandStatus),
    Message,
}

impl TaskBoardStaffEventStatus {
    pub fn label(self) -> &'static str {
        match self {
            TaskBoardStaffEventStatus::Command(status) => status.label(),
            TaskBoardStaffEventStatus::Message => "消息",
        }
    }

    pub fn command_status(self) -> Option<TaskBoardStaffCommandStatus> {
        match self {
            TaskBoardStaffEventStatus::Command(status) => Some(status),
            TaskBoardStaffEventStatus::Message => None,
        }
    }

    pub fn is_failed(self) -> bool {
        self.command_status() == Some(TaskBoardStaffCommandStatus::Failed)
    }
}

impl From<TaskBoardStaffCommandStatus> for TaskBoardStaffEventStatus {
    fn from(status: TaskBoardStaffCommandStatus) -> Self {
        TaskBoardStaffEventStatus::Command(status)
    }
}

impl PartialEq<TaskBoardStaffCommandStatus> for TaskBoardStaffEventStatus {
    fn eq(&self, other: &TaskBoardStaffCommandStatus) -> bool {
        self.command_status().is_some_and(|status| status == *other)
    }
}

impl PartialEq<TaskBoardStaffEventStatus> for TaskBoardStaffCommandStatus {
    fn eq(&self, other: &TaskBoardStaffEventStatus) -> bool {
        other == self
    }
}

/// 员工交接消息投递状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskBoardStaffMessageDeliveryStatus {
    Pending,
    Delivered,
    Failed,
}

impl TaskBoardStaffMessageDeliveryStatus {
    pub fn label(self) -> &'static str {
        match self {
            TaskBoardStaffMessageDeliveryStatus::Pending => "待投递",
            TaskBoardStaffMessageDeliveryStatus::Delivered => "已送达",
            TaskBoardStaffMessageDeliveryStatus::Failed => "投递失败",
        }
    }
}

/// 员工之间的短交接消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardStaffMessageEvent {
    pub task_id: String,
    pub from_member_id: String,
    pub to_member_id: String,
    pub body: String,
    pub timestamp_ms: u64,
    pub delivery_status: Option<TaskBoardStaffMessageDeliveryStatus>,
}

/// 员工任务流中会被互斥去重的命令组。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskBoardStaffCommandGroup {
    Assignment,
    Start,
    PauseResume,
}

/// 员工模式事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardStaffEvent {
    pub kind: TaskBoardStaffEventKind,
    pub command_id: String,
    pub task_id: Option<String>,
    pub member_id: Option<String>,
    pub status: TaskBoardStaffEventStatus,
    pub timestamp_ms: u64,
    pub message: String,
    pub orchestration: Option<TaskBoardStaffOrchestrationContext>,
    pub staff_message: Option<TaskBoardStaffMessageEvent>,
}

/// 阶段编排上下文，用于让命令事件保留阶段、层级和目标语义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardStaffOrchestrationContext {
    pub stage_status: TaskStatus,
    pub stage_label: String,
    pub member_level: TaskBoardStaffLevel,
    pub member_level_label: String,
    pub member_name: String,
    pub action_source: String,
    pub target: String,
    pub manager_member_names: Vec<String>,
}

impl TaskBoardStaffOrchestrationContext {
    pub fn new(
        stage_status: TaskStatus,
        member: &TaskBoardStaffMember,
        action_source: impl Into<String>,
        target: impl Into<String>,
        manager_member_names: Vec<String>,
    ) -> Self {
        Self {
            stage_status,
            stage_label: stage_status.label().to_string(),
            member_level: member.level,
            member_level_label: staff_level_label(member.level).to_string(),
            member_name: member.name.clone(),
            action_source: action_source.into(),
            target: target.into(),
            manager_member_names,
        }
    }
}

impl TaskBoardStaffEvent {
    pub fn command_event(
        command_id: impl Into<String>,
        task_id: Option<String>,
        member_id: Option<String>,
        status: TaskBoardStaffCommandStatus,
        timestamp_ms: u64,
        message: impl Into<String>,
        orchestration: Option<TaskBoardStaffOrchestrationContext>,
    ) -> Self {
        Self {
            kind: TaskBoardStaffEventKind::Command,
            command_id: command_id.into(),
            task_id,
            member_id,
            status: status.into(),
            timestamp_ms,
            message: message.into(),
            orchestration,
            staff_message: None,
        }
    }

    pub fn staff_message(
        task_id: impl Into<String>,
        from_member_id: impl Into<String>,
        to_member_id: impl Into<String>,
        body: impl Into<String>,
        timestamp_ms: u64,
        delivery_status: Option<TaskBoardStaffMessageDeliveryStatus>,
    ) -> Self {
        let staff_message = TaskBoardStaffMessageEvent {
            task_id: task_id.into(),
            from_member_id: from_member_id.into(),
            to_member_id: to_member_id.into(),
            body: body.into(),
            timestamp_ms,
            delivery_status,
        };
        Self {
            kind: TaskBoardStaffEventKind::Message,
            command_id: String::new(),
            task_id: Some(staff_message.task_id.clone()),
            member_id: Some(staff_message.from_member_id.clone()),
            status: TaskBoardStaffEventStatus::Message,
            timestamp_ms,
            message: staff_message.body.clone(),
            orchestration: None,
            staff_message: Some(staff_message),
        }
    }

    pub fn command_status(&self) -> Option<TaskBoardStaffCommandStatus> {
        self.status.command_status()
    }

    fn from_command(
        command: &TaskBoardStaffCommand,
        status: TaskBoardStaffCommandStatus,
        timestamp_ms: u64,
        message: impl Into<String>,
        orchestration: Option<TaskBoardStaffOrchestrationContext>,
    ) -> Self {
        let message = event_message(status, message.into(), orchestration.as_ref());
        Self::command_event(
            command.command_id().to_string(),
            command.task_id().map(ToString::to_string),
            command.member_id().map(ToString::to_string),
            status,
            timestamp_ms,
            message,
            orchestration,
        )
    }
}

fn event_message(
    status: TaskBoardStaffCommandStatus,
    message: String,
    orchestration: Option<&TaskBoardStaffOrchestrationContext>,
) -> String {
    let Some(orchestration) = orchestration else {
        return message;
    };
    match status {
        TaskBoardStaffCommandStatus::Applied => format!(
            "{}阶段已交给{}：{}",
            orchestration.stage_label, orchestration.member_level_label, message
        ),
        TaskBoardStaffCommandStatus::Failed => format!(
            "{}阶段{}处理失败：{}",
            orchestration.stage_label, orchestration.member_level_label, message
        ),
        _ => message,
    }
}

fn staff_level_label(level: TaskBoardStaffLevel) -> &'static str {
    match level {
        TaskBoardStaffLevel::Principal => "主智能体",
        TaskBoardStaffLevel::Manager => "管理智能体",
        TaskBoardStaffLevel::Executor => "执行智能体",
        TaskBoardStaffLevel::Member => "普通智能体",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CommandQueueScope {
    Member(String),
    Global,
}

impl CommandQueueScope {
    fn for_command(command: &TaskBoardStaffCommand) -> Self {
        command
            .member_id()
            .map(|member_id| CommandQueueScope::Member(member_id.to_string()))
            .unwrap_or(CommandQueueScope::Global)
    }
}

/// 内存员工命令队列，按成员串行 claim。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskBoardStaffCommandQueue {
    pending: VecDeque<String>,
    commands: HashMap<String, TaskBoardStaffCommand>,
    statuses: HashMap<String, TaskBoardStaffCommandStatus>,
    orchestration_contexts: HashMap<String, TaskBoardStaffOrchestrationContext>,
    inflight_scopes: HashMap<CommandQueueScope, String>,
    known_command_ids: HashSet<String>,
}

impl TaskBoardStaffCommandQueue {
    pub fn remember_orchestration_context(
        &mut self,
        command_id: &str,
        context: TaskBoardStaffOrchestrationContext,
    ) {
        let command_id = command_id.trim();
        if !command_id.is_empty() {
            self.orchestration_contexts.insert(command_id.to_string(), context);
        }
    }

    pub fn enqueue(
        &mut self,
        command: TaskBoardStaffCommand,
        timestamp_ms: u64,
    ) -> TaskBoardStaffEvent {
        let command_id = command.command_id().trim().to_string();
        if command_id.is_empty() {
            return TaskBoardStaffEvent::from_command(
                &command,
                TaskBoardStaffCommandStatus::Skipped,
                timestamp_ms,
                "员工命令缺少 command id，已跳过",
                None,
            );
        }

        let orchestration = self.orchestration_context_for(&command);
        if let Some(group) = task_command_group(&command) {
            if command.task_id().is_some_and(|task_id| self.has_active_task_command(task_id, group))
            {
                return TaskBoardStaffEvent::from_command(
                    &command,
                    TaskBoardStaffCommandStatus::Skipped,
                    timestamp_ms,
                    format!("{} 已入队，请等待当前命令完成", command.label()),
                    orchestration,
                );
            }
        }

        if self.known_command_ids.contains(&command_id) {
            return TaskBoardStaffEvent::from_command(
                &command,
                TaskBoardStaffCommandStatus::Skipped,
                timestamp_ms,
                format!("重复员工命令 {}，已跳过", command_id),
                orchestration,
            );
        }

        self.known_command_ids.insert(command_id.clone());
        self.statuses.insert(command_id.clone(), TaskBoardStaffCommandStatus::Pending);
        self.commands.insert(command_id.clone(), command.clone());
        self.pending.push_back(command_id);

        TaskBoardStaffEvent::from_command(
            &command,
            TaskBoardStaffCommandStatus::Pending,
            timestamp_ms,
            format!("{} 已进入员工命令队列", command.label()),
            orchestration,
        )
    }

    pub fn claim_next(
        &mut self,
        timestamp_ms: u64,
    ) -> Option<(TaskBoardStaffCommand, TaskBoardStaffEvent)> {
        let position = self.next_claimable_position()?;
        let command_id = self.pending.remove(position)?;
        let command = self.commands.get(&command_id)?.clone();
        let scope = CommandQueueScope::for_command(&command);
        self.inflight_scopes.insert(scope, command_id.clone());
        self.statuses.insert(command_id, TaskBoardStaffCommandStatus::Inflight);
        let event = TaskBoardStaffEvent::from_command(
            &command,
            TaskBoardStaffCommandStatus::Inflight,
            timestamp_ms,
            format!("{} 正在校验", command.label()),
            self.orchestration_context_for(&command),
        );
        Some((command, event))
    }

    pub fn mark_applied(
        &mut self,
        command_id: &str,
        timestamp_ms: u64,
        message: impl Into<String>,
    ) -> Option<TaskBoardStaffEvent> {
        self.finish_command(command_id, TaskBoardStaffCommandStatus::Applied, timestamp_ms, message)
    }

    pub fn mark_failed(
        &mut self,
        command_id: &str,
        timestamp_ms: u64,
        message: impl Into<String>,
    ) -> Option<TaskBoardStaffEvent> {
        self.finish_command(command_id, TaskBoardStaffCommandStatus::Failed, timestamp_ms, message)
    }

    pub fn has_claimable_pending(&self) -> bool {
        self.next_claimable_position().is_some()
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn status_for(&self, command_id: &str) -> Option<TaskBoardStaffCommandStatus> {
        self.statuses.get(command_id).copied()
    }

    pub fn has_active_task_command(
        &self,
        task_id: &str,
        group: TaskBoardStaffCommandGroup,
    ) -> bool {
        let task_id = task_id.trim();
        !task_id.is_empty()
            && self.commands.iter().any(|(command_id, command)| {
                self.status_for(command_id).is_some_and(command_is_active)
                    && command.task_id() == Some(task_id)
                    && task_command_group(command) == Some(group)
            })
    }

    fn finish_command(
        &mut self,
        command_id: &str,
        status: TaskBoardStaffCommandStatus,
        timestamp_ms: u64,
        message: impl Into<String>,
    ) -> Option<TaskBoardStaffEvent> {
        if self.status_for(command_id) != Some(TaskBoardStaffCommandStatus::Inflight) {
            return None;
        }
        let command = self.commands.get(command_id)?.clone();
        let scope = CommandQueueScope::for_command(&command);
        if self.inflight_scopes.get(&scope).is_some_and(|value| value == command_id) {
            self.inflight_scopes.remove(&scope);
        }
        self.statuses.insert(command_id.to_string(), status);
        Some(TaskBoardStaffEvent::from_command(
            &command,
            status,
            timestamp_ms,
            message,
            self.orchestration_context_for(&command),
        ))
    }

    fn next_claimable_position(&self) -> Option<usize> {
        let mut candidate: Option<(usize, &TaskBoardStaffCommand)> = None;
        for (index, command_id) in self.pending.iter().enumerate() {
            let Some(command) = self.commands.get(command_id) else {
                continue;
            };
            if self.scope_is_inflight(command) {
                continue;
            }
            candidate = match candidate {
                Some((_, existing))
                    if command.member_id().is_none()
                        && existing.member_id().is_none()
                        && command.command_id() < existing.command_id() =>
                {
                    Some((index, command))
                }
                None => Some((index, command)),
                Some(value) => Some(value),
            };
        }
        candidate.map(|(index, _)| index)
    }

    fn scope_is_inflight(&self, command: &TaskBoardStaffCommand) -> bool {
        self.inflight_scopes.contains_key(&CommandQueueScope::for_command(command))
    }

    fn orchestration_context_for(
        &self,
        command: &TaskBoardStaffCommand,
    ) -> Option<TaskBoardStaffOrchestrationContext> {
        self.orchestration_contexts.get(command.command_id()).cloned()
    }
}

fn command_is_active(status: TaskBoardStaffCommandStatus) -> bool {
    matches!(status, TaskBoardStaffCommandStatus::Pending | TaskBoardStaffCommandStatus::Inflight)
}

fn task_command_group(command: &TaskBoardStaffCommand) -> Option<TaskBoardStaffCommandGroup> {
    match command {
        TaskBoardStaffCommand::AssignTask { .. } | TaskBoardStaffCommand::TakeOverTask { .. } => {
            Some(TaskBoardStaffCommandGroup::Assignment)
        }
        TaskBoardStaffCommand::StartTask { .. } => Some(TaskBoardStaffCommandGroup::Start),
        TaskBoardStaffCommand::PauseTask { .. } | TaskBoardStaffCommand::ResumeTask { .. } => {
            Some(TaskBoardStaffCommandGroup::PauseResume)
        }
        TaskBoardStaffCommand::OpenTaskLogs { .. }
        | TaskBoardStaffCommand::FilterByMember { .. } => None,
    }
}
