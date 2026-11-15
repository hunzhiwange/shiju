//! Project deletion freezes roots at confirmation and dispatches one request at a time.
use super::*;
use crate::app::state::{ArchivedChatsDeleteConfirmation, ArchivedProjectDelete};

pub(super) fn handles(state: &ArchivedChatsState, message: &ArchivedChatsMessage) -> bool {
    matches!(
        message,
        ArchivedChatsMessage::ProjectActionsToggled(_)
            | ArchivedChatsMessage::ProjectDeleteRequested(_)
            | ArchivedChatsMessage::ProjectDeleteRetry
            | ArchivedChatsMessage::ProjectDeleted { .. }
    ) || (matches!(message, ArchivedChatsMessage::DeleteConfirmed)
        && matches!(state.delete_confirmation, Some(ArchivedChatsDeleteConfirmation::Project(_))))
}

fn snapshot(state: &ArchivedChatsState, project: &str) -> Vec<Info> {
    let mut rows: Vec<_> = state
        .sessions
        .iter()
        .filter(|row| {
            row.project_id == project && row.time.archived.is_some() && row.is_user_navigable()
        })
        .cloned()
        .collect();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    rows.dedup_by(|a, b| a.id == b.id);
    let ids: std::collections::HashSet<_> = rows.iter().map(|row| row.id.clone()).collect();
    rows.retain(|row| {
        let mut parent = row.parent_id.as_ref();
        let mut seen = std::collections::HashSet::new();
        while let Some(id) = parent {
            if !seen.insert(id) {
                return false;
            }
            if ids.contains(id) {
                return false;
            }
            parent = state
                .sessions
                .iter()
                .find(|item| &item.id == id)
                .and_then(|item| item.parent_id.as_ref());
        }
        true
    });
    rows
}

pub(super) fn apply(
    state: &mut ArchivedChatsState,
    message: &ArchivedChatsMessage,
    current: &Identity,
) -> bool {
    if !handles(state, message) {
        return false;
    }
    match message {
        ArchivedChatsMessage::ProjectActionsToggled(id) => {
            if state.delete_confirmation.is_none()
                && state.operating_ids.is_empty()
                && !state.loading
            {
                state.project_action_menu =
                    (state.project_action_menu.as_ref() != Some(id)).then(|| id.clone());
            }
        }
        ArchivedChatsMessage::ProjectDeleteRequested(id) => {
            if state.identity.as_ref() != Some(current)
                || state.loading
                || !state.operating_ids.is_empty()
                || state.delete_confirmation.is_some()
            {
                return true;
            }
            let targets = snapshot(state, id);
            if targets.is_empty() {
                return true;
            }
            let name = state
                .project_options()
                .into_iter()
                .find(|p| &p.id == id)
                .map(|p| p.name)
                .unwrap_or_else(|| id.clone());
            state.project_delete = Some(ArchivedProjectDelete {
                identity: current.clone(),
                project_id: id.clone(),
                name,
                targets,
                next: 0,
                succeeded: 0,
                skipped: 0,
                failed: vec![],
            });
            state.delete_confirmation = Some(ArchivedChatsDeleteConfirmation::Project(id.clone()));
            state.project_action_menu = None;
        }
        ArchivedChatsMessage::ProjectDeleteRetry => {
            if state.loading
                || !state.operating_ids.is_empty()
                || state.delete_confirmation.is_some()
                || state.identity.as_ref() != Some(current)
            {
                return true;
            }
            if let Some(batch) = &mut state.project_delete {
                batch.targets = batch
                    .failed
                    .iter()
                    .filter(|target| {
                        state.sessions.iter().any(|row| {
                            row.id == target.id
                                && row.project_id == batch.project_id
                                && row.directory == target.directory
                                && row.time.archived.is_some()
                        })
                    })
                    .cloned()
                    .collect();
                batch.failed.clear();
                batch.next = 0;
                batch.succeeded = 0;
                batch.skipped = 0;
                if !batch.targets.is_empty() {
                    state.delete_confirmation =
                        Some(ArchivedChatsDeleteConfirmation::Project(batch.project_id.clone()));
                }
            }
        }
        ArchivedChatsMessage::DeleteConfirmed => {
            if state.project_delete.is_none()
                && let Some(ArchivedChatsDeleteConfirmation::Project(id)) =
                    &state.delete_confirmation
            {
                let request = ArchivedChatsMessage::ProjectDeleteRequested(id.clone());
                state.delete_confirmation = None;
                apply(state, &request, current);
            }
            if state.loading
                || !state.operating_ids.is_empty()
                || state.identity.as_ref() != Some(current)
            {
                return true;
            }
            if let Some(batch) = &state.project_delete {
                if &batch.identity != current || batch.targets.is_empty() {
                    return true;
                }
                state.request_generation += 1;
                state.delete_generation = Some(state.request_generation);
                state.operating_ids.extend(batch.targets.iter().map(|row| row.id.clone()));
            }
        }
        ArchivedChatsMessage::ProjectDeleted { identity, generation, id, result } => {
            if identity != current
                || state.identity.as_ref() != Some(current)
                || state.delete_generation != Some(*generation)
            {
                return true;
            }
            let Some(batch) = &mut state.project_delete else {
                return true;
            };
            if batch.targets.get(batch.next).is_none_or(|row| &row.id != id) {
                return true;
            }
            match result {
                Ok(Some(ids)) if ids.contains(id) => {
                    state.sessions.retain(|row| !ids.contains(&row.id));
                    batch.succeeded += 1;
                }
                Ok(None) => batch.skipped += 1,
                _ => batch.failed.push(batch.targets[batch.next].clone()),
            }
            batch.next += 1;
            if batch.next == batch.targets.len() {
                state.delete_generation = None;
                state.operating_ids.clear();
                state.delete_confirmation = None;
                state.project_action_menu = None;
                state.request_generation += 1;
                if state
                    .selected_project_id
                    .as_ref()
                    .is_some_and(|id| !state.project_options().iter().any(|p| &p.id == id))
                {
                    state.selected_project_id = None;
                }
            }
        }
        _ => {}
    }
    true
}

