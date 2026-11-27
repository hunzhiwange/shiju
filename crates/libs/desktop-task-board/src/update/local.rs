//! 拖放、筛选、草稿和编辑器的局部消息更新。
use crate::message::{TaskBoardDetailTab, TaskBoardMessage, TaskBoardSettingsModalTab};
use iced::widget::text_editor;
use std::collections::{HashMap, HashSet};

/// 本地交互更新借用的看板字段；不包含应用或跨域状态。
pub struct LocalState<'a> {
    pub worktree_pixel_office: &'a mut bool,
    pub project_path: Option<&'a str>,
    pub clear_prompt_after_create: &'a mut bool,
    pub close_after_create: &'a mut bool,
    pub close_after_edit: &'a mut bool,
    pub settings: &'a desktop_task_model::TaskBoardSettings,
    pub logs_auto_scroll: &'a mut bool,
    pub editing_task_id: &'a mut Option<String>,
    pub detail_tab: &'a mut TaskBoardDetailTab,
    pub last_model: &'a mut String,
    pub create_modal_open: &'a mut bool,
    pub tasks: &'a mut Vec<desktop_task_model::Task>,
    pub bulk_acp_agent: &'a mut Option<String>,
    pub bulk_agent: &'a mut String,
    pub bulk_executor_popover: &'a mut bool,
    pub bulk_priority_input: &'a mut String,
    pub bulk_reasoning_effort: &'a mut Option<vw_api_chat_identity::ReasoningEffortDto>,
    pub column_has_vertical_scrollbar: &'a mut HashMap<desktop_task_model::TaskStatus, bool>,
    pub context_menu: &'a mut Option<(String, f32, f32)>,
    pub create_submit_success: &'a mut bool,
    pub desc_editor: &'a mut text_editor::Content,
    pub draft: &'a mut desktop_task_model::TaskDraft,
    pub drag_pending: &'a mut Option<(String, desktop_task_model::TaskStatus, iced::Point)>,
    pub dragging: &'a mut Option<(String, desktop_task_model::TaskStatus)>,
    pub edit_submit_success: &'a mut bool,
    pub executor_popover: &'a mut bool,
    pub filter_priority: &'a mut Option<(u32, u32)>,
    pub filter_status: &'a mut Option<desktop_task_model::TaskStatus>,
    pub import_editor: &'a mut text_editor::Content,
    pub is_import_mode: &'a mut bool,
    pub last_acp_agent: &'a mut Option<String>,
    pub logs_context_menu_open: &'a mut bool,
    pub logs_context_menu_pos: &'a mut Option<(f32, f32)>,
    pub prompt_editor: &'a mut text_editor::Content,
    pub selected_task: &'a mut Option<String>,
    pub selected_tasks: &'a mut HashSet<String>,
    pub settings_modal_open: &'a mut bool,
    pub settings_modal_tab: &'a mut TaskBoardSettingsModalTab,
    pub sort_ascending: &'a mut bool,
    pub sort_by_priority: &'a mut bool,
}

