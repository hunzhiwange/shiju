use super::*;
use desktop_task_board::work_views::{
    BatchDraft, ChildTaskDraft, RunHistory, WorkPlanTask, positive_batch,
};
use vw_api_agent_scheduler::{
    AgentSchedulerTaskBoardEventDto, AgentSchedulerTaskPlanChangeDto as Change,
    AgentSchedulerTaskPlanPlacementDto as Placement, AgentSchedulerTaskPlanUpdateRequest,
};

const OUTER_INPUT: &str = "work-plan-outer-batch";
const CHILD_PROMPT_INPUT: &str = "work-plan-child-prompt";

pub(super) fn update(
    app: &mut crate::app::App,
    message: &TaskBoardMessage,
) -> Option<iced::Task<Message>> {
    let project = app.navigation_tabs.project_path.clone();
    let board = &mut app.task_board;
    if board.work_plan.project != project {
        board.work_plan = Default::default();
        board.work_plan.project.clone_from(&project);
    }
    let plan = &mut board.work_plan;
    let project = project?;
    let selected = board.task_board_editing_task_id.as_deref();
    match message {
        TaskBoardMessage::OpenWorkPlanView => plan.open = true,
        TaskBoardMessage::OpenSavedWorkView(_) | TaskBoardMessage::SelectWorkView(_) => {
            plan.open = false;
            return None;
        }
        TaskBoardMessage::TaskCreated { project_path, task } if project == *project_path => {
            plan.tasks.entry(task.id.clone()).or_insert(WorkPlanTask {
                placement: None,
                status: task.status,
                status_label: None,
                blocker: None,
            });
            return None;
        }
        TaskBoardMessage::WorkPlanSearchChanged(value) => plan.search.clone_from(value),
        TaskBoardMessage::ToggleWorkPlanGroup(key) => {
            if !plan.collapsed.remove(key) {
                plan.collapsed.insert(key.clone());
            }
        }
        TaskBoardMessage::EditWorkPlanBatch => {
            let id = selected?;
            if plan.pending_batches.contains_key(id) {
                return Some(iced::Task::none());
            }
            let original = plan.tasks.get(id)?.placement.clone();
            let (outer, inner) = match &original {
                Some(Placement::Root { outer_batch, inner_batch }) => {
                    (outer_batch.to_string(), inner_batch.to_string())
                }
                Some(Placement::Child { .. }) => return Some(iced::Task::none()),
                None => (String::new(), String::new()),
            };
            plan.batch_draft = Some(BatchDraft {
                task_id: id.to_owned(),
                original,
                outer,
                inner,
                pending: None,
                feedback: None,
            });
            return Some(iced::widget::operation::focus(OUTER_INPUT));
        }
        TaskBoardMessage::WorkPlanBatchChanged { outer, value } => {
            let draft = plan.batch_draft.as_mut()?;
            if draft.pending.is_none() && selected == Some(draft.task_id.as_str()) {
                if *outer {
                    draft.outer.clone_from(value);
                } else {
                    draft.inner.clone_from(value);
                }
                draft.feedback = None;
            }
        }
        TaskBoardMessage::SaveWorkPlanBatch { clear } => {
            let draft = plan.batch_draft.as_mut()?;
            if draft.pending.is_some() || selected != Some(draft.task_id.as_str()) {
                return Some(iced::Task::none());
            }
            let current = plan.tasks.get(&draft.task_id).map(|task| &task.placement);
            if current != Some(&draft.original) {
                draft.feedback = Some(Err("批次已被更新，请重新打开批次编辑".into()));
                return Some(iced::Task::none());
            }
            let placement = if *clear {
                Change::Clear
            } else {
                let (Some(outer_batch), Some(inner_batch)) =
                    (positive_batch(&draft.outer), positive_batch(&draft.inner))
                else {
                    draft.feedback = Some(Err("外层和组内批次均须填写正整数".into()));
                    return Some(iced::Task::none());
                };
                Change::Set { placement: Placement::Root { outer_batch, inner_batch } }
            };
            let request = next_agent_scheduler_status_generation();
            draft.pending = Some(request);
            let task_id = draft.task_id.clone();
            plan.pending_batches.insert(task_id.clone(), request);
            let (path, id) = (project.clone(), task_id.clone());
            return Some(iced::Task::perform(
                async move {
                    let scope = resolve_task_board_scheduler_scope(&path).await?;
                    let client = crate::app::config::gateway_client()?;
                    gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
                        .agent_scheduler_update_task_plan(
                            &scope,
                            &id,
                            &AgentSchedulerTaskPlanUpdateRequest { placement },
                        )
                        .await
                },
                move |result| {
                    Message::TaskBoard(TaskBoardMessage::WorkPlanBatchSaved {
                        project_path: project.clone(),
                        task_id: task_id.clone(),
                        request,
                        result,
                    })
                },
            ));
        }
        TaskBoardMessage::WorkPlanBatchSaved { project_path, task_id, request, result } => {
            if project != *project_path {
                return Some(iced::Task::none());
            }
            if plan.pending_batches.get(task_id) != Some(request) {
                return Some(sync_agent_scheduler_status(project));
            }
            plan.pending_batches.remove(task_id);
            let draft = plan
                .batch_draft
                .as_mut()
                .filter(|draft| draft.pending == Some(*request) && draft.task_id == *task_id);
            if let Some(draft) = draft {
                draft.pending = None;
                draft.feedback = Some(result.as_ref().map(|_| ()).map_err(|error| error.clone()));
                if let Ok(snapshot) = result {
                    draft.original.clone_from(&snapshot.placement);
                    if snapshot.placement.is_none() {
                        draft.outer.clear();
                        draft.inner.clear();
                    }
                }
            }
            if let Ok(snapshot) = result {
                if let Some(task) = plan.tasks.get_mut(task_id) {
                    task.placement.clone_from(&snapshot.placement);
                }
                board.task_board_workflow_generation = next_agent_scheduler_status_generation();
            }
            return Some(sync_agent_scheduler_status(project));
        }
        TaskBoardMessage::OpenWorkPlanChild => {
            let parent =
                board.task_board_tasks.iter().find(|task| selected == Some(task.id.as_str()))?;
            if plan.child_draft.is_some()
                || parent.archived
                || parent.deleted
                || !plan.tasks.contains_key(&parent.id)
                || matches!(plan.placement(&parent.id), Some(Placement::Child { .. }))
            {
                return Some(iced::Task::none());
            }
            let status = board
                .task_board_settings
                .workflow()
                .and_then(|workflow| workflow.statuses.into_iter().find(|status| status.enabled))
                .map_or(TaskStatus::Pool, |status| status.status);
            plan.child_draft = Some(ChildTaskDraft::new(project, parent, status));
            return Some(iced::widget::operation::focus(CHILD_PROMPT_INPUT));
        }
        TaskBoardMessage::CloseWorkPlanChild => plan.child_draft = None,
        TaskBoardMessage::WorkPlanChildPromptEdited(action) => {
            let draft = plan.child_draft.as_mut().filter(|draft| !draft.pending)?;
            draft.prompt.perform(action.clone());
            draft.error = None;
        }
        TaskBoardMessage::WorkPlanChildBatchChanged(value) => {
            let draft = plan.child_draft.as_mut().filter(|draft| !draft.pending)?;
            draft.batch.clone_from(value);
            draft.error = None;
        }
        TaskBoardMessage::CreateWorkPlanChild => {
            let draft = plan.child_draft.as_mut().filter(|draft| !draft.pending)?;
            if draft.project != project || selected != Some(draft.parent_id.as_str()) {
                return Some(iced::Task::none());
            }
            let Some((task, placement)) = draft.build() else {
                draft.error = Some("请填写任务提示词和正整数批次".into());
                return Some(iced::Task::none());
            };
            draft.pending = true;
            let mut request =
                agent_scheduler_create_task_request(&task, &board.task_board_settings.workflow_id);
            request.placement = Some(placement.clone());
            let path = project.clone();
            return Some(iced::Task::perform(
                async move {
                    let scope = resolve_task_board_scheduler_scope(&path).await?;
                    let client = crate::app::config::gateway_client()?;
                    let created =
                        gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
                            .agent_scheduler_create_task(&scope, &request)
                            .await?;
                    if created.task_id != request.task_id
                        || created.task.placement != request.placement
                    {
                        return Err("服务端返回的子任务归属与请求不符".into());
                    }
                    Ok(())
                },
                move |result| {
                    Message::TaskBoard(TaskBoardMessage::WorkPlanChildCreated {
                        project_path: project.clone(),
                        task: task.clone(),
                        placement: placement.clone(),
                        result,
                    })
                },
            ));
        }
        TaskBoardMessage::WorkPlanChildCreated { project_path, task, placement, result } => {
            if project != *project_path {
                return Some(iced::Task::none());
            }
            let active = plan.child_draft.as_mut().filter(|draft| draft.task.id == task.id);
            if let Err(error) = result {
                if let Some(draft) = active {
                    draft.pending = false;
                    draft.error = Some(error.clone());
                }
                return Some(iced::Task::none());
            }
            let open_child = active.is_some();
            if open_child {
                plan.child_draft = None;
            }
            plan.tasks.entry(task.id.clone()).or_insert_with(|| taskboard::work_views::WorkPlanTask {
                placement: Some(placement.clone()),
                status: task.status,
                status_label: None,
                blocker: Some(vw_api_agent_scheduler::AgentSchedulerDispatchBlockerDto::WorkPlanNotPublished),
            });
            if !board.task_board_tasks.iter().any(|existing| existing.id == task.id) {
                board.task_board_tasks.push(task.clone());
            }
            board.task_board_workflow_generation = next_agent_scheduler_status_generation();
            let open = if open_child {
                super::update(
                    app,
                    TaskBoardMessage::OpenTaskDetail {
                        project_path: project.clone(),
                        task_id: task.id.clone(),
                    },
                )
            } else {
                iced::Task::none()
            };
            return Some(iced::Task::batch([open, sync_agent_scheduler_status(project)]));
        }
        TaskBoardMessage::LoadWorkPlanHistory { include_children } => {
            let Some(id) = selected else {
                return Some(iced::Task::none());
            };
            let mut ids = std::collections::HashSet::from([id.to_owned()]);
            if *include_children {
                ids.extend(plan.tasks.iter().filter_map(
                    |(child_id, task)| match &task.placement {
                        Some(Placement::Child { parent_task_id, .. }) if parent_task_id == id => {
                            Some(child_id.clone())
                        }
                        _ => None,
                    },
                ));
            }
            let request = next_agent_scheduler_status_generation();
            plan.history = Some(RunHistory {
                task_id: id.to_owned(),
                request,
                include_children: *include_children,
                result: None,
            });
            let path = project.clone();
            return Some(iced::Task::perform(
                async move { load_history(&path, &ids).await },
                move |result| {
                    Message::TaskBoard(TaskBoardMessage::WorkPlanHistoryLoaded {
                        project_path: project.clone(),
                        request,
                        result,
                    })
                },
            ));
        }
        TaskBoardMessage::WorkPlanHistoryLoaded { project_path, request, result } => {
            if project == *project_path
                && let Some(history) = &mut plan.history
                && history.request == *request
                && selected == Some(history.task_id.as_str())
            {
                history.result = Some(result.clone());
            }
        }
        _ => return None,
    };
    Some(iced::Task::none())
}

async fn load_history(
    project: &str,
    task_ids: &std::collections::HashSet<String>,
) -> Result<Vec<AgentSchedulerTaskBoardEventDto>, String> {
    let scope = resolve_task_board_scheduler_scope(project).await?;
    let client = crate::app::config::gateway_client()?;
    let client = gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone());
    let mut cursor = None;
    let mut runs = std::collections::HashMap::new();
    loop {
        let page = client.agent_scheduler_events(&scope, cursor).await?;
        if page.events.is_empty() {
            break;
        }
        if cursor.is_some_and(|previous| page.cursor <= previous) {
            return Err("执行记录游标未推进".into());
        }
        cursor = Some(page.cursor);
        for event in page.events {
            if let Some(run_id) = event.run_id.as_ref()
                && task_ids.contains(event.task_id.as_ref())
            {
                runs.insert((event.task_id.0.clone(), run_id.clone()), event);
            }
        }
    }
    let mut records: Vec<_> = runs.into_values().collect();
    records.sort_by_key(|record| std::cmp::Reverse(record.cursor));
    Ok(records)
}
