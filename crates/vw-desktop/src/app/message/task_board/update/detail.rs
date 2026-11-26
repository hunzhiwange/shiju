use super::{
    next_agent_scheduler_status_generation, resolve_task_board_scheduler_scope,
    sync_task_board_gateway_snapshot,
};
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};
use desktop_task_board::detail::{Event, Field, Operation, State};
use iced::Task;
use iced::widget::text_editor;
use vw_api_agent_scheduler::AgentSchedulerTaskDetailChange as Change;

pub(super) fn open(app: &mut App) -> Task<Message> {
    app.task_board.detail = State::default();
    let (Some(project), Some(task)) = (
        app.navigation_tabs.project_path.as_ref(),
        app.task_board.task_board_viewing_logs.as_ref(),
    ) else {
        return Task::none();
    };
    app.task_board.detail.project.clone_from(project);
    app.task_board.detail.task_id.clone_from(&task.id);
    request(app, Operation::Load, None)
}

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    let state = &mut app.task_board.detail;
    let Some(task) = app.task_board.task_board_viewing_logs.as_ref().filter(|task| {
        task.id == state.task_id
            && app.navigation_tabs.project_path.as_deref() == Some(state.project.as_str())
    }) else {
        return Task::none();
    };
    match event {
        Event::Loaded { request: id, result } => {
            let Some((_, operation)) = state.request.filter(|(pending, _)| *pending == id) else {
                return Task::none();
            };
            state.request = None;
            let data = match result {
                Ok(data) => data,
                Err(error) => {
                    state.load_failed = operation == Operation::Load && state.data.is_none();
                    return app.show_error_toast(error);
                }
            };
            state.load_failed = false;
            state.data = Some(data);
            if operation == Operation::PostComment {
                state.comment = text_editor::Content::new();
                state.comment_id = None;
                state.reply_to = None;
            }
            if operation == Operation::SaveField {
                let board_field = matches!(state.editor, Some(Field::Assignee | Field::Priority));
                state.editor = None;
                state.input.clear();
                if board_field {
                    let path = state.project.clone();
                    app.task_board.task_board_workflow_generation =
                        next_agent_scheduler_status_generation();
                    return sync_task_board_gateway_snapshot(path);
                }
            }
        }
        Event::Reload if state.request.is_none() => {
            let logs = super::sync_agent_scheduler_logs_for_project_path(
                state.project.clone(),
                state.task_id.clone(),
            );
            return Task::batch([request(app, Operation::Load, None), logs]);
        }
        Event::EditComment(action) if !posting(state) => {
            if action.is_edit() {
                state.comment_id = None;
            }
            state.comment.perform(action);
        }
        Event::ReplyTo(id) if !posting(state) => {
            if state.reply_to != id {
                state.comment_id = None;
                state.reply_to = id;
            }
            return iced::widget::operation::focus("task-detail-comment");
        }
        Event::PostComment if state.request.is_none() && state.data.is_some() => {
            let body = state.comment.text();
            if body.trim().is_empty() {
                return Task::none();
            }
            let id = match &state.comment_id {
                Some(id) => id.clone(),
                None => match ordered_id::ascending(ordered_id::Prefix::Message, None) {
                    Ok(id) => id,
                    Err(error) => return app.show_error_toast(error.to_string()),
                },
            };
            state.comment_id = Some(id.clone());
            let change = Change::Comment { id, reply_to: state.reply_to.clone(), body };
            return request(app, Operation::PostComment, Some(change));
        }
        Event::EditField(None) if state.request.is_none() => {
            state.editor = None;
            state.input.clear();
        }
        Event::EditField(field) if state.request.is_none() && !task.archived && !task.deleted => {
            let data = state.data.as_ref();
            state.input = match field {
                Some(Field::Assignee) => data
                    .and_then(|detail| detail.assignee.clone())
                    .unwrap_or_else(|| task.assignee.clone()),
                Some(Field::Priority) => {
                    data.and_then(|detail| detail.priority).unwrap_or(task.priority).to_string()
                }
                Some(Field::StartDate) => data
                    .and_then(|detail| detail.collaboration.start_date.clone())
                    .unwrap_or_default(),
                Some(Field::DueDate) => data
                    .and_then(|detail| detail.collaboration.due_date.clone())
                    .unwrap_or_default(),
                _ => String::new(),
            };
            state.editor = field;
        }
        Event::FieldInput(value) if state.request.is_none() && state.editor.is_some() => {
            state.input = value;
        }
        Event::SaveField | Event::ClearField if state.request.is_none() => {
            let clearing = matches!(event, Event::ClearField);
            let value = state.input.trim();
            let change = match state.editor {
                Some(Field::Assignee) => Change::SetAssignee {
                    assignee: if clearing { String::new() } else { value.to_owned() },
                },
                Some(Field::Priority) => {
                    let Ok(priority) = value.parse() else {
                        return Task::none();
                    };
                    Change::SetPriority { priority }
                }
                Some(Field::StartDate) => Change::SetStartDate {
                    date: (!clearing && !value.is_empty()).then(|| value.to_owned()),
                },
                Some(Field::DueDate) => Change::SetDueDate {
                    date: (!clearing && !value.is_empty()).then(|| value.to_owned()),
                },
                Some(Field::PullRequest) if !value.is_empty() => {
                    Change::LinkPullRequest { url: value.to_owned() }
                }
                _ => return Task::none(),
            };
            return request(app, Operation::SaveField, Some(change));
        }
        Event::UnlinkPullRequest(url)
            if state.request.is_none() && !task.archived && !task.deleted =>
        {
            return request(app, Operation::Unlink, Some(Change::UnlinkPullRequest { url }));
        }
        Event::ToggleActivity => state.activity_expanded = !state.activity_expanded,
        Event::ShowExecution => state.activity_expanded = true,
        Event::ToggleRun(id) => {
            state.expanded_run = (state.expanded_run.as_ref() != Some(&id)).then_some(id);
        }
        _ => {}
    }
    Task::none()
}

fn posting(state: &State) -> bool {
    state.request.is_some_and(|(_, operation)| operation == Operation::PostComment)
}

fn request(app: &mut App, operation: Operation, change: Option<Change>) -> Task<Message> {
    let state = &mut app.task_board.detail;
    let request = next_agent_scheduler_status_generation();
    state.request = Some((request, operation));
    let (path, task_id) = (state.project.clone(), state.task_id.clone());
    Task::perform(
        async move {
            let scope = resolve_task_board_scheduler_scope(&path).await?;
            let client = crate::app::config::gateway_client()?;
            let client = gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone());
            match change {
                Some(change) => {
                    client.agent_scheduler_change_task_detail(&scope, &task_id, &change).await
                }
                None => client.agent_scheduler_task_detail(&scope, &task_id).await,
            }
        },
        move |result| TaskBoardMessage::Detail(Event::Loaded { request, result }).into(),
    )
}