/// 日志编辑器同步由既有日志状态入口完成。
pub enum LocalEffect {
    None,
    OpenDetail(desktop_task_model::Task),
    CloseDetail,
    Persist(desktop_task_model::Task),
    OpenSession(String),
    SyncLogs,
    Import(Result<Vec<desktop_task_model::Task>, String>),
    ImportLoaded(Result<(), String>),
    Preference(&'static str, bool),

    Create(desktop_task_model::Task),
    Delete(String),
    Archive(Vec<String>),
    Move(String, desktop_task_model::TaskStatus, Option<i64>),
    ClearSuccess(bool),
}

/// 未消费的跨域消息交回桌面编排层。
pub fn update_local(
    mut state: LocalState<'_>,
    message: TaskBoardMessage,
    main_agent_key: &str,
) -> Result<LocalEffect, TaskBoardMessage> {
    match message {
        TaskBoardMessage::OpenTaskInSession { task_id } => {
            if state.tasks.iter().any(|task| task.id == task_id) {
                return Ok(LocalEffect::OpenSession(task_id));
            }
        }

        TaskBoardMessage::OpenTaskDetail { project_path, task_id } => {
            if !crate::update::projection::task_board_project_matches(
                state.project_path,
                &project_path,
            ) {
                return Ok(LocalEffect::None);
            }
            return Ok(open_detail(
                &mut state,
                task_id,
                TaskBoardDetailTab::Basic,
                main_agent_key,
            ));
        }
        TaskBoardMessage::ViewTaskLogs { project_path, task_id } => {
            if !crate::update::projection::task_board_project_matches(
                state.project_path,
                &project_path,
            ) {
                return Ok(LocalEffect::None);
            }
            if state.editing_task_id.as_ref() == Some(&task_id) {
                *state.detail_tab = TaskBoardDetailTab::Logs;
                *state.logs_auto_scroll = true;
                return Ok(LocalEffect::SyncLogs);
            }
            return Ok(open_detail(
                &mut state,
                task_id,
                TaskBoardDetailTab::Logs,
                main_agent_key,
            ));
        }
        TaskBoardMessage::SelectDetailTab(tab) => {
            *state.detail_tab = tab;
            if tab == TaskBoardDetailTab::Logs {
                *state.logs_auto_scroll = true;
                return Ok(LocalEffect::SyncLogs);
            }
        }

        TaskBoardMessage::ToggleClearPromptAfterCreate(value) => {
            *state.clear_prompt_after_create = value;
            return Ok(LocalEffect::Preference("task_board_clear_prompt_after_create", value));
        }
        TaskBoardMessage::ToggleCloseAfterCreate(value) => {
            *state.close_after_create = value;
            return Ok(LocalEffect::Preference("task_board_close_after_create", value));
        }
        TaskBoardMessage::ToggleCloseAfterEdit(value) => {
            *state.close_after_edit = value;
            return Ok(LocalEffect::Preference("task_board_close_after_edit", value));
        }
        TaskBoardMessage::ToggleWorktreePixelOffice(value) => {
            *state.worktree_pixel_office = value;
            return Ok(LocalEffect::Preference("task_board_worktree_pixel_office", value));
        }
        TaskBoardMessage::ImportFileLoaded(result) => {
            return Ok(LocalEffect::ImportLoaded(result.map(|content| {
                *state.import_editor = text_editor::Content::with_text(&content);
            })));
        }
        TaskBoardMessage::ImportTasksSubmitted => {
            return Ok(LocalEffect::Import(crate::update::forms::tasks_from_import_content(
                &state.import_editor.text(),
                state.settings.default_priority,
                state.draft,
                crate::draft::initial_task_status_for_workflow_id(&state.settings.workflow_id),
                main_agent_key,
            )));
        }

        TaskBoardMessage::TaskCreated { project_path, task } => {
            if !crate::update::projection::task_board_project_matches(
                state.project_path,
                &project_path,
            ) {
                return Ok(LocalEffect::None);
            }
            state.tasks.push(task);
            crate::bulk::prune_bulk_selection(state.tasks, state.selected_tasks);
            *state.create_submit_success = true;
            if *state.clear_prompt_after_create {
                state.draft.description.clear();
                state.draft.prompt.clear();
                *state.prompt_editor = text_editor::Content::new();
            }
            if *state.close_after_create {
                *state.create_modal_open = false;
                *state.create_submit_success = false;
                reset_draft(&mut state);
            } else {
                return Ok(LocalEffect::ClearSuccess(true));
            }
        }
        TaskBoardMessage::EditingTaskSaved { project_path, task_id, result } => {
            if result.is_ok()
                && crate::update::projection::task_board_project_matches(state.project_path, &project_path)
                && state.editing_task_id.as_ref() == Some(&task_id)
            {
                if *state.close_after_edit {
                    return Ok(close_detail(&mut state));
                }
                *state.edit_submit_success = true;
                return Ok(LocalEffect::ClearSuccess(false));
            }
        }
        TaskBoardMessage::SaveEditingTask => {
            if let Some(id) = state.editing_task_id.as_ref()
                && let Some(task) = state.tasks.iter_mut().find(|t| &t.id == id) {
                    crate::update::forms::apply_task_draft(
                        task,
                        state.draft,
                        main_agent_key,
                        runtime_time::now_ms(),
                    );
                    return Ok(LocalEffect::Persist(task.clone()));
                }
        }
        TaskBoardMessage::CreateTaskSubmitted => {
            if let Some(task) = crate::update::forms::task_from_draft(
                state.draft,
                crate::draft::initial_task_status_for_workflow_id(&state.settings.workflow_id),
                main_agent_key,
            ) {
                return Ok(LocalEffect::Create(task));
            }
        }
        TaskBoardMessage::DuplicateTask(id) => {
            if let Some(task) = state.tasks.iter().find(|t| t.id == id).cloned() {
                return Ok(LocalEffect::Create(crate::update::forms::duplicate_task(
                    task,
                    crate::draft::initial_task_status_for_workflow_id(&state.settings.workflow_id),
                )));
            }
        }
        TaskBoardMessage::TaskDeleted(id) => {
            state.selected_tasks.remove(&id);
            return Ok(LocalEffect::Delete(id));
        }
        TaskBoardMessage::TaskArchived(id) => {
            state.selected_tasks.remove(&id);
            return Ok(LocalEffect::Archive(vec![id]));
        }
        TaskBoardMessage::ArchiveCompletedTasks => {
            *state.context_menu = None;
            return Ok(LocalEffect::Archive(crate::update::rules::completed_task_ids(state.tasks)));
        }
        TaskBoardMessage::DropOnStatus { to_status, insert_index } => {
            if let Some((id, _)) = state.dragging.take() {
                *state.drag_pending = None;
                if let Some(index) = crate::update::rules::drop_insert_index(
                    state.tasks,
                    &id,
                    to_status,
                    insert_index,
                ) {
                    return Ok(LocalEffect::Move(
                        id,
                        to_status,
                        Some(i64::try_from(index).unwrap_or(i64::MAX)),
                    ));
                }
            }
        }

        TaskBoardMessage::EditTask(task_id) => {
            return Ok(open_detail(
                &mut state,
                task_id,
                TaskBoardDetailTab::Basic,
                main_agent_key,
            ));
        }
        TaskBoardMessage::CloseTaskLogs => return Ok(close_detail(&mut state)),
        TaskBoardMessage::CloseTaskPanel => {
            *state.create_modal_open = false;
            *state.create_submit_success = false;
            return Ok(close_detail(&mut state));
        }
        TaskBoardMessage::TaskUpdated(task) => {
            if let Some(existing) = state.tasks.iter_mut().find(|t| t.id == task.id) {
                *existing = task.clone();
            }
            crate::bulk::prune_bulk_selection(state.tasks, state.selected_tasks);
        }
        TaskBoardMessage::InsertDemoData(template) => {
            if let Some(content) = crate::draft::import_demo_content(&template) {
                *state.import_editor = text_editor::Content::with_text(content);
            }
        }
        TaskBoardMessage::CardReleased { task_id } => {
            if state.dragging.is_some() {
                *state.dragging = None;
                *state.drag_pending = None;
            } else if state.drag_pending.take().is_some() {
                return Ok(open_detail(
                    &mut state,
                    task_id,
                    TaskBoardDetailTab::Basic,
                    main_agent_key,
                ));
            }
        }
        TaskBoardMessage::DragStarted { task_id, from_status } => {
            (*state.dragging) = Some((task_id, from_status));
            (*state.drag_pending) = None;
        }
        TaskBoardMessage::DragPending { task_id, from_status, press_position } => {
            (*state.drag_pending) = Some((task_id, from_status, press_position));
        }
        TaskBoardMessage::DragEnded => {
            (*state.dragging) = None;
            (*state.drag_pending) = None;
        }
        TaskBoardMessage::SelectTask(task_id) => {
            (*state.selected_task) = Some(task_id);
        }
        TaskBoardMessage::DeselectTask => {
            (*state.selected_task) = None;
        }
        TaskBoardMessage::FilterByStatus(status) => {
            (*state.filter_status) = status;
        }
        TaskBoardMessage::FilterByPriority(range) => {
            (*state.filter_priority) = range;
        }
        TaskBoardMessage::SortByPriority(ascending) => {
            (*state.sort_by_priority) = true;
            (*state.sort_ascending) = ascending;
        }
        TaskBoardMessage::SortByDate(ascending) => {
            (*state.sort_by_priority) = false;
            (*state.sort_ascending) = ascending;
        }
        TaskBoardMessage::ContextMenuOpened { task_id, x, y } => {
            (*state.context_menu) = Some((task_id, x, y));
        }
        TaskBoardMessage::ContextMenuClosed => {
            (*state.context_menu) = None;
        }
        TaskBoardMessage::UpdateDraftPriority(v) => {
            state.draft.priority = v;
        }
        TaskBoardMessage::UpdateDraftAssignee(v) => {
            state.draft.assignee = v;
        }
        TaskBoardMessage::UpdateDraftReasoningEffort(reasoning_effort) => {
            state.draft.reasoning_effort = reasoning_effort;
        }
        TaskBoardMessage::UpdateDraftDescription(v) => {
            state.draft.description = v;
        }
        TaskBoardMessage::UpdateDraftPrompt(v) => {
            state.draft.prompt = v;
        }
        TaskBoardMessage::UpdateDraftAutoPromoteDelay(v) => {
            state.draft.auto_promote_delay_seconds = v;
        }
        TaskBoardMessage::UpdateEditingTaskPriority(priority) => {
            state.draft.priority = priority;
        }
        TaskBoardMessage::UpdateEditingTaskAgent(agent) => {
            state.draft.agent = agent.or_else(|| Some(main_agent_key.to_string()));
        }
        TaskBoardMessage::UpdateEditingTaskReasoningEffort(reasoning_effort) => {
            state.draft.reasoning_effort = reasoning_effort;
        }
        TaskBoardMessage::UpdateEditingTaskPrompt(prompt) => {
            state.draft.prompt = prompt;
        }
        TaskBoardMessage::ClearCreateSubmitSuccess => {
            (*state.create_submit_success) = false;
        }
        TaskBoardMessage::ClearEditSubmitSuccess => {
            (*state.edit_submit_success) = false;
        }
        TaskBoardMessage::DescEditorAction(action) => {
            (*state.desc_editor).perform(action);
            state.draft.description = (*state.desc_editor).text().to_string();
        }
        TaskBoardMessage::PromptEditorAction(action) => {
            (*state.prompt_editor).perform(action);
            state.draft.prompt = (*state.prompt_editor).text().to_string();
        }
        TaskBoardMessage::LogsViewerOpenContextMenu { x, y } => {
            (*state.logs_context_menu_open) = true;
            (*state.logs_context_menu_pos) = Some((x, y));
        }
        TaskBoardMessage::ToggleExecutorPopover => {
            (*state.executor_popover) = !(*state.executor_popover);
            if *state.executor_popover {
                (*state.bulk_executor_popover) = false;
            }
        }
        TaskBoardMessage::CloseExecutorPopover => {
            *state.executor_popover = false;
        }
        TaskBoardMessage::ToggleTaskSelection { task_id, selected } => {
            if selected {
                (*state.selected_tasks).insert(task_id);
            } else {
                (*state.selected_tasks).remove(&task_id);
            }
        }
        TaskBoardMessage::UpdateBulkPriorityInput(value) => {
            (*state.bulk_priority_input) = value;
        }
        TaskBoardMessage::BulkAgentSelected(agent) => {
            (*state.bulk_agent) =
                if agent.trim().is_empty() { main_agent_key.to_string() } else { agent };
        }
        TaskBoardMessage::UpdateBulkReasoningEffort(reasoning_effort) => {
            (*state.bulk_reasoning_effort) = reasoning_effort;
        }
        TaskBoardMessage::ToggleBulkExecutorPopover => {
            (*state.bulk_executor_popover) = !(*state.bulk_executor_popover);
            if *state.bulk_executor_popover {
                (*state.executor_popover) = false;
            }
        }
        TaskBoardMessage::BulkExecutorSelected(executor) => {
            (*state.bulk_acp_agent) = executor.clone();
            (*state.last_acp_agent) = executor;
            (*state.bulk_executor_popover) = false;
        }
        TaskBoardMessage::CloseBulkExecutorPopover => {
            (*state.bulk_executor_popover) = false;
        }
        TaskBoardMessage::CloseSettingsModal => {
            (*state.settings_modal_open) = false;
        }
        TaskBoardMessage::SelectSettingsModalTab(tab) => {
            (*state.settings_modal_tab) = tab;
        }
        TaskBoardMessage::ColumnScrollChanged { status, has_vertical_scrollbar } => {
            (*state.column_has_vertical_scrollbar).insert(status, has_vertical_scrollbar);
        }
        TaskBoardMessage::ToggleImportMode(enabled) => {
            (*state.is_import_mode) = enabled;
            *state.executor_popover = false;
        }
        TaskBoardMessage::ImportEditorAction(action) => {
            (*state.import_editor).perform(action);
        }
        TaskBoardMessage::ClearImportEditor => {
            (*state.import_editor) = iced::widget::text_editor::Content::new();
        }
        TaskBoardMessage::SelectAllTasksInStatus(status) => {
            for task_id in crate::bulk::visible_task_ids_for_status(state.tasks, status) {
                (*state.selected_tasks).insert(task_id);
            }
        }
        TaskBoardMessage::InvertTaskSelectionInStatus(status) => {
            for task_id in crate::bulk::visible_task_ids_for_status(state.tasks, status) {
                if !(*state.selected_tasks).remove(&task_id) {
                    (*state.selected_tasks).insert(task_id);
                }
            }
        }
        TaskBoardMessage::OpenSettingsModal => {
            (*state.settings_modal_open) = true;
            (*state.settings_modal_tab) = TaskBoardSettingsModalTab::default();
        }
        TaskBoardMessage::UpdateEditingTaskExecutor(executor) => {
            crate::draft::set_draft_executor_selection(state.draft, executor);
            if state.draft.acp_agent.is_some() {
                state.draft.agent = Some(main_agent_key.to_owned());
            }
            (*state.executor_popover) = false;
        }
        TaskBoardMessage::SelectDraftAgent { agent, acp_agent } => {
            if state.draft.agent.as_deref() != Some(agent.as_str())
                || state.draft.acp_agent != acp_agent
            {
                state.draft.reasoning_effort = None;
            }
            state.draft.agent = Some(agent);
            crate::draft::set_draft_executor_selection(state.draft, acp_agent);
            (*state.last_acp_agent) = state.draft.acp_agent.clone();
            (*state.last_model).clone_from(&state.draft.model);
            (*state.executor_popover) = false;
        }
        TaskBoardMessage::CreateTaskCancelled => {
            (*state.create_modal_open) = false;
            (*state.create_submit_success) = false;
            reset_draft(&mut state);
        }
        TaskBoardMessage::CreateTask => {
            (*state.create_modal_open) = true;
            (*state.create_submit_success) = false;
            reset_draft(&mut state);
        }
        other => return Err(other),
    }
    Ok(LocalEffect::None)
}

fn open_detail(
    state: &mut LocalState<'_>,
    task_id: String,
    tab: TaskBoardDetailTab,
    main_agent_key: &str,
) -> LocalEffect {
    let Some(task) = state.tasks.iter().find(|task| task.id == task_id).cloned() else {
        return LocalEffect::None;
    };

    *state.logs_auto_scroll = true;
    *state.editing_task_id = Some(task_id);
    *state.detail_tab = tab;
    *state.edit_submit_success = false;
    *state.context_menu = None;
    crate::update::forms::draft_from_task(state.draft, &task, main_agent_key);
    *state.prompt_editor = text_editor::Content::with_text(&state.draft.prompt);
    LocalEffect::OpenDetail(task)
}

fn close_detail(state: &mut LocalState<'_>) -> LocalEffect {
    *state.detail_tab = TaskBoardDetailTab::Basic;
    *state.logs_auto_scroll = true;
    *state.editing_task_id = None;
    *state.edit_submit_success = false;
    LocalEffect::CloseDetail
}

fn reset_draft(state: &mut LocalState<'_>) {
    *state.executor_popover = false;
    crate::draft::reset_create_draft(
        state.draft,
        state.prompt_editor,
        state.last_model.clone(),
        state.last_acp_agent.clone(),
    );
}

/// 关闭看板时清除暂态选择；路由和窗口归宿主所有。
pub fn close_board(
    loading: &mut bool,
    active: &mut Option<desktop_task_model::TaskStatus>,
    selected: &mut HashSet<String>,
) {
    *loading = false;
    *active = None;
    selected.clear();
}
