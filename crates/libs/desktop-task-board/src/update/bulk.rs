//! 批量编辑的本地转换与持久化意图。
use crate::message::TaskBoardMessage;
use desktop_task_model::{Task, TaskStatus};
pub struct BulkState<'a> {
    pub task_board_bulk_active_status: &'a mut Option<TaskStatus>,
    pub task_board_bulk_acp_agent: &'a mut Option<String>,
    pub task_board_bulk_agent: &'a mut String,
    pub task_board_bulk_model_input: &'a mut String,
    pub task_board_bulk_priority_input: &'a mut String,
    pub task_board_bulk_reasoning_effort: &'a mut Option<vw_api_chat_identity::ReasoningEffortDto>,
    pub task_board_bulk_task_split_mode: &'a mut desktop_task_model::TaskSplitMode,
    pub task_board_context_menu: &'a mut Option<(String, f32, f32)>,
    pub task_board_draft: &'a mut desktop_task_model::TaskDraft,
    pub task_board_editing_task_id: &'a mut Option<String>,
    pub task_board_last_acp_agent: &'a mut Option<String>,
    pub task_board_last_model: &'a mut String,
    pub task_board_manual_merge_queue: &'a mut Vec<String>,
    pub task_board_selected_tasks: &'a mut std::collections::HashSet<String>,
    pub task_board_tasks: &'a mut Vec<Task>,
}
pub enum BulkEffect {
    None,
    Persist(Vec<Task>, &'static str),
    Archive(Vec<String>),
    Delete(Vec<String>),
    Move(Vec<String>, TaskStatus),
    Merge(Vec<Task>),
}
pub fn update_bulk(
    mut state: BulkState<'_>,
    message: TaskBoardMessage,
    model_options: &[Option<vw_api_chat_identity::ReasoningEffortDto>],
    main_agent_key: &str,
) -> Result<(BulkEffect, Vec<String>), TaskBoardMessage> {
    let mut sync_logs = Vec::new();
    let effect = match message {
        TaskBoardMessage::ToggleBulkSelectionMode(status) => {
            *state.task_board_context_menu = None;
            let active = *state.task_board_bulk_active_status == Some(status);
            state.deactivate();
            if !active {
                *state.task_board_bulk_active_status = Some(status);
                state.reset_inputs(main_agent_key);
            }
            BulkEffect::None
        }
        TaskBoardMessage::BulkSetPriorityInStatus(status) => {
            (*state.task_board_context_menu) = None;
            let mut tasks = crate::bulk::selected_tasks_for_status(
                state.task_board_tasks,
                state.task_board_selected_tasks,
                status,
            );
            if tasks.is_empty() {
                return Ok((BulkEffect::None, sync_logs));
            }

            let priority = crate::draft::parse_priority_or_default(
                &(*state.task_board_bulk_priority_input),
                999,
            );
            (*state.task_board_bulk_priority_input) = priority.to_string();

            crate::update::rules::set_bulk_priority(&mut tasks, priority);

            let task_ids = tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
            crate::bulk::clear_selected_task_ids(state.task_board_selected_tasks, &task_ids);

            BulkEffect::Persist(tasks, "批量设置优先级")
        }
        TaskBoardMessage::BulkSetModelInStatus(status) => {
            (*state.task_board_context_menu) = None;
            let mut tasks = crate::bulk::selected_tasks_for_status(
                state.task_board_tasks,
                state.task_board_selected_tasks,
                status,
            );
            if tasks.is_empty() {
                return Ok((BulkEffect::None, sync_logs));
            }

            let model =
                crate::bulk::normalized_bulk_model_input(&(*state.task_board_bulk_model_input));
            (*state.task_board_bulk_model_input) = model.clone();
            (*state.task_board_bulk_reasoning_effort) =
                crate::update::model::validated_reasoning_effort(
                    (*state.task_board_bulk_acp_agent).as_deref(),
                    (*state.task_board_bulk_reasoning_effort),
                    model_options,
                );
            (*state.task_board_last_model) = model.clone();

            crate::update::rules::set_bulk_model(&mut tasks, &model);

            let task_ids = tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
            crate::bulk::clear_selected_task_ids(state.task_board_selected_tasks, &task_ids);

            BulkEffect::Persist(tasks, "批量设置模型")
        }
        TaskBoardMessage::BulkSetAgentInStatus(status) => {
            (*state.task_board_context_menu) = None;
            let mut tasks = crate::bulk::selected_tasks_for_status(
                state.task_board_tasks,
                state.task_board_selected_tasks,
                status,
            );
            if tasks.is_empty() {
                return Ok((BulkEffect::None, sync_logs));
            }

            let agent = crate::update::rules::normalized_bulk_agent(
                &(*state.task_board_bulk_agent),
                main_agent_key,
            );
            (*state.task_board_bulk_agent) = agent.clone();

            crate::update::rules::set_bulk_agent(&mut tasks, &agent, main_agent_key);

            let task_ids = tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
            crate::bulk::clear_selected_task_ids(state.task_board_selected_tasks, &task_ids);

            BulkEffect::Persist(tasks, "批量设置代理")
        }
        TaskBoardMessage::BulkSetExecutorInStatus { status, executor } => {
            (*state.task_board_context_menu) = None;
            let mut tasks = crate::bulk::selected_tasks_for_status(
                state.task_board_tasks,
                state.task_board_selected_tasks,
                status,
            );
            if tasks.is_empty() {
                return Ok((BulkEffect::None, sync_logs));
            }

            (*state.task_board_bulk_acp_agent) = executor.clone();
            (*state.task_board_last_acp_agent) = executor.clone();

            crate::update::rules::set_bulk_executor(&mut tasks, executor.clone());

            let task_ids = tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
            crate::bulk::clear_selected_task_ids(state.task_board_selected_tasks, &task_ids);

            BulkEffect::Persist(tasks, "批量设置 ACP 智能体")
        }
        TaskBoardMessage::BulkSetReasoningEffortInStatus { status, reasoning_effort } => {
            (*state.task_board_context_menu) = None;
            let task_ids = crate::bulk::selected_task_ids_for_status(
                state.task_board_tasks,
                state.task_board_selected_tasks,
                status,
            );
            if task_ids.is_empty() {
                return Ok((BulkEffect::None, sync_logs));
            }

            (*state.task_board_bulk_reasoning_effort) = reasoning_effort;
            let tasks = crate::update::rules::set_bulk_reasoning_effort(
                &mut (*state.task_board_tasks),
                &task_ids,
                reasoning_effort,
            );

            for task_id in &task_ids {
                sync_logs.push(task_id.clone());
            }
            if (*state.task_board_editing_task_id)
                .as_ref()
                .is_some_and(|editing_task_id| task_ids.contains(editing_task_id))
            {
                (*state.task_board_draft).reasoning_effort = reasoning_effort;
            }
            crate::bulk::clear_selected_task_ids(state.task_board_selected_tasks, &task_ids);

            BulkEffect::Persist(tasks, "批量设置推理强度")
        }
        TaskBoardMessage::BulkSetTaskSplitModeInStatus { status, task_split_mode } => {
            (*state.task_board_context_menu) = None;
            let task_ids = crate::bulk::selected_task_ids_for_status(
                state.task_board_tasks,
                state.task_board_selected_tasks,
                status,
            );
            if task_ids.is_empty() {
                return Ok((BulkEffect::None, sync_logs));
            }

            (*state.task_board_bulk_task_split_mode) = task_split_mode;

            let tasks = crate::update::rules::set_bulk_split_mode(
                &mut (*state.task_board_tasks),
                &task_ids,
                task_split_mode,
            );

            for task_id in &task_ids {
                sync_logs.push(task_id.clone());
            }
            if (*state.task_board_editing_task_id)
                .as_ref()
                .is_some_and(|editing_task_id| task_ids.contains(editing_task_id))
            {
                (*state.task_board_draft).task_split_mode = task_split_mode;
            }
            crate::bulk::clear_selected_task_ids(state.task_board_selected_tasks, &task_ids);

            BulkEffect::Persist(tasks, "批量设置任务拆分")
        }
        TaskBoardMessage::BulkArchiveTasksInStatus(status) => {
            (*state.task_board_context_menu) = None;
            let task_ids = crate::bulk::selected_task_ids_for_status(
                state.task_board_tasks,
                state.task_board_selected_tasks,
                status,
            );
            if task_ids.is_empty() {
                return Ok((BulkEffect::None, sync_logs));
            }
            crate::bulk::clear_selected_task_ids(state.task_board_selected_tasks, &task_ids);
            BulkEffect::Archive(task_ids)
        }
        TaskBoardMessage::BulkDeleteTasksInStatus(status) => {
            (*state.task_board_context_menu) = None;
            let task_ids = crate::bulk::selected_task_ids_for_status(
                state.task_board_tasks,
                state.task_board_selected_tasks,
                status,
            );
            if task_ids.is_empty() {
                return Ok((BulkEffect::None, sync_logs));
            }
            crate::bulk::clear_selected_task_ids(state.task_board_selected_tasks, &task_ids);
            BulkEffect::Delete(task_ids)
        }
        TaskBoardMessage::BulkMoveTasksInStatus { from_status, to_status } => {
            (*state.task_board_context_menu) = None;
            if from_status == to_status {
                return Ok((BulkEffect::None, sync_logs));
            }
            let task_ids = crate::bulk::selected_task_ids_for_status(
                state.task_board_tasks,
                state.task_board_selected_tasks,
                from_status,
            );
            if task_ids.is_empty() {
                return Ok((BulkEffect::None, sync_logs));
            }
            crate::bulk::clear_selected_task_ids(state.task_board_selected_tasks, &task_ids);
            BulkEffect::Move(task_ids, to_status)
        }
        TaskBoardMessage::BulkMergeTasksInStatus(status) => {
            (*state.task_board_context_menu) = None;
            if status != TaskStatus::PrSubmitted {
                return Ok((BulkEffect::None, sync_logs));
            }
            let mut tasks = crate::bulk::selected_tasks_for_status(
                state.task_board_tasks,
                state.task_board_selected_tasks,
                status,
            );
            if tasks.is_empty() {
                return Ok((BulkEffect::None, sync_logs));
            }
            let task_ids = crate::update::rules::prepare_manual_merge_queue(&mut tasks);
            (*state.task_board_manual_merge_queue) = task_ids.clone();
            crate::bulk::clear_selected_task_ids(state.task_board_selected_tasks, &task_ids);

            BulkEffect::Merge(tasks)
        }
        other => return Err(other),
    };
    Ok((effect, sync_logs))
}

impl BulkState<'_> {
    pub fn reset_inputs(&mut self, main_agent_key: &str) {
        *self.task_board_bulk_priority_input =
            crate::draft::parse_priority_or_default(&self.task_board_draft.priority, 999)
                .to_string();
        *self.task_board_bulk_model_input =
            crate::bulk::normalized_bulk_model_input(self.task_board_last_model);
        *self.task_board_bulk_agent =
            self.task_board_draft.agent.clone().unwrap_or_else(|| main_agent_key.to_string());
        *self.task_board_bulk_acp_agent = self.task_board_last_acp_agent.clone();
        *self.task_board_bulk_reasoning_effort = self.task_board_draft.reasoning_effort;
        *self.task_board_bulk_task_split_mode = self.task_board_draft.task_split_mode;
    }
    pub fn deactivate(&mut self) {
        if let Some(status) = self.task_board_bulk_active_status.take() {
            crate::bulk::clear_selection_for_status(
                self.task_board_tasks,
                self.task_board_selected_tasks,
                status,
            );
        }
    }
}

/// 网关确认后的看板刷新与执行唤醒策略。
pub fn completion_messages(queue_empty: bool) -> Vec<TaskBoardMessage> {
    if queue_empty { vec![TaskBoardMessage::LoadTasks] } else { Vec::new() }
}
pub fn move_completion_messages(
    result: &Result<(), String>,
    target: TaskStatus,
    running: bool,
) -> Vec<TaskBoardMessage> {
    let mut messages = vec![TaskBoardMessage::LoadTasks];
    if result.is_ok() && target == TaskStatus::Pending && running {
        messages.push(TaskBoardMessage::ExecutionTick);
    }
    messages
}