pub(super) fn update(
    app: &mut App,
    message: ArchivedChatsMessage,
    current: &Identity,
) -> Task<Message> {
    let mut committed = None;
    if let ArchivedChatsMessage::ProjectDeleted { identity, generation, id, result } = &message {
        let valid = identity == current
            && app.archived_chats.identity.as_ref() == Some(current)
            && app.archived_chats.delete_generation == Some(*generation)
            && app
                .archived_chats
                .project_delete
                .as_ref()
                .and_then(|b| b.targets.get(b.next))
                .is_some_and(|r| &r.id == id);
        if !valid {
            return Task::none();
        }
        if let Ok(Some(ids)) = result
            && ids.contains(id)
        {
            committed = Some(ids.clone());
        }
    }
    let was_running = app.archived_chats.delete_generation.is_some();
    apply(&mut app.archived_chats, &message, current);
    if let Some(ids) = committed {
        sync_deleted_sessions(app, &ids);
    }
    if matches!(message, ArchivedChatsMessage::ProjectDeleteRetry) {
        return Task::none();
    }
    if was_running && !matches!(message, ArchivedChatsMessage::ProjectDeleted { .. }) {
        return Task::none();
    }
    if let Some(generation) = app.archived_chats.delete_generation {
        let batch = app.archived_chats.project_delete.as_ref().unwrap();
        let row = batch.targets[batch.next].clone();
        let candidates = delete_candidates(app, &row.id);
        for id in candidates {
            if !app.archived_chats.delete_reconcile_ids.contains(&id) {
                app.archived_chats.delete_reconcile_ids.push(id);
            }
        }
        return task(crate::app::config::gateway_client(), current.clone(), generation, row);
    }
    if was_running {
        let batch = app.archived_chats.project_delete.as_ref().unwrap();
        let summary = format!(
            "归档聊天删除结果：成功 {}，失败 {}，状态变化跳过 {}。",
            batch.succeeded,
            batch.failed.len(),
            batch.skipped
        );
        let kind = if batch.failed.is_empty() && batch.skipped == 0 {
            crate::app::state::ToastKind::Success
        } else {
            crate::app::state::ToastKind::Error
        };
        let toast = app.show_toast(kind, summary);
        return Task::batch([toast, super::update(app, ArchivedChatsMessage::Refresh)]);
    }
    Task::none()
}

fn task(
    client: Result<vw_gateway_client::GatewayClient, String>,
    identity: Identity,
    generation: u64,
    row: Info,
) -> Task<Message> {
    let id = row.id.clone();
    Task::perform(async move { delete_checked(client?, row).await }, move |result| {
        Message::Settings(SettingsMessage::ArchivedChats(ArchivedChatsMessage::ProjectDeleted {
            identity: identity.clone(),
            generation,
            id: id.clone(),
            result,
        }))
    })
}

pub(super) async fn delete_checked(
    client: vw_gateway_client::GatewayClient,
    row: Info,
) -> Result<Option<Vec<String>>, String> {
    let latest: Info = client.session_get(&row.id, Some(&row.directory)).await?;
    if latest.id != row.id
        || latest.project_id != row.project_id
        || latest.directory != row.directory
        || latest.time.archived.is_none()
        || !latest.is_user_navigable()
    {
        return Ok(None);
    }
    client.session_delete_archived(&row.id, Some(&row.directory)).await.map(|r| Some(r.deleted_ids))
}
