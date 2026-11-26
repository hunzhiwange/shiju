use super::{
    attachment_link, next_agent_scheduler_status_generation, resolve_task_board_scheduler_scope,
    sync_task_board_gateway_snapshot,
};
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};
use desktop_task_board::detail::{
    ATTACHMENT_INPUT_ID, COMMENT_EDITOR_ID, Event, Field, Operation, State,
};
use iced::Task;
use iced::widget::text_editor;
use vw_api_agent_scheduler::{
    AgentSchedulerTaskDetailChange as Change, AgentSchedulerTaskReplyDelivery,
    AgentSchedulerTaskReplyMode,
};

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
            state.reply_poll_generation = id;
            let data = match result {
                Ok(data) => data,
                Err(_) if operation == Operation::PollReplies => return poll_replies(state),
                Err(error) => {
                    state.load_failed = operation == Operation::Load && state.data.is_none();
                    if operation == Operation::PostComment {
                        return Task::batch([
                            app.show_error_toast(error),
                            request(app, Operation::PollReplies, None),
                        ]);
                    }
                    let poll = poll_replies(state);
                    return Task::batch([poll, app.show_error_toast(error)]);
                }
            };
            state.load_failed = false;
            let posted = operation == Operation::PostComment
                || state.comment_id.as_ref().is_some_and(|id| {
                    data.collaboration.comments.iter().any(|comment| &comment.id == id)
                });
            if state.data.as_ref().and_then(|detail| detail.reply_agent.as_ref())
                != data.reply_agent.as_ref()
            {
                state.reply_mode = None;
                state.restart_turn_id = None;
                state.reply_menu_open = false;
                state.comment_id = None;
            }
            state.data = Some(data);
            if posted {
                state.comment = text_editor::Content::new();
                state.comment_id = None;
                state.reply_to = None;
                state.attachments.clear();
                state.comment_expanded = false;
                state.reply_mode = None;
                state.restart_turn_id = None;
                state.reply_menu_open = false;
            }
            if operation == Operation::SaveField {
                let board_field = matches!(state.editor, Some(Field::Assignee | Field::Priority));
                state.editor = None;
                state.input.clear();
                if board_field {
                    let path = state.project.clone();
                    app.task_board.task_board_workflow_generation =
                        next_agent_scheduler_status_generation();
                    return Task::batch([
                        poll_replies(state),
                        sync_task_board_gateway_snapshot(path),
                    ]);
                }
            }
            return poll_replies(state);
        }
        Event::Reload if !state.write_pending() => {
            let logs = super::sync_agent_scheduler_logs_for_project_path(
                state.project.clone(),
                state.task_id.clone(),
            );
            return Task::batch([request(app, Operation::Load, None), logs]);
        }
        Event::PollReplies(generation)
            if generation == state.reply_poll_generation
                && state.request.is_none()
                && state.has_active_replies() =>
        {
            return request(app, Operation::PollReplies, None);
        }
        Event::ExpandComment if state.data.is_some() && !task.archived && !task.deleted => {
            state.comment_expanded = true;
            return iced::widget::operation::focus(COMMENT_EDITOR_ID);
        }
        Event::CollapseComment
            if !posting(state)
                && state.comment.text().trim().is_empty()
                && state.attachments.is_empty() =>
        {
            state.comment_expanded = false;
            state.reply_to = None;
            state.reply_mode = None;
            state.restart_turn_id = None;
            state.reply_menu_open = false;
            state.comment_id = None;
        }
        Event::EditComment(action) if state.comment_expanded && !posting(state) => {
            if action.is_edit() {
                state.comment_id = None;
            }
            state.comment.perform(action);
        }
        Event::ReplyTo(id) if !posting(state) && !task.archived && !task.deleted => {
            if state.reply_to != id {
                state.comment_id = None;
                state.reply_to = id;
            }
            state.comment_expanded = true;
            return iced::widget::operation::focus(COMMENT_EDITOR_ID);
        }
        Event::ToggleReplyMenu
            if !posting(state)
                && state.data.as_ref().is_some_and(|detail| detail.reply_agent.is_some()) =>
        {
            state.reply_menu_open = !state.reply_menu_open;
        }
        Event::CloseReplyMenu => state.reply_menu_open = false,
        Event::SelectReplyMode(mode) if !posting(state) => {
            if mode.is_none_or(|mode| state.reply_mode_allowed(mode)) {
                state.reply_menu_open = false;
                let restart_turn_id = state
                    .data
                    .as_ref()
                    .filter(|_| mode == Some(AgentSchedulerTaskReplyMode::Restart))
                    .and_then(|detail| detail.reply_active_turn_id.clone());
                if state.reply_mode != mode || state.restart_turn_id != restart_turn_id {
                    state.comment_id = None;
                    state.reply_mode = mode;
                    state.restart_turn_id = restart_turn_id;
                }
            }
        }
        Event::AddAttachment if state.editor == Some(Field::Attachment) && !posting(state) => {
            if let Some(link) =
                attachment_link(&state.input).filter(|_| state.attachments.len() < 5)
            {
                if !state.attachments.contains(&link) {
                    state.attachments.push(link);
                    state.comment_id = None;
                }
                state.editor = None;
                state.input.clear();
                return iced::widget::operation::focus(COMMENT_EDITOR_ID);
            }
        }
        Event::RemoveAttachment(index) if !posting(state) && index < state.attachments.len() => {
            state.attachments.remove(index);
            state.comment_id = None;
        }
        Event::PostComment if state.can_post() && !task.archived && !task.deleted => {
            let body = state.comment.text();
            let id = match &state.comment_id {
                Some(id) => id.clone(),
                None => match ordered_id::ascending(ordered_id::Prefix::Message, None) {
                    Ok(id) => id,
                    Err(error) => return app.show_error_toast(error.to_string()),
                },
            };
            state.comment_id = Some(id.clone());
            let delivery = state
                .reply_mode
                .zip(state.data.as_ref().and_then(|detail| detail.reply_agent.clone()))
                .map(|(mode, agent)| AgentSchedulerTaskReplyDelivery {
                    agent,
                    mode,
                    active_turn_id: state.restart_turn_id.clone(),
                });
            state.reply_menu_open = false;
            let change = Change::Comment {
                id,
                reply_to: state.reply_to.clone(),
                body,
                attachments: state.attachments.clone(),
                delivery,
            };
            return request(app, Operation::PostComment, Some(change));
        }
        Event::StopReply(id) if !state.write_pending() => {
            return request(app, Operation::StopReply, Some(Change::StopReply { id }));
        }
        Event::EditField(None) if !state.write_pending() => {
            state.editor = None;
            state.input.clear();
        }
        Event::EditField(Some(Field::Attachment))
            if !state.write_pending()
                && state.data.is_some()
                && !task.archived
                && !task.deleted
                && state.attachments.len() < 5 =>
        {
            state.comment_expanded = true;
            state.reply_menu_open = false;
            state.editor = Some(Field::Attachment);
            state.input.clear();
            return iced::widget::operation::focus(ATTACHMENT_INPUT_ID);
        }
        Event::EditField(Some(Field::Attachment)) => {}
        Event::EditField(field) if !state.write_pending() && !task.archived && !task.deleted => {
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
        Event::FieldInput(value) if !state.write_pending() && state.editor.is_some() => {
            state.input = value;
        }
        Event::SaveField | Event::ClearField if !state.write_pending() => {
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
            if !state.write_pending() && !task.archived && !task.deleted =>
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

fn poll_replies(state: &State) -> Task<Message> {
    if !state.has_active_replies() {
        return Task::none();
    }
    crate::app::message::after(
        std::time::Duration::from_secs(1),
        TaskBoardMessage::Detail(Event::PollReplies(state.reply_poll_generation)).into(),
    )
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
